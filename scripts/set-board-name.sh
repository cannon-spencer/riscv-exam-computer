#!/bin/sh
# Runs as root. The agent reports the board by hostname. A fresh card is
# "orangepirv", and this renames it to orangepi-<last 6 of a hardware id> so
# two boards are not the same host. A board that already has another name
# is left alone.
#
# The id is the devicetree serial number, else the burned-in MAC of the first
# physical NIC (random, locally administered MACs are skipped), else
# /etc/machine-id.
set -eu

current="$(hostname)"
id=""
source=""

if [ -r /sys/firmware/devicetree/base/serial-number ]; then
  id="$(tr -d '\000' < /sys/firmware/devicetree/base/serial-number | tr -cd '[:alnum:]' | tr 'A-Z' 'a-z')"
  source="serial number"
fi
case "$id" in *[!0]*) ;; *) id="" ;; esac

if [ -z "$id" ]; then
  for dev in /sys/class/net/*; do
    [ -e "$dev/device" ] || continue
    mac="$(cat "$dev/address")"
    first="${mac%%:*}"
    [ $((0x$first & 2)) -eq 0 ] || continue
    case "$mac" in *[!0:]*) id="$(printf '%s' "$mac" | tr -d ':')"; source="MAC of ${dev##*/}"; break ;; esac
  done
fi

if [ -z "$id" ]; then
  [ -s /etc/machine-id ] || { echo "error: no hardware id or machine-id to name this board" >&2; exit 1; }
  id="$(cat /etc/machine-id)"
  source="/etc/machine-id"
  echo "warning: no hardware id found; using /etc/machine-id, which may not be unique on cloned cards"
fi

want="orangepi-$(printf '%s' "$id" | tail -c 6)"
if [ "$current" = "$want" ]; then
  echo "hostname: $current"
  exit 0
fi
if [ "$current" != orangepirv ]; then
  echo "hostname: $current (kept)"
  exit 0
fi

hostnamectl set-hostname "$want"
if grep -q '^127\.0\.1\.1' /etc/hosts; then
  sed -i "s/^127\.0\.1\.1.*/127.0.1.1\t$want/" /etc/hosts
else
  printf '127.0.1.1\t%s\n' "$want" >> /etc/hosts
fi
echo "hostname: $current -> $want (from $source)"
