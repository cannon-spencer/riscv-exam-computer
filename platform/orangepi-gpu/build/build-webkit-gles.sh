#!/bin/bash
# Builds WebKitGTK 2.38.2 (API 4.1) with OpenGL ES. ~11.5 h on JH7110.
# Needs ~30 GB free disk and >= 8 GB swap (see system/setup-system.sh). Run inside tmux.
set -eo pipefail
echo 'deb-src [check-valid-until=no] https://snapshot.debian.org/archive/debian/20221225T084846Z unstable main' \
  | sudo tee /etc/apt/sources.list.d/webkit-src.list
sudo apt update
sudo apt install -y dpkg-dev cmake ninja-build libgles-dev libegl-dev
mkdir -p ~/build && cd ~/build
[ -d webkit2gtk-2.38.2 ] || apt source webkit2gtk=2.38.2-1
cd webkit2gtk-2.38.2
sudo apt build-dep -y .
mkdir -p build-gles && cd build-gles
cmake .. -G Ninja \
  -DPORT=GTK -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_C_FLAGS_RELEASE="-O2 -DNDEBUG" -DCMAKE_CXX_FLAGS_RELEASE="-O2 -DNDEBUG" \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DCMAKE_INSTALL_LIBDIR=lib/riscv64-linux-gnu -DCMAKE_INSTALL_LIBEXECDIR=lib/riscv64-linux-gnu \
  -DCMAKE_INSTALL_SYSCONFDIR=/etc \
  -DUSE_GTK4=OFF -DUSE_SOUP2=OFF -DUSE_OPENGL_OR_ES=ON -DENABLE_GLES2=ON \
  -DENABLE_DOCUMENTATION=OFF -DENABLE_INTROSPECTION=OFF \
  -DENABLE_MINIBROWSER=ON -DENABLE_WEBDRIVER=OFF -DENABLE_BUBBLEWRAP_SANDBOX=OFF \
  2>&1 | tee cmake.log
grep -E 'ENABLE_GLES2|USE_OPENGL_OR_ES' cmake.log
ninja -j3 2>&1 | tee build.log
echo "Build done. Next: build/install-webkit-gles.sh"
