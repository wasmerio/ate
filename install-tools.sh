#!/bin/bash -e
pushd tokera >/dev/null
cargo build --release
popd >/dev/null
pushd tokterm >/dev/null
cargo build --release
popd >/dev/null
pushd atefs >/dev/null
cargo build --release
popd >/dev/null
sudo cp -f target/release/tok /usr/bin/
sudo cp -f target/release/tokterm /usr/bin/
sudo cp -f target/release/atefs /usr/bin/
