<!-- markdown-link-check-disable -->
# Skynet Offchain Worker Example Pallet

The Skynet Offchain Worker Example: A simple pallet demonstrating
simple interaction with the Skynet Substrate SDK in the context of an offchain workers.
Based on the Offchain Worker Example Pallet

Run `cargo doc --package pallet-example-offchain-worker --open` to view this module's
documentation.

- [`Config`]
- [`Pallet`]

**This pallet serves as an example showcasing Substrate off-chain worker using Skynet andis not meant to
be used in production.**

## Overview

In this example we are going to build a very simplistic, naive and definitely NOT
production-ready off-chain data publisher and consumer using Skynet.

Offchain Worker (OCW) will be triggered after every block, fetch the "historical block heght"
saved by this node and then update this value using the current block height before persisting to Skynet.
By using a resolver skylink, access to the latest value is consistent across any Skynet portal.
You could also change the "data key" to, for example, save historical blockheight per a day, at predictable
resolver skylinks for your web application.
In the example, we have hard-coded keys, but you will likely want to incorporate ed25519 keys from
your node.

License: Unlicense

## Running the Node

Make sure you are using Rust nightly with the wasm toolchain installed and run:

```
cd bin/node-template/node
cargo run --release -- --dev --offchain-worker --tmp
```
