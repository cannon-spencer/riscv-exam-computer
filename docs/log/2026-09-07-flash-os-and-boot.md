# flash-os.sh on a microSD, then boot the Orange Pi RV
## 9/7/2026

### Flash the SD card

`scripts/flash-os.sh` worked. Check the SD disk with `diskutil list`, then:

```bash
./scripts/flash-os.sh --device /dev/rdiskN
```

### Serial console

Need USB UART **and** power.

```bash
ls /dev/cu.*
screen /dev/cu.usbserial-3130 115200
```

Stuck `screen` on that port — `lsof` shows which PID is holding it:

```
$ sudo lsof /dev/cu.usbserial-3130
COMMAND   PID          USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
screen  91356 cannonspencer    5u   CHR    9,5   0t1651 1129 /dev/cu.usbserial-3130

$ sudo kill -9 91356
```

Default login: `orangepi` / `orangepi`.

### Wifi and outbound ping

Wifi via `sudo nmtui`. Then ping to confirm outbound:

```
$ ping -c 4 google.com
4 packets transmitted, 4 received, 0% packet loss
rtt min/avg/max/mdev = 26.045/46.788/106.415/34.431 ms
```

Board address (wlan0):

```
$ ip -4 addr
3: wlan0: <BROADCAST,MULTICAST,UP,LOWER_UP> ...
    inet 10.0.0.40/24 brd 10.0.0.255 scope global dynamic noprefixroute wlan0
```

### SSH

Laptop on the same network. Laptop was `10.0.0.45`:

```
$ ipconfig getifaddr en0
10.0.0.45

$ ssh orangepi@10.0.0.40
Welcome to Orange Pi 1.0.0 Sid with Linux 5.15.0-starfive2
IP: 10.0.0.40
```

### curl POST to the laptop

Dummy server on the Mac (does not handle POST — 501 is expected):

```
$ python3 -m http.server 8000
Serving HTTP on :: port 8000 (http://[::]:8000/) ...
```

On the board, first check that a POST can reach the Mac at all:

```
orangepi@orangepirv:~$ curl -sS -X POST http://10.0.0.45:8000/heartbeat \
  -H 'Content-Type: application/json' \
  -d '{"host":"orangepi","state":"idle"}'
```

Mac saw:

```
::ffff:10.0.0.40 - - [07/Sep/2026 23:57:22] "POST /heartbeat HTTP/1.1" 501 -
```

### install-software.sh

Cross-compile `seb-agent` and copy it over (password prompt a few times for scp/ssh):

```
$ ./scripts/install-software.sh --host orangepi@10.0.0.40 \
    --control-url http://10.0.0.45:8000
     rust-std installed                       24.18 MiB
    Finished `release` profile [optimized] target(s) in 13.04s
copy .../release/seb-agent -> orangepi@10.0.0.40:~/seb-agent
seb-agent  100% 1924KB
wrote orangepi@10.0.0.40:~/seb-agent.env
```

### seb-agent heartbeat

On the board:

```
$ set -a && source ~/seb-agent.env && set +a && ~/seb-agent
heartbeat host=unknown
heartbeat failed: http status: 501
heartbeat host=unknown
heartbeat failed: io: Connection reset by peer (os error 104)
^C
```

`host=unknown` is `HOSTNAME` unset on the image. 501 is the same python server. Connection reset was stopping/restarting that server. Mac side saw the POSTs:

```
::ffff:10.0.0.40 - - [08/Sep/2026 00:13:48] "POST /heartbeat HTTP/1.1" 501 -
::ffff:10.0.0.40 - - [08/Sep/2026 00:13:53] "POST /heartbeat HTTP/1.1" 501 -
::ffff:10.0.0.40 - - [08/Sep/2026 00:14:30] "POST /heartbeat HTTP/1.1" 501 -
::ffff:10.0.0.40 - - [08/Sep/2026 00:14:35] "POST /heartbeat HTTP/1.1" 501 -
```

The board can POST to a control URL on the LAN. Wiring `seb-agent` to a real server that accepts `/heartbeat` is the next step.
