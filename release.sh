#!/bin/bash -e

echo "Mounting Tokera"
umount -l /mnt/tok 2>/dev/null || true
killall atefs 2>/dev/null || true
atefs mount /mnt/tok tokera.sh/www --non-empty &
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
rm -r -f tokterm/dist/*
cd tokterm
npm install
npm run build
cd ..

echo "Building package"
mkdir -p target/release/www
rm -f -r target/release/www/* 2>/dev/null || true
cp -f tokterm/dist/* target/release/www
cp -r -f tokterm/public/* target/release/www

echo "Synchronizing the files"
rsync target/release/www/ /mnt/tok --verbose --checksum --recursive --whole-file --links --delete --human-readable

echo "Sync the files"
cd /mnt/tok
sync
