#!/bin/bash
# Swap for the WebKit build + sudo rules for running SEB in cage.
set -e
if ! swapon --show | grep -q /swapfile; then
  sudo fallocate -l 8G /swapfile && sudo chmod 600 /swapfile
  sudo mkswap /swapfile && sudo swapon --priority 10 /swapfile
  echo '/swapfile none swap sw,pri=10 0 0' | sudo tee -a /etc/fstab
fi
echo 'orangepi ALL=(root) NOPASSWD:SETENV: /home/orangepi/seb-linux/build/bin/safe-exam-browser' \
  | sudo tee /etc/sudoers.d/seb-kiosk >/dev/null
echo 'orangepi ALL=(root) NOPASSWD: /home/orangepi/seb-remote.sh' \
  | sudo tee /etc/sudoers.d/seb-remote >/dev/null
sudo chmod 440 /etc/sudoers.d/seb-kiosk /etc/sudoers.d/seb-remote
sudo visudo -c
