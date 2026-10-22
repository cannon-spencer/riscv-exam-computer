#!/bin/bash
# Board setup for the Orange Pi RV GPU stack.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
SEB_USER="${SEB_USER:-orangepi}"

# 8 GB swap is only for compiling WebKitGTK. A board that installs the
# prebuilt tarball does not need it at exam time.
if ! swapon --show | grep -q /swapfile; then
  sudo fallocate -l 8G /swapfile
  sudo chmod 600 /swapfile
  sudo mkswap /swapfile
  sudo swapon --priority 10 /swapfile
  echo '/swapfile none swap sw,pri=10 0 0' | sudo tee -a /etc/fstab
fi

sudo install -m 755 "$HERE/scripts/seb-launch.sh" /usr/local/bin/seb-launch.sh
sudo install -m 755 "$HERE/scripts/seb-remote.sh" /usr/local/bin/seb-remote.sh
sudo rm -f /etc/sudoers.d/seb-kiosk
echo "$SEB_USER ALL=(root) NOPASSWD: /usr/local/bin/seb-remote.sh" \
  | sudo tee /etc/sudoers.d/seb-remote >/dev/null
sudo chmod 440 /etc/sudoers.d/seb-remote
sudo visudo -c
