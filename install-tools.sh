#!/bin/bash -e
pushd wasmer-deploy-cli >/dev/null
cargo build --release --bin wasmer-deploy
popd >/dev/null
pushd wasmer-term >/dev/null
cargo build --release
popd >/dev/null
pushd wasmer-dfs >/dev/null
cargo build --release
popd >/dev/null
sudo cp -f target/release/wasmer-deploy /usr/bin/
sudo cp -f target/release/wasmer-terminal /usr/bin/
sudo cp -f target/release/wasmer-dfs /usr/bin/
sudo ln -s /usr/bin/wasmer-deploy /usr/bin/wd
