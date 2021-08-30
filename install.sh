#!/bin/bash -e
cargo build --release
sudo cp -f target/release/atedb /usr/bin/
sudo cp -f target/release/atefs /usr/bin/
sudo cp -f target/release/auth-server /usr/bin
sudo cp -f target/release/auth-tools /usr/bin
