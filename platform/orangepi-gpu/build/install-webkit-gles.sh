#!/bin/bash
# Backs up the stock WebKit, installs the GLES build, and holds the packages.
set -e
cd ~
dpkg-query -W -f='${Package}\n' | grep -E '^lib(webkit2gtk|javascriptcoregtk)-4\.1' > ~/webkit-pkgs.txt
for p in $(cat ~/webkit-pkgs.txt); do dpkg -L $p; done | sort -u \
  | while read f; do [ -f "$f" -o -L "$f" ] && echo "$f"; done > ~/webkit-rollback.list
sudo tar czf ~/webkit-rollback.tgz -T ~/webkit-rollback.list
sudo apt-mark hold $(cat ~/webkit-pkgs.txt)
cd ~/build/webkit2gtk-2.38.2/build-gles
sudo ninja install
sudo ldconfig
echo "Installed. Rollback: sudo tar xzf ~/webkit-rollback.tgz -C / && sudo ldconfig"
