#!/bin/bash -e
rm -f ../tokterm/public/bin/tok.wasm

cargo wasi build --features client_web,bus --no-default-features

wasm-opt --strip-debug --enable-reference-types -o ../target/wasm32-wasi/debug/tok.small.wasm ../target/wasm32-wasi/debug/tok.wasi.wasm

cp -f ../target/wasm32-wasi/debug/tok.small.wasm ../tokterm/public/bin/tok.wasm
chmod +x ../tokterm/public/bin/tok.wasm
