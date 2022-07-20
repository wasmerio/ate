#!/bin/bash -e
rm -f ../wasmer_term/public/bin/deploy.wasm

cargo wasix build --features client_web,bus,force_tty --no-default-features

cp -f ../target/wasm32-wasmer-wasi/debug/deploy.wasm ../wasmer_term/public/bin/deploy.wasm
chmod +x ../wasmer_term/public/bin/deploy.wasm
