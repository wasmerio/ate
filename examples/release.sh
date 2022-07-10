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
cd crypto-bus
cargo wasix build --release
cd ..
cd thread-local
cargo wasix build --release
cd ..
cd wasm64-example
cargo wasix build64 --release
cd ..
cd ws-client
cargo wasix build --release
cd ..
cd sub-process
cargo wasix build --release
cd ..
cd fuse
cargo wasix build --release
cd ..
cp -f ../target/wasm32-wasmer-wasi/release/tcp-listener.wasm /prog/ate/wasmer-web/public/bin/example-tcp-listener.wasm
cp -f ../target/wasm32-wasmer-wasi/release/tcp-client.wasm /prog/ate/wasmer-web/public/bin/example-tcp-client.wasm
cp -f ../target/wasm32-wasmer-wasi/release/multi-threading.wasm /prog/ate/wasmer-web/public/bin/example-multi-threading.wasm
cp -f ../target/wasm32-wasmer-wasi/release/crypto-bus.wasm /prog/ate/wasmer-web/public/bin/crypto-bus.wasm
cp -f ../target/wasm32-wasmer-wasi/release/thread-local.wasm /prog/ate/wasmer-web/public/bin/example-thread-local.wasm
cp -f ../target/wasm64-wasmer-wasi/release/wasm64-example.wasm /prog/ate/wasmer-web/public/bin/example-wasm64.wasm
cp -f ../target/wasm32-wasmer-wasi/release/ws-client.wasm /prog/ate/wasmer-web/public/bin/example-ws-client.wasm
cp -f ../target/wasm32-wasmer-wasi/release/sub-process.wasm /prog/ate/wasmer-web/public/bin/example-sub-process.wasm
cp -f ../target/wasm32-wasmer-wasi/release/fuse.wasm /prog/ate/wasmer-web/public/bin/example-fuse.wasm
