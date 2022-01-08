#!/bin/bash -e
echo "Compiling"
cargo wasi build --release --features client_web,bus,force_tty --no-default-features
#cp -f ../target/wasm32-wasi/release/tok.wasi.wasm ../target/wasm32-wasi/release/tok.small.wasm
wasm-opt --strip-debug --enable-reference-types -Oz -o ../target/wasm32-wasi/release/tok.small.wasm ../target/wasm32-wasi/release/tok.wasi.wasm
#cargo wasi build --features client_web --no-default-features
#wasm-opt --strip-debug --enable-reference-types -Oz -o ../target/wasm32-wasi/release/tok.small.wasm ../target/wasm32-wasi/debug/tok.wasi.wasm

echo "Release"
cp -f ../target/wasm32-wasi/release/tok.small.wasm ../tokterm/public/bin/tok.wasm
chmod +x ../tokterm/public/bin/tok.wasm
