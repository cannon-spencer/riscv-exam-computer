# Laptop control server + Cloudflare tunnel

Dropped the Oracle Always Free VM from [2026-09-16](2026-09-16-oracle-server-heartbeat.md). The 1 GB box OOMed on package installs, needed swap + Podman, and still needed NSG + `firewalld` + `--network host` just to get basic ACKs.

Public name:

```
https://riscv-exam-computer.download
```

That is HTTPS 443 to Cloudflare. Cloudflare forwards HTTP to Axum on the laptop. Nothing opens port 8000 on the home router or campus NAT.

### One-time tunnel

Domain `riscv-exam-computer.download` lives in Cloudflare. Tunnel name `exam-server`, id `9fee6539-bb37-4ef5-8768-0cfe465cf52d`.

```
~/.cloudflared/cert.pem
~/.cloudflared/9fee6539-bb37-4ef5-8768-0cfe465cf52d.json
~/.cloudflared/config.yml
```

`config.yml` points the hostname at localhost:

```yaml
tunnel: 9fee6539-bb37-4ef5-8768-0cfe465cf52d
credentials-file: ~/.cloudflared/9fee6539-bb37-4ef5-8768-0cfe465cf52d.json

ingress:
  - hostname: riscv-exam-computer.download
    service: http://127.0.0.1:8000
  - service: http_status:404
```

The JSON is the secret. `cert.pem` is only for creating/changing tunnels. The URL itself is public — anyone who knows it can POST `/heartbeat` until we add auth.

### Deploy from the laptop

Docker must already be running (Colima on this Mac, Docker Desktop on Windows). The script does not start or stop the engine.

```bash
./scripts/deploy-server.sh
```

Builds `ghcr.io/cannon-spencer/exam-server:latest` from `exam-env/server`, `docker run -p 8000:8000`, then blocks on `cloudflared tunnel run exam-server`. Ctrl-C stops the tunnel and `docker stop exam-server`. 

Through the live url:

```bash
curl -sS -X POST https://riscv-exam-computer.download/heartbeat \
  -H 'Content-Type: application/json' \
  -d '{"host":"test","state":"idle"}'
```

```
{"ok":true}
```

### Caveats 

A clone is not enough. We need: Cloudflared,  Docker running, and the tunnel JSON + `config.yml`. Only **one** laptop can `tunnel run exam-server` at a time.

Windows: same bash script in WSL if `docker` and `cloudflared` are on `PATH`. 