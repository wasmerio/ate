#!/bin/bash -e

echo "Mounting Tokera"
umount -l /mnt/tok 2>/dev/null || true
killall atefs 2>/dev/null || true
#atefs mount --log-path ~/ate/fs /mnt/tok tokera.sh/www --non-empty &
#atefs mount /mnt/tok tokera.sh/www --non-empty &
cd atefs
cargo build
cargo run -- mount /mnt/tok tokera.sh/www --non-empty &
cd ..
PID_MNT=$1

function finish {
  echo "Unmounting Tokera"
  umount -l /mnt/tok 2>/dev/null || true
  killall atefs 2>/dev/null || true
}
trap finish EXIT

echo
echo "Waiting for mount..."
for N in $(seq 0 30); do
  [ -d /mnt/tok/bin ] && break
  echo -n "."
  sleep 1
done

echo "Compiling"
rm -r -f tokweb/dist/*
cd tokweb
npm install
npm run build
cd ..

echo "Building package"
mkdir -p target/release/www
rm -f -r target/release/www/* 2>/dev/null || true
cp -f tokweb/dist/* target/release/www
cp -r -f tokweb/public/* target/release/www

echo "Synchronizing the files"
rsync target/release/www/ /mnt/tok --verbose --exclude '.conf' --checksum --recursive --whole-file --links --delete --human-readable
chmod o+r /mnt/tok
chmod o+x /mnt/tok
chmod -R o+r /mnt/tok/bin
chmod o+x /mnt/tok/bin

echo "Sync the files"
cd /mnt/tok
sync
