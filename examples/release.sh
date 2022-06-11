#!/bin/bash -e
cd tcp-listener
cargo wasi build --release
cd ..
cd tcp-client
cargo wasi build --release
cd ..
cd multi-threading
cargo wasi build --release
cd ..
cp -f ../target/wasm32-wasi/release/tcp-listener.wasi.wasm /prog/ate/tokweb/public/bin/example-tcp-listener.wasm
cp -f ../target/wasm32-wasi/release/tcp-client.wasi.wasm /prog/ate/tokweb/public/bin/example-tcp-client.wasm
cp -f ../target/wasm32-wasi/release/multi-threading.wasi.wasm /prog/ate/tokweb/public/bin/example-multi-threading.wasm
