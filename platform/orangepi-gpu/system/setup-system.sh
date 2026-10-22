#!/bin/bash
# Board setup for the Orange Pi RV GPU stack.
# Pass --swap only when compiling WebKitGTK on this board.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
SEB_USER="${SEB_USER:-orangepi}"

if [ "${1:-}" = "--swap" ]; then
  if ! swapon --show | grep -q /swapfile; then
    sudo fallocate -l 8G /swapfile
    sudo chmod 600 /swapfile
    sudo mkswap /swapfile
    sudo swapon --priority 10 /swapfile
    sudo grep -q '^/swapfile[[:space:]]' /etc/fstab \
      || echo '/swapfile none swap sw,pri=10 0 0' | sudo tee -a /etc/fstab
  fi
fi

sudo install -m 755 "$HERE/scripts/seb-launch.sh" /usr/local/bin/seb-launch.sh
sudo install -m 755 "$HERE/scripts/seb-remote.sh" /usr/local/bin/seb-remote.sh
sudo rm -f /etc/sudoers.d/seb-kiosk
rule=$(mktemp)
trap 'rm -f "$rule"' EXIT
echo "$SEB_USER ALL=(root) NOPASSWD: /usr/local/bin/seb-remote.sh" > "$rule"
sudo visudo -cf "$rule"
sudo install -m 440 "$rule" /etc/sudoers.d/seb-remote
sudo visudo -c
