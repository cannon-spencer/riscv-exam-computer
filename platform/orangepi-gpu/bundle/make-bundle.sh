#!/bin/bash
set -e
B=~/bundle/seb-gpu-orangepi-rv
rm -rf ~/bundle && mkdir -p $B/root
cd ~/build/webkit2gtk-2.38.2/build-gles
DESTDIR=$B/root ninja install >/dev/null
mkdir -p $B/root/usr/lib/riscv64-linux-gnu $B/root/usr/bin
cp $(readlink -f /usr/lib/riscv64-linux-gnu/libwlroots.so.10) $B/root/usr/lib/riscv64-linux-gnu/libwlroots.so.10
cp /usr/bin/cage $B/root/usr/bin/cage
cat > $B/install.sh <<'EOI'
#!/bin/bash
set -e
cd "$(dirname "$0")"
TS=$(date +%Y%m%d-%H%M%S)
echo "Backing up files this will overwrite -> ~/seb-gpu-backup-$TS.tgz"
(cd root && find . -type f -o -type l) | sed 's#^\./#/#' | while read f; do [ -e "$f" ] && echo "$f"; done > /tmp/seb-gpu-overwrite.list
sudo tar czf ~/seb-gpu-backup-$TS.tgz -T /tmp/seb-gpu-overwrite.list 2>/dev/null || true
sudo cp -a root/. /
sudo ldconfig
sudo apt-mark hold libwebkit2gtk-4.1-0 libjavascriptcoregtk-4.1-0 libwlroots10 cage >/dev/null 2>&1 || true
echo "Installed. Rollback: sudo tar xzf ~/seb-gpu-backup-$TS.tgz -C / && sudo ldconfig"
EOI
chmod +x $B/install.sh
cat > $B/README.txt <<'EOR'
Prebuilt GPU stack for SEB on Orange Pi RV (JH7110, PowerVR BXE-4-32).
ONLY for: vendor Orange Pi RV Debian image, debian-ports snapshot 20221225T084846Z,
kernel 5.15.0-starfive2, IMG GPU userspace 1.17.6210866 installed.

Contents:
- WebKitGTK 2.38.2 (API 4.1), built from Debian source 2.38.2-1 with
  -DENABLE_GLES2=ON -DUSE_GTK4=OFF -DUSE_SOUP2=OFF -DENABLE_BUBBLEWRAP_SANDBOX=OFF
  -DENABLE_INTROSPECTION=OFF -DENABLE_DOCUMENTATION=OFF -DENABLE_WEBDRIVER=OFF, Release, -O2
- libwlroots.so.10 (0.15.1-4), patched: skip GL_EXT_unpack_subimage check on GLES 3+
- cage (0.1.4-4), patched: CAGE_MODE=WxH env var picks the display mode
Source patches + build scripts: see the repo folder.

Install: ./install.sh   (backs up overwritten files first)
Do NOT run apt upgrade on this snapshot.
EOR
cd ~/bundle
tar czf seb-gpu-orangepi-rv.tar.gz seb-gpu-orangepi-rv
sha256sum seb-gpu-orangepi-rv.tar.gz > seb-gpu-orangepi-rv.tar.gz.sha256
cp ~/make-bundle.sh ~/seb-gpu-repo/bundle/
ls -lh ~/bundle
