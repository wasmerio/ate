#!/bin/bash -e
cd tcp-listener
cargo wasix build --release
cd ..
cd tcp-client
cargo wasix build --release
cd ..
cd multi-threading
cargo wasix build --release
cd ..
#cd thread-local
#cargo wasix build --release
#cd ..
cd wasm64-example
cargo wasix build64 --release
cd ..
cp -f ../target/wasm32-wasmer-wasi/release/tcp-listener.wasix.wasm /prog/ate/tokweb/public/bin/example-tcp-listener.wasm
cp -f ../target/wasm32-wasmer-wasi/release/tcp-client.wasix.wasm /prog/ate/tokweb/public/bin/example-tcp-client.wasm
cp -f ../target/wasm32-wasmer-wasi/release/multi-threading.wasix.wasm /prog/ate/tokweb/public/bin/example-multi-threading.wasm
#cp -f ../target/wasm32-wasmer-wasi/release/thread-local.wasix.wasm /prog/ate/tokweb/public/bin/example-thread-local.wasm
cp -f ../target/wasm64-wasmer-wasi/release/wasm64-example.wasm /prog/ate/tokweb/public/bin/example-wasm64.wasm
