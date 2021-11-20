#!/bin/bash -e
echo "Compiling"
cargo wasi build --release
wasm-opt --strip-debug --enable-reference-types -Oz -o ../target/wasm32-wasi/release/tok-cli.small.wasm ../target/wasm32-wasi/release/tok-cli.wasi.wasm

echo "Release"
cp -f ../target/wasm32-wasi/release/tok-cli.small.wasm ../tokterm/public/bin/tok.wasm
chmod +x ../tokterm/public/bin/tok.wasm
