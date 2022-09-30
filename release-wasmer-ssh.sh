#!/bin/bash -e

rm -f target/release/wasmer-ssh
rm -f target/release/wasmer-ssh-debug

cd wasmer-ssh
cargo build --bin wasmer-ssh --release
cd ..
mv -f target/release/wasmer-ssh target/release/wasmer-ssh-debug
objcopy --strip-all target/release/wasmer-ssh-debug target/release/wasmer-ssh
cp -f target/release/wasmer-ssh /usr/bin

systemctl stop wasmer-ssh || true
systemctl start wasmer-ssh
