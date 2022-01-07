#!/bin/bash -e

cd atessh
cargo build --bin atessh --release
cd ..
mv target/release/atessh target/release/atessh-debug
objcopy --strip-all target/release/atessh-debug target/release/atessh
cp -f target/release/atessh /usr/bin

systemctl stop atessh
killall atessh
systemctl start atessh

