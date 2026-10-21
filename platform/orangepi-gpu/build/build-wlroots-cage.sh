#!/bin/bash
# Patches + builds wlroots 0.15.1 and cage 0.1.4, installs both (with backups).
set -e
HERE="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p ~/build && cd ~/build
# wlroots
[ -d wlroots-0.15.1 ] || apt source wlroots=0.15.1-4
sudo apt build-dep -y wlroots
cd wlroots-0.15.1
patch -p1 -N < "$HERE/patches/wlroots-0.15.1-gles3-unpack-subimage.patch" || true
DEB_BUILD_OPTIONS="nocheck parallel=3" dpkg-buildpackage -b -uc -us > ~/wlr-build.log 2>&1 || true
LIB=$(find . -name 'libwlroots.so.10' -path '*obj-*' | head -1)
[ -n "$LIB" ] || { echo "wlroots build failed, see ~/wlr-build.log"; exit 1; }
DEST=$(readlink-f /usr/lib/riscv64-linux-gnu/libwlroots.so.10)
[ -f ~/libwlroots.so.10.orig ] || cp "$DEST" ~/libwlroots.so.10.orig
sudo cp "$LIB" "$DEST" && sudo ldconfig
sudo apt-mark hold libwlroots10
# cage
cd ~/build
[ -d cage-0.1.4 ] || apt source cage=0.1.4-4
sudo apt install -y cage
sudo apt build-dep -y cage
sudo apt install -y libwlroots-dev
cd cage-0.1.4
patch -p1 -N < "$HERE/patches/cage-0.1.4-cage-mode.patch" || true
rm -rf build && meson setup build --buildtype=release && ninja -C build
[ -f /usr/bin/cage.orig ] || sudo cp /usr/bin/cage /usr/bin/cage.orig
sudo cp build/cage /usr/bin/cage
sudo apt-mark hold cage
echo "wlroots + cage installed. Rollbacks: ~/libwlroots.so.10.orig, /usr/bin/cage.orig"
