#!/usr/bin/env bash
# install-boot-services.sh - can-bridge + BMS UI on boot (systemd)
#
# Builds release artifacts, installs:
#   - can-bridge -> /usr/local/bin + systemd unit (WS :9002)
#   - client dist -> /var/www/bms-ui, Caddy on :80
#
# Usage (on the Pi):
#   sudo ./scripts/install-boot-services.sh

set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "error: run as root (sudo $0)" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
UI_ROOT="${UI_ROOT:-/var/www/bms-ui}"
# Prefer the invoking user for cargo/npm caches when run via sudo.
BUILD_USER="${SUDO_USER:-${USER}}"

run_as_build_user() {
  if [[ "${BUILD_USER}" == "root" ]] || [[ -z "${BUILD_USER}" ]]; then
    "$@"
  else
    sudo -u "${BUILD_USER}" -- "$@"
  fi
}

echo "==> Building can-bridge (release)"
run_as_build_user bash -lc "cd '${REPO_ROOT}/server' && cargo build --release"

echo "==> Installing CAN bring-up helpers + can-bridge"
install -m 755 "${REPO_ROOT}/test/setup_vcan.sh" /usr/local/sbin/bms-setup-vcan
install -m 644 "${SCRIPT_DIR}/bms-can0.service" /etc/systemd/system/bms-can0.service
install -m 644 "${SCRIPT_DIR}/bms-vcan-lab.service" /etc/systemd/system/bms-vcan-lab.service
install -m 755 "${REPO_ROOT}/server/target/release/can-bridge" /usr/local/bin/can-bridge
install -m 644 "${SCRIPT_DIR}/can-bridge.service" /etc/systemd/system/can-bridge.service

echo "==> Building BMS UI"
run_as_build_user bash -lc "cd '${REPO_ROOT}/client' && npm ci && npm run build"

echo "==> Installing UI -> ${UI_ROOT}"
install -d -m 755 "${UI_ROOT}"
rsync -a --delete "${REPO_ROOT}/client/dist/" "${UI_ROOT}/"

echo "==> Installing Caddy (static file server on :80)"
export DEBIAN_FRONTEND=noninteractive
if ! command -v caddy >/dev/null 2>&1; then
  apt-get update -qq
  apt-get install -y -qq caddy
fi
install -d /etc/caddy
install -m 644 "${SCRIPT_DIR}/Caddyfile.prod" /etc/caddy/Caddyfile

echo "==> Enabling services on boot"
systemctl daemon-reload
systemctl enable --now bms-can0
systemctl enable --now can-bridge
systemctl enable --now caddy
# bms-vcan-lab is manual-only (no [Install] section)
systemctl restart bms-can0
systemctl restart can-bridge
systemctl restart caddy

echo "==> Status"
systemctl --no-pager --full status bms-can0 | sed -n '1,12p'
echo
systemctl --no-pager --full status can-bridge | sed -n '1,12p'
echo
systemctl --no-pager --full status caddy | sed -n '1,12p'
echo

HOST="$(hostname -s 2>/dev/null || echo berry)"
cat <<EOF
Done. On every boot:
  - bms-can0    -> can0 up (vcan lab, or hardware if present)
  - can-bridge  -> ws://${HOST}.local:9002  (CAN_INTERFACE=can0)
  - Caddy UI    -> http://${HOST}.local/    (static files in ${UI_ROOT})

For bms_emulator.py (not on boot):
  sudo systemctl start bms-vcan-lab
  cd ${REPO_ROOT}/test && source .venv/bin/activate && python bms_emulator.py
  # when done:
  sudo systemctl stop bms-vcan-lab

After UI code changes:
  cd ${REPO_ROOT}/client && npm run build
  sudo rsync -a --delete dist/ ${UI_ROOT}/

After bridge code changes:
  cd ${REPO_ROOT}/server && cargo build --release
  sudo install -m 755 target/release/can-bridge /usr/local/bin/can-bridge
  sudo systemctl restart can-bridge
EOF
