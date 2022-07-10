#!/bin/bash -e
pushd wasmer >/dev/null
cargo build --release
popd >/dev/null
pushd wasmer-term >/dev/null
cargo build --release
popd >/dev/null
pushd wasmer-dfs >/dev/null
cargo build --release
popd >/dev/null
sudo cp -f target/release/deploy /usr/bin/
sudo cp -f target/release/wasmer-term /usr/bin/
sudo cp -f target/release/wasmer-dfs /usr/bin/
