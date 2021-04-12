#!/bin/bash -e

#sudo apt install cargo make pkg-config libfuse-dev libfuse3-dev openssl libssl-dev
cargo build --release --bin atedb
cargo build --release --bin atefs
cargo build --release --example auth-server
