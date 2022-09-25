#!/bin/bash -e

#echo "Mounting Wasmer"
#mkdir -p /mnt/was
#umount -l /mnt/was 2>/dev/null || true
#killall wasmer-dfs 2>/dev/null || true
##wasmer-dfs mount --log-path ~/wasmer/fs /mnt/was wasmer.sh/www --non-empty &
#wasmer-dfs mount /mnt/was wasmer.sh/www --non-empty &
##cd wasmer-dfs
##cargo build
##cargo run -- mount /mnt/was wasmer.sh/www --non-empty &
##cd ..
#PID_MNT=$1
#
#function finish {
#  echo "Unmounting Wasmer"
#  umount -l /mnt/was 2>/dev/null || true
#  killall wasmer-dfs 2>/dev/null || true
#}
#trap finish EXIT
#
#echo
#echo "Waiting for mount..."
#for N in $(seq 0 30); do
#  [ -d /mnt/was/bin ] && break
#  echo -n "."
#  sleep 1
#done

echo "Compiling"
rm -r -f wasmer-web/dist/*
cd wasmer-web
npm install
npm run build
cd ..

echo "Building package"
mkdir -p target/release/www
rm -f -r target/release/www/* 2>/dev/null || true
cp -f -r wasmer-web/dist/* target/release/www
cp -r -f wasmer-web/public/* target/release/www

#echo "Synchronizing the files"
#rsync target/release/www/ /mnt/was --verbose --exclude '.conf' --checksum --recursive --whole-file --links --delete --human-readable
#chmod o+r /mnt/was
#chmod o+x /mnt/was
#chmod -R o+r /mnt/was/bin
#chmod o+x /mnt/was/bin
#
#echo "Sync the files"
#cd /mnt/was
#sync
