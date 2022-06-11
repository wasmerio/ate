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
cd thread-local
cargo wasi build --release
cd ..
cd wasm64-example
cargo wasix build --release
cd ..
cp -f ../target/wasm32-wasi/release/tcp-listener.wasi.wasm /prog/ate/tokweb/public/bin/example-tcp-listener.wasm
cp -f ../target/wasm32-wasi/release/tcp-client.wasm /prog/ate/tokweb/public/bin/example-tcp-client.wasm
cp -f ../target/wasm32-wasi/release/multi-threading.wasi.wasm /prog/ate/tokweb/public/bin/example-multi-threading.wasm
cp -f ../target/wasm32-wasi/release/thread-local.wasm /prog/ate/tokweb/public/bin/example-thread-local.wasm
cp -f ../target/wasm64-wasi/release/wasm64-example.wasm /prog/ate/tokweb/public/bin/example-wasm64.wasm
