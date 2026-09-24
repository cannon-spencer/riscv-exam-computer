ip link
nmcli device 2>/dev/null
rfkill list

sudo nmcli device wifi rescan
nmcli device wifi list
sudo nmcli device wifi connect "<SSID>" password "<password>"
ping -c2 github.com

ping -c2 github.com
sudo apt update
sudo apt install -y --no-install-recommends xorg xinit openbox xterm git build-essential qt6-base-dev libwebkit2gtk-4.1-dev libgtk-3-dev pkg-config
echo 'exec openbox' > ~/.xinitrc

cd ~
wget https://github.com/cannon-spencer/seb-linux/releases/download/riscv64/safe-exam-browser
chmod +x safe-exam-browser
file safe-exam-browser
ldd ./safe-exam-browser | grep "not found"

sudo apt install -y qt6-base-dev
ldd ~/safe-exam-browser | grep "not found"

// have Qt 6.3.1 but calls for 6.8
// proceed anyways

git clone https://github.com/cannon-spencer/seb-linux.git
cd seb-linux
./scripts/build.sh CONFIG+=force_webkitgtk 2>&1 | tee ~/build.log

// "2>&1 | tee ~/build.log" logs everything for debugging, can be removed from line
// let it build, takes a bit (about 15 min)

// need to patch it now, find usb drive with file and copy it over (sda1 here)

lsblk
sudo mount /dev/sda1 /mnt
cp /mnt/webkitgtk_view.cpp ~/seb-linux/src/browser/engines/webkitgtk/
sudo umount /mnt

// apply patches

grep -n 'appendPkexecEnvironmentVariable(pkexecArgs, "QT_QPA_PLATFORM");' src/main.cpp

// no print so build again (this ones fast), didn't find line to attach patches too

./scripts/build.sh CONFIG+=force_webkitgtk

// time to run and test, launch startx for graphics

startx
// right click open terminal
GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 ~/seb-linux/build/bin/safe-exam-browser --url https://elearning.ufl.edu

// errors

which pkexec

// no print so missing pkexec. install

sudo apt install -y pkexec

// try SEB again

GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 ~/seb-linux/build/bin/safe-exam-browser --url https://elearning.ufl.edu


// learn I don't know how to spell environment and misspelled grep line. it DOES need patches

sed -i '/appendPkexecEnvironmentVariable(pkexecArgs, "QT_QPA_PLATFORM");/a appendPkexecEnvironmentVariable(pkexecArgs, "GDK_BACKEND");' src/main.cpp

// trying without GPU disable line (..DISABLE_COMPOSITING..), rebuild and retest

./scripts/build.sh CONFIG+=force_webkitgtk
startx
//right click, terminal
GDK_BACKEND=x11 ~/seb-linux/build/bin/safe-exam-browser --url https://elearning.ufl.edu

// breaks, authorization problems

sudo apt install -y libpam-systemd dbus-user-session policykit-1
sudo reboot

// making launch script so not have to retype everytime

nano ~/run-seb.sh

	#!/bin/bash
	export GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1
	exec sudo -E ~/seb-linux/build/bin/safe-exam-browser --url https://elearning.ufl.edu/

chmod +x ~/run-seb.sh

startx
~/run-seb.sh

// breaks, try adding WEBKIT_DISABLE_COMPOSITING_MODE

nano ~/run-seb.sh

	export GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1

startx
~/run-seb.sh

// make openbox stop opening 4 desktops:

mkdir -p ~/.config/openbox
cp /etc/xdg/openbox/rc.xml ~/.config/openbox/
nano ~/.config/openbox/rc.xml

// n nano, press Ctrl+W, type <number>, and press Enter. Change 4 to 1, then save with Ctrl+O, Enter, Ctrl+X, and restart startx.

// breaks again
// seb instances are staying open, screen frozen and unresponsive. SSH works tho
// debugging more :|

// this opens help options without needing a display!

QT_QPA_PLATFORM=offscreen ~/seb-linux/build/bin/safe-exam-browser --help

// maybe startx not working with SEB

sudo apt install -y lightdm

// cant get SEB working on this, resetting to fresh ish install of OS. going to try different approach


