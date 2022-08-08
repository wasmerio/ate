#!/bin/bash -e

echo "Compiling"
cargo wasix build --release --features client_web,bus,force_tty --no-default-features

echo "Release"
cp -f ../target/wasm32-wasmer-wasi/release/wasmer-deploy.wasm ../wasmer-web/public/bin/deploy.wasm
chmod +x ../wasmer-web/public/bin/deploy.wasm
