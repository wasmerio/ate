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

echo "Synchronizing the bin files"
rsync tokweb/public/bin/ /mnt/tok/bin --verbose --exclude '.conf' --checksum --recursive --whole-file --links --delete --human-readable
chmod o+r /mnt/tok
chmod o+x /mnt/tok
chmod -R o+r /mnt/tok/bin
chmod o+x /mnt/tok/bin

echo "Sync the files"
cd /mnt/tok
sync
