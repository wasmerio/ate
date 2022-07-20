#!/bin/bash -e

echo "Compiling"
cargo wasix build --release --features client_web,bus,force_tty --no-default-features

echo "Release"
cp -f ../target/wasm32-wasmer-wasi/release/deploy.wasm ../wasmer-term/public/bin/deploy.wasm
chmod +x ../wasmer-term/public/bin/deploy.wasm
