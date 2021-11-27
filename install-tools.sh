#!/bin/bash -e
pushd tokera >/dev/null
cargo build --release
popd >/dev/null
sudo cp -f target/release/tok /usr/bin/
