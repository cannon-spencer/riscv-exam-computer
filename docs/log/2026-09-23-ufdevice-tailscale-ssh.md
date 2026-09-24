# Campus SSH via ufdevice + Tailscale

Eduroam will not let a laptop SSH to the board. Register the board as a UF device, join `ufdevice`, then Tailscale. UART is still how you do this the first time.

Laptop also needs Tailscale. Then `install-software.sh --host orangepi@TAILSCALE_IP`.

### 1. Register the MAC

On the board:

```bash
ip link show wlan0
```

```
3: wlan0: <NO-CARRIER,BROADCAST,MULTICAST,UP,LOWER_UP> ...
    link/ether 9c:b8:b4:8e:fa:0e brd ff:ff:ff:ff:ff:ff
```

Paste that `link/ether` into [https://my.device.ufl.edu/](https://my.device.ufl.edu/)

### 2. Join ufdevice

```bash
sudo nmcli dev wifi connect "ufdevice" password "gogators"
```

```
Device 'wlan0' successfully activated with '0bd8a3d2-7b67-47c6-962c-8000e89b3a17'.
```

### 3. Tailscale (userspace)

RISC-V does not get a normal TUN device. Install, then force userspace networking before `up`.

```bash
curl -fsSL https://tailscale.com/install.sh | sh
```

```bash
sudo nano /etc/default/tailscaled
```

```
FLAGS="--tun=userspace-networking"
```

```bash
sudo systemctl daemon-reload
sudo systemctl start tailscaled
sudo tailscale up
```

Open the auth URL it prints. After the node is in the tailnet:

```bash
ssh orangepi@TAILSCALE_IP
```

Password is still `orangepi`.

### Caveats

`ufdevice` is a one-time campus registration per MAC. A new board (or a new Wi-Fi chip) needs step 1 again.

`tailscale up` without the `FLAGS` line fails on this image. Userspace is slower than kernel TUN; fine for SSH and `install-software.sh`.