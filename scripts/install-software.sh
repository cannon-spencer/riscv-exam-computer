#!/usr/bin/env bash
# Stage 2: install seb-linux + seb-agent on a board that already boots.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

usage() {
  cat <<EOF
Usage: $(basename "$0")
EOF
}

wait_for_board() {
  echo "TODO: wait for SSH or serial on a running board"
}

install_seb_linux() {
  echo "TODO: install $ROOT/platform/seb-linux onto the board"
}

install_seb_agent() {
  echo "TODO: install $ROOT/exam-env/seb-agent and enable the unit"
}

write_control_url() {
  echo "TODO: write CONTROL_URL onto the board"
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h | --help)
        usage
        exit 0
        ;;
      *)
        usage >&2
        echo "error: unknown argument: $1" >&2
        exit 1
        ;;
    esac
  done

  wait_for_board
  install_seb_linux
  install_seb_agent
  write_control_url

  echo "TODO: implementation missing"
  exit 0
}

main "$@"
