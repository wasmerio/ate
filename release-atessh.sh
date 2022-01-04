#!/bin/bash -e

cd atessh
cargo build --bin atessh --release
cd ..
cp -f target/release/atessh /usr/bin
