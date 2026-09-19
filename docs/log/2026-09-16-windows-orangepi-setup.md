# Orange Pi RV setup (Windows)
## 9/16/2026

Windows-device writeup. Overlaps the Mac `flash-os.sh` / UART / `install-software.sh` logs; kept as the Windows path.

## Requirements
- MicroSD card

---

## 1. OS

### Download a working OS image
~~Get `os.img.xz` from the reptilian-riscv repo releases page (tag: orangepi-rv)~~

Get Debian OS image from manufacturer website http://www.orangepi.org/html/hardWare/computerAndMicrocontrollers/service-and-support/Orange-Pi-RV.html

### Extract the image (if needed)
~~Install 7-Zip if you don't have it, then right-click `os.img.xz` -> 7-Zip -> Extract Here. You should get `os.img`.~~

### Download Balena Etcher
https://etcher.balena.io/

### Run Balena Etcher as admin
Right-click the installer/app -> Run as administrator.

### Flash the .img to your microSD card
- Flash from file -> select os.img
- Select target -> your microSD card (double-check size before confirming)
- Flash

### Connect the board via UART
USB-to-TTL adapter -> board's UART pins (TX<->RX, RX<->TX, GND<->GND). Open PuTTY:
- Connection type: Serial
- Serial line: your COM port (check Device Manager)
- Speed: 115200

### Insert the microSD card into the board

### Power on the board
Should boot straight into Debian.

---

## 2. Wi-Fi
If on UF wifi, suggested to setup as a UF device https://my.device.ufl.edu/

In your PuTTY session:

1. Run:
   sudo nmtui
2. Select "Activate a connection" (arrow keys + Enter)
3. Pick your network, enter the password if prompted
4. Arrow to "Back", then "OK", returning to the terminal

Then run:
   ip addr show wlan0

Copy down the inet line IP (e.g. 100.100.1.123) - you'll need this for the software install step.

---

## 3. Software

### Set up WSL
Install WSL (Ubuntu works) if you don't already have it, then open the WSL terminal.

### Install dependencies
   sudo apt install sshpass
   sudo snap install zig --classic --beta
   cargo install cargo-zigbuild

If cargo/rustup aren't installed yet, install Rust first:
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

### Clone the repo
   git clone --recurse-submodules https://github.com/cannon-spencer/riscv-exam-computer.git
   cd riscv-exam-computer

### Run the install script
   ./scripts/install-software.sh --host orangepi@100.100.1.xxx

(replace with the board's actual IP from the Wi-Fi step)

### Verify install
On the board:
   ls -la ~/seb-agent

You should see something like:
orangepi@orangepirv:~$ ls -la ~/seb-agent
-rwxr-xr-x 1 orangepi orangepi 1969280 Sep 16 19:19 /home/orangepi/seb-agent
