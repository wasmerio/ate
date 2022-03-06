#!/bin/bash -e
cargo build --release --example find --target wasm32-wasi
cp -f ../target/wasm32-wasi/release/examples/find.wasm ../tokweb/public/bin
