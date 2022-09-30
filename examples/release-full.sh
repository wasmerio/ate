#!/bin/bash -e
cd ../../rust
./build-wasix.sh
cd ../ate/examples

cargo clean

./release.sh
