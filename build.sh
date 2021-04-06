#!/bin/bash -e

apt install cargo make pkg-config libfuse-dev libfuse3-dev openssl libssl-dev
cargo build --release --bin atedb
cargo build --release --bin atefs

