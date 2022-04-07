#!/bin/bash -e

rm -f target/release/atessh
rm -f target/release/atessh-debug

cd atessh
cargo build --bin atessh --release
cd ..
mv -f target/release/atessh target/release/atessh-debug
objcopy --strip-all target/release/atessh-debug target/release/atessh
cp -f target/release/atessh /usr/bin

systemctl stop atessh || true
systemctl start atessh
