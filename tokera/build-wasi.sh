#!/bin/bash -e
rm -f ../tokterm/public/bin/tok.wasm

cargo wasix build --features client_web,bus,force_tty --no-default-features

cp -f ../target/wasm32-wasmer-wasi/debug/tok.wasm ../tokterm/public/bin/tok.wasm
chmod +x ../tokterm/public/bin/tok.wasm
