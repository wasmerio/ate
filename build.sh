#!/bin/bash -e

#sudo apt install cargo make pkg-config libfuse-dev libfuse3-dev openssl libssl-dev
pushd tokera >/dev/null
cargo build --release --bin tok
popd >/dev/null
#cargo build --release --bin atedb
#cargo build --release --bin atefs
#cargo build --release --bin auth-server
#cargo build --release --bin auth-tools
