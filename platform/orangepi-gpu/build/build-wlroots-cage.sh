#!/bin/bash
# Patches + builds wlroots 0.15.1 and cage 0.1.4, installs both (with backups).
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"

apply_patch() {
  local patch_file="$1"
  if patch -p1 --dry-run --reverse < "$patch_file" >/dev/null 2>&1; then
    echo "already applied: $(basename "$patch_file")"
    return 0
  fi
  patch -p1 --forward < "$patch_file"
}

mkdir -p ~/build && cd ~/build
# wlroots
[ -d wlroots-0.15.1 ] || apt source wlroots=0.15.1-4
sudo apt build-dep -y wlroots
cd wlroots-0.15.1
apply_patch "$HERE/patches/wlroots-0.15.1-gles3-unpack-subimage.patch"
DEB_BUILD_OPTIONS="nocheck parallel=3" dpkg-buildpackage -b -uc -us > ~/wlr-build.log 2>&1
LIB=$(find "$PWD" -name 'libwlroots.so.10' -path '*obj-*' | head -1)
[ -n "$LIB" ] || { echo "wlroots build failed, see ~/wlr-build.log"; exit 1; }

# Headers and the distro library first. Replacing libwlroots.so.10 before this
# lets a later apt install put the unpatched library back.
cd ~/build
[ -d cage-0.1.4 ] || apt source cage=0.1.4-4
sudo apt install -y cage
sudo apt build-dep -y cage
sudo apt install -y libwlroots-dev

DEST=$(readlink -f /usr/lib/riscv64-linux-gnu/libwlroots.so.10)
[ -f ~/libwlroots.so.10.orig ] || cp "$DEST" ~/libwlroots.so.10.orig
sudo cp "$LIB" "$DEST"
sudo ldconfig
sudo apt-mark hold libwlroots10

cd ~/build/cage-0.1.4
apply_patch "$HERE/patches/cage-0.1.4-cage-mode.patch"
rm -rf build && meson setup build --buildtype=release && ninja -C build
[ -f /usr/bin/cage.orig ] || sudo cp /usr/bin/cage /usr/bin/cage.orig
sudo cp build/cage /usr/bin/cage
sudo apt-mark hold cage
echo "wlroots + cage installed. Rollbacks: ~/libwlroots.so.10.orig, /usr/bin/cage.orig"
