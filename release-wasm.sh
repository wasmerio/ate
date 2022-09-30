#!/bin/bash -e

echo "Mounting Wasmer"
mkdir -p /mnt/was
umount -l /mnt/was 2>/dev/null || true
killall wasmer-dfs 2>/dev/null || true
wasmer-dfs mount /mnt/was wasmer.sh/www --non-empty &
PID_MNT=$1

function finish {
  echo "Unmounting Wasmer"
  umount -l /mnt/was 2>/dev/null || true
  killall wasmer-dfs 2>/dev/null || true
}
trap finish EXIT

echo
echo "Waiting for mount..."
for N in $(seq 0 30); do
  [ -d /mnt/was/bin ] && break
  echo -n "."
  sleep 1
done

echo "Synchronizing the bin files"
rsync wasmer-web/public/bin/ /mnt/was/bin --verbose --exclude '.conf' --checksum --recursive --whole-file --links --delete --human-readable
chmod o+r /mnt/was
chmod o+x /mnt/was
chmod -R o+r /mnt/was/bin
chmod o+x /mnt/was/bin

echo "Sync the files"
cd /mnt/was
sync
