#!/bin/bash -e
cd tokterm

echo "Mounting Tokera"
umount -l /mnt/tok || true
atefs mount /mnt/tok sh.tokera.com/www --non-empty &
PID_MNT=$1

echo
echo "Waiting for mount..."
for N in $(seq 0 30); do
  [ -d /mnt/tok/bin ] && break
  echo -n "."
  sleep 1
done

echo "Compiling"
npm install
npm run build

echo "Removing existing files"
rm -f /mnt/tok/bin/* || true
rm -f /mnt/tok/* || true

echo "Copying new files"
cp -f dist/* /mnt/tok
cp -r -f public/* /mnt/tok

echo "Unmounting Tokera"
umount -l /mnt/tok || true
kill $PID_MNT
