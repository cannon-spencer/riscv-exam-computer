#!/usr/bin/env bash
# Storage and boot benchmark for a board. Used to compare SD card vs NVMe
# root (docs/log/2026-11-03-nvme-root.md).
#
# From the laptop, result saved to cache/bench/:
#   ./scripts/bench-board.sh --host orangepi@10.0.0.40 [--label sd]
#
# On the board itself, result saved to the current directory:
#   sudo ./scripts/bench-board.sh [--label sd]
#
# Reboot the board right before running it so the boot numbers are fresh.
# Measures: boot time, 4k random read/write (fio, queue depth 1), a cold
# read of many small files, and NVMe timeouts (a known JH7110 PCIe issue).

set -euo pipefail

HOST=""
LABEL=""
STDOUT=0
BOARD_PASS="${BOARD_PASS:-orangepi}"
SSH_OPTS=(-o StrictHostKeyChecking=accept-new -o ConnectTimeout=10 -o ServerAliveInterval=30)

usage() {
  echo "Usage: $(basename "$0") [--host <user@ip>] [--label <name>]"
  echo "  --host   run on that board over SSH and save the result under cache/bench/"
  echo "  --label  added to the result file name, e.g. sd, nvme, io2"
  echo "Without --host, run it on the board with sudo."
}

die() { echo "error: $*" >&2; exit 1; }

section() { printf '\n===== %s =====\n' "$1"; }

result_name() {
  local disk
  disk="$(basename "$(findmnt -no SOURCE /)")"
  echo "bench-$(hostname)-${disk}${LABEL:+-$LABEL}-$(date +%Y%m%d-%H%M)"
}

# Runs on the board as root. In --host mode the script itself arrives on
# stdin, so commands that read stdin (apt-get, fio) get </dev/null.
run_benchmarks() {
  local testfile=/var/tmp/bench-fio.test
  trap 'rm -f /var/tmp/bench-fio.test' EXIT

  if ! command -v fio >/dev/null; then
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq fio >/dev/null \
      || die "could not install fio"
  fi

  echo "name: $(result_name)"
  section "board"
  echo "host:    $(hostname)"
  echo "date:    $(date -Is)"
  echo "kernel:  $(uname -r)"
  echo "cmdline: $(cat /proc/cmdline)"
  [[ -n "$LABEL" ]] && echo "label:   $LABEL"
  findmnt / /boot || true

  section "ext4lazyinit (should not be running on a settled filesystem)"
  ps -eo pid,etime,comm | grep '[e]xt4lazyinit' || echo "not running"

  section "boot time"
  systemd-analyze || echo "unavailable"
  section "slowest units"
  systemd-analyze blame 2>/dev/null | head -15 || echo "unavailable"

  section "nvme timeouts this boot"
  dmesg | grep -iE 'nvme.*timeout' || echo "none"

  section "fio 4k random read/write, queue depth 1, 30s"
  fio --name=rand --filename="$testfile" --size=256M --bs=4k --rw=randrw \
    --direct=1 --runtime=30 --time_based --group_reporting </dev/null
  rm -f "$testfile"

  section "cold read of many small files (ls -R /usr)"
  sync
  echo 3 > /proc/sys/vm/drop_caches
  { time ls -R /usr >/dev/null; } 2>&1

  section "nvme timeouts after tests"
  dmesg | grep -iE 'nvme.*timeout' || echo "none"
}

summary() {
  local file="$1" iops
  grep -m1 '^Startup finished' "$file" || echo "boot: n/a"
  iops="$(grep -E '^ +(read|write): IOPS' "$file" | sed -E 's/^ +//; s/,.*//' | paste -sd ' ' -)"
  echo "fio: $iops"
  echo "ls -R /usr: $(grep -m1 '^real' "$file" | awk '{print $2}')"
  echo "nvme timeouts this boot: $(awk '/===== nvme timeouts after tests/ {f = 1; next}
    f && /nvme nvme[0-9]+: .*timeout/ {n++} END {print n + 0}' "$file")"
  echo "full results: $file"
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h | --help) usage; exit 0 ;;
      --host) HOST="${2:?}"; shift 2 ;;
      --label) LABEL="${2:?}"; shift 2 ;;
      --stdout) STDOUT=1; shift ;;
      *) usage >&2; die "unknown argument: $1" ;;
    esac
  done

  if [[ -n "$HOST" ]]; then
    command -v sshpass >/dev/null || die "install sshpass"
    local root dir tmp name
    root="$(cd "$(dirname "$0")/.." && pwd)"
    dir="$root/cache/bench"
    mkdir -p "$dir"
    tmp="$(mktemp)"
    echo "benchmarking $HOST (about a minute)"
    # sudo -S reads the first line of stdin as the password; bash -s runs
    # the rest, so nothing has to be copied to the board.
    {
      printf '%s\n' "$BOARD_PASS"
      cat "$0"
    } | sshpass -p "$BOARD_PASS" ssh "${SSH_OPTS[@]}" "$HOST" \
      "sudo -S -p '' bash -s -- --stdout ${LABEL:+--label $(printf %q "$LABEL")}" >"$tmp" 2>&1 \
      || { cat "$tmp"; rm -f "$tmp"; die "benchmark failed"; }
    name="$(sed -n 's/^name: //p' "$tmp" | head -1)"
    [[ -n "$name" ]] || { cat "$tmp"; rm -f "$tmp"; die "no result from the board"; }
    mv "$tmp" "$dir/$name.txt"
    summary "$dir/$name.txt"
    return
  fi

  [[ $EUID -eq 0 ]] || die "run with sudo on the board, or pass --host from the laptop"
  if [[ "$STDOUT" == 1 ]]; then
    run_benchmarks
  else
    local out
    out="$PWD/$(result_name).txt"
    run_benchmarks >"$out" 2>&1
    summary "$out"
  fi
}

main "$@"
