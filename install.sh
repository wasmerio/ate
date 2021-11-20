#!/bin/bash -e
pushd tokera >/dev/null
cargo build --release
popd >/dev/null
sudo cp -f target/release/tok /usr/bin/
#sudo cp -f target/release/atedb /usr/bin/
#sudo cp -f target/release/atefs /usr/bin/
#sudo cp -f target/release/auth-server /usr/bin
#sudo cp -f target/release/auth-tools /usr/bin
