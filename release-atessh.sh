#!/bin/bash -e

cd atessh
cargo build --bin atessh --release
cd ..
cp -f target/release/atessh /usr/bin

systemctl stop atessh
killall atessh
systemctl start atessh

