// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! <!-- markdown-link-check-disable -->
//! # Skynet Offchain Worker Example Pallet
//!
//! The Skynet Offchain Worker Example: A simple pallet demonstrating
//! simple interaction with the Skynet Substrate SDK in the context of an offchain workers.
//! Based on the Offchain Worker Example Pallet
//!
//! Run `cargo doc --package pallet-example-offchain-worker --open` to view this module's
//! documentation.
//!
//! - [`Config`]
//! - [`Pallet`]
//!
//! **This pallet serves as an example showcasing Substrate off-chain worker using Skynet and is not meant to
//! be used in production.**
//!
//! ## Overview
//!
//! In this example we are going to build a very simplistic, naive and definitely NOT
//! production-ready off-chain data publisher and consumer using Skynet.
//!
//! Offchain Worker (OCW) will be triggered after every block, fetch the "historical block height"
//! saved by this node and then update this value using the current block height before persisting to Skynet.
//! By using a resolver skylink, access to the latest value is consistent across any Skynet portal.
//! You could also change the "data key" to, for example, save historical blockheight per a day, at predictable
//! resolver skylinks for your web application.
//! In the example, we have hard-coded keys, but you will likely want to incorporate ed25519 keys from
//! your node.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_core::crypto::KeyTypeId;
use sp_std::str;
use sp_runtime::traits::UniqueSaturatedInto;

#[cfg(test)]
mod tests;

pub use pallet::*;

// Here we define the keys used to sign the registry entry on Skynet.
// This will likely need to be unique-per-node, and clients can reference aggregate data from various nodes.
const PUBLIC_KEY: &str = "658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
const PRIVATE_KEY: &str = "7caffac49ac914a541b28723f11776d36ce81e7b9b0c96ccacd1302db429c79c658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
const DATA_KEY: &str = "block-heights-test";

/// Our error type.
#[derive(Debug)]
enum Error {
	GetHistoricalHeightError(skynet_substrate::DownloadError),
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: T::BlockNumber) {
			// Since off-chain workers are just part of the runtime code, they have direct access
			// to the storage and other included pallets.
			//
			// We can easily import `frame_system` and retrieve a block hash of the parent block.
			let parent_hash = <frame_system::Pallet<T>>::block_hash(block_number - 1u32.into());
			log::info!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);

			// Get resolver skylink for reading historical height below.
			let skylink_bytes =
				skynet_substrate::get_entry_link(PUBLIC_KEY, DATA_KEY, None).unwrap();
			let skylink = str::from_utf8(&skylink_bytes).unwrap();
			log::info!("Entry link: {:?}", skylink);

			// This method downloads the skylink, and returns the historical height if it exists,
			// else None.
			let historical_height = Self::get_historical_height(skylink);

			// Get the average height based on the downloaded historical height.
			// If the return status is 404, assume 0 historical height (None). On other error,
			// return early from the offchain worker.
			let average_height = match historical_height {
				Ok(historical_height) => {
					log::info!("Got historical height from skylink! Data: {:?}", historical_height);

					Self::average_heights(historical_height)
				},
				Err(e) => {
					log::info!("Error: {:?}", e);
					return
				},
			};

			// Encode the average height into bytes.
			let average_height_bytes = encode_number(average_height);

			// Upload the average height and update the v2 resolver skylink.
			let result = skynet_substrate::upload_bytes(
				&average_height_bytes,
				DATA_KEY,
				Some(&skynet_substrate::UploadOptions { timeout: 30_000, ..Default::default() }),
			);
			if let Ok(skylink_bytes) = result {
				let skylink = str::from_utf8(&skylink_bytes).unwrap();

				log::info!("Uploaded average height! Skylink: {:?}", skylink);

				// Set the skylink in the v2 resolver skylink with the result from upload.
				let _ = skynet_substrate::set_data_link(PRIVATE_KEY, DATA_KEY, skylink, None);
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	/// This method will use a function for taking a historical_height and "averaging" with the
	/// current block height We'll average over roughly 1 day of blocks.
	fn average_heights(historical_height: Option<u64>) -> u64 {
		let block_height: u32 = <frame_system::Pallet<T>>::block_number().unique_saturated_into();
		let block_height = block_height as u64;
		log::info!("Block height: {:?}", block_height);
		let samples = 10u64;

		if let Some(historical_height) = historical_height {
			// method to average
			let mut average_height = historical_height - historical_height / samples;
			average_height += block_height / samples;

			average_height
		} else {
			// No historical height was found, so use the block height.
			block_height
		}
	}

	/// Gets the historical average height from Skynet. Returns `Ok(None)` if we do 
	/// not have historical data.
	fn get_historical_height(skylink: &str) -> Result<Option<u64>, Error> {
		// Check to see if we have already saved a block height average
		let historical_height = skynet_substrate::download_bytes(skylink, None);
		log::info!("Historical height: {:?}", historical_height);

		match historical_height {
			// If we get a 404, return None to indicate that there is no historical height.
			Err(skynet_substrate::DownloadError::RequestError(
				skynet_substrate::RequestError::UnexpectedStatus(404),
			)) => Ok(None),
			Err(e) => Err(Error::GetHistoricalHeightError(e)),
			Ok(data) => {
				// If data isn't parsable, return None and we'll reset.
				if data.len() != 8 {
					return Ok(None)
				}

				// Decode the returned data to a u64 from Vec<u8>.
				let height = decode_number(data[0..8].try_into().unwrap());

				Ok(Some(height))
			},
		}
	}
}

/// Decodes the given byte array into a u64 number.
fn decode_number(encoded_num: [u8; 8]) -> u64 {
	let mut decoded: u64 = 0;
	for encoded_byte in encoded_num.into_iter().rev() {
		decoded <<= 8;
		decoded |= encoded_byte as u64
	}
	decoded
}

/// Converts the given number into a byte array. Uses little-endian encoding.
fn encode_number(mut num: u64) -> [u8; 8] {
	let mut encoded: [u8; 8] = [0; 8];
	for encoded_byte in &mut encoded {
		let byte = num & 0xff;
		*encoded_byte = byte as u8;
		num >>= 8;
	}
	encoded
}
