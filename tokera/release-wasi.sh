#!/bin/bash -e

echo "Compiling"
cargo wasix build --release --features client_web,bus,force_tty --no-default-features

echo "Release"
cp -f ../target/wasm32-wasmer-wasi/release/tok.wasm ../tokterm/public/bin/tok.wasm
chmod +x ../tokterm/public/bin/tok.wasm
