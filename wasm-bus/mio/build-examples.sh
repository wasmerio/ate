#!/bin/bash -e

cargo wasix build --example ping --release --no-default-features
wasm-opt --strip-debug --enable-reference-types -Oz -o ../../target/wasm32-wasi/release/examples/ping.small.wasm ../../target/wasm32-wasi/release/examples/ping.wasi.wasm
cp -f ../../target/wasm32-wasi/release/examples/ping.small.wasm ../../tokweb/public/bin/ping.wasm
chmod +x ../../tokweb/public/bin/ping.wasm
