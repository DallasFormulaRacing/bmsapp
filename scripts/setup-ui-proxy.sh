#!/usr/bin/env bash
# setup-ui-proxy.sh - serve the BMS UI on port 80 (no :5173 in the URL)
#
# Installs Caddy and reverse-proxies http://*:80 -> Vite on 127.0.0.1:5173.
# Keep `npm run dev` (or a production static server) running on 5173.
#
# Usage:
#   sudo ./scripts/setup-ui-proxy.sh

set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "error: run as root (sudo $0)" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_CADDYFILE="${SCRIPT_DIR}/Caddyfile"
DEST_CADDYFILE="/etc/caddy/Caddyfile"
UI_PORT="${UI_PORT:-5173}"

echo "==> Installing caddy"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq caddy

echo "==> Installing Caddyfile -> ${DEST_CADDYFILE}"
install -d /etc/caddy
# Rewrite upstream port if overridden
sed "s/127\\.0\\.0\\.1:5173/127.0.0.1:${UI_PORT}/" "${SRC_CADDYFILE}" >"${DEST_CADDYFILE}"

echo "==> Enabling and restarting caddy"
systemctl enable caddy
systemctl restart caddy

echo "==> Verifying"
systemctl --no-pager --full status caddy | sed -n '1,15p'
echo
if curl -fsS -o /dev/null -w '%{http_code}\n' http://127.0.0.1/; then
  :
else
  echo "warning: port 80 responded with an error — is Vite running on :${UI_PORT}?" >&2
  echo "         start it with: cd client && npm run dev" >&2
fi

cat <<EOF

Done. With Vite running on :${UI_PORT}, open:

  http://berry.local/
  http://bms.local/
  http://192.168.1.60/

(No port number needed.)

EOF
