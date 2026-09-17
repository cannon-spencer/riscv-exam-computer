# Oracle Cloud control server + Pi heartbeat

### What runs where


| Piece                         | Where                                       | Role                                                                   |
| ----------------------------- | ------------------------------------------- | ---------------------------------------------------------------------- |
| `exam-env/server`             | Oracle VM (Podman, host network, port 8000) | `POST /heartbeat` → `{"ok":true}`                                      |
| `exam-env/seb-agent`          | Orange Pi                                   | every 5s POST to `CONTROL_URL/heartbeat`                               |
| `scripts/deploy-server.sh`    | Mac                                         | Docker-build the server image, SSH it to the VM, restart the container |
| `scripts/install-software.sh` | Mac                                         | zig-cross-compile agent, scp to the Pi, write `~/seb-agent.env`        |


Local secrets:

```
SERVER_HOST=opc@129.213.203.27
CONTROL_URL=http://129.213.203.27:8000
```

The VM is **Oracle Linux 9**, **x86_64**, Always Free, **~1 GB RAM**. It is slow and will freeze or OOM if you `dnf` anything heavy without swap. It is **not** Ampere/arm64 - images must be `linux/amd64`.

SSH uses a **personal public key** pasted at instance create. User is `opc`.

```bash
ssh opc@129.213.203.27
```

### Colima + image

Homebrew `docker` is only the CLI. Start the engine, then build from `exam-env/server` (or just use the deploy script).

```bash
colima start
docker info
```

Image is a two-stage Dockerfile: compile with `rust:1-bookworm`, copy the binary into `debian:bookworm-slim`. Manual build (the script does this with `--platform linux/amd64`):

```bash
cd exam-env/server
DOCKER_BUILDKIT=1 docker build --platform linux/amd64 \
  -t ghcr.io/cannon-spencer/exam-server:latest .
```

Need `docker-buildx` so BuildKit works.

### Deploy to the VM

```bash
./scripts/deploy-server.sh
```

Reads `SERVER_HOST`. Builds `linux/amd64`, `docker save | gzip` over SSH, `sudo docker load`, then:

```bash
sudo docker rm -f exam-server
sudo docker run -d --name exam-server --restart=always --network host \
  ghcr.io/cannon-spencer/exam-server:latest
```

`--network host` is required. `-p 8000:8000` accepted TCP from the internet but never sent the HTTP body back. Host net binds Axum on the VM’s port 8000 directly.

Successful deploy ended with:

```
Loaded image: ghcr.io/cannon-spencer/exam-server:latest
listening at http://129.213.203.27:8000/heartbeat
```

From the Mac:

```bash
curl -sS -X POST http://129.213.203.27:8000/heartbeat \
  -H 'Content-Type: application/json' \
  -d '{"host":"test","state":"idle"}'
```

```
{"ok":true}
```

### First-time VM setup

Swap first, then Podman (provides a `docker` CLI):

```bash
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
free -h

sudo dnf install -y podman podman-docker
sudo systemctl enable --now podman-restart
sudo docker info
```

Open **8000** on the guest **and** in OCI (NSG or subnet security list, ingress TCP 8000 from `0.0.0.0/0`). Guest:

```bash
sudo firewall-cmd --permanent --add-port=8000/tcp
sudo firewall-cmd --reload
sudo firewall-cmd --list-ports
```

### Point the Pi at Oracle

Agent default is `http://127.0.0.1:8000` unless `CONTROL_URL` is set. From the Mac (when SSH to the board works):

```bash
./scripts/install-software.sh --host orangepi@10.0.0.xx
```

Run:

```bash
set -a && source ~/seb-agent.env && set +a
export HOSTNAME="$(hostname)"
~/seb-agent
```

Board output (working):

```
heartbeat host=orangepirv
heartbeat host=orangepirv
heartbeat host=orangepirv
```

No `heartbeat failed:` line. The agent does not print `{"ok":true}` confirmed on the VM:

```bash
sudo docker logs -f exam-server
```

We see `heartbeat {"host":"orangepirv","state":"idle"}` every ~5s.

