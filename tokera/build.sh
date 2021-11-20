#!/bin/bash -e
rm -f ../tokterm/public/bin/tok.wasm

cargo wasi build

wasm-opt --strip-debug --enable-reference-types -o ../target/wasm32-wasi/debug/tok-cli.small.wasm ../target/wasm32-wasi/debug/tok-cli.wasi.wasm

cp -f ../target/wasm32-wasi/debug/tok-cli.small.wasm ../tokterm/public/bin/tok.wasm
chmod +x ../tokterm/public/bin/tok.wasm
