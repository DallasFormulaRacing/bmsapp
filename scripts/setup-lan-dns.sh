#!/usr/bin/env bash
# setup-lan-dns.sh - LAN hostnames for the BMS UI on this Pi
#
# What this does:
#   1. Installs dnsmasq (+ makes sure avahi-daemon is present for .local names)
#   2. Finds this machine's primary LAN interface + IPv4 address
#   3. Writes /etc/dnsmasq.d/bms-lan.conf (bms.lan + berry.lan -> this Pi)
#   4. Makes sure /etc/dnsmasq.conf loads /etc/dnsmasq.d/
#   5. Adds matching /etc/hosts entries (so the Pi itself resolves the names)
#   6. Publishes bms.local via Avahi/mDNS (works on WLAN without router DNS changes)
#   7. Enables and restarts dnsmasq (+ reloads avahi)
#
# Names:
#   http://bms.local:5173/  - works on most WLAN devices via mDNS (no router change)
#   http://bms.lan:5173/    - needs clients to use this Pi as DNS (router DHCP DNS)
#
# Usage:
#   sudo ./scripts/setup-lan-dns.sh
#   sudo BMS_DNS_NAME=pack.lan ./scripts/setup-lan-dns.sh

set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "error: run as root (sudo $0)" >&2
  exit 1
fi

BMS_DNS_NAME="${BMS_DNS_NAME:-bms.lan}"
ALT_DNS_NAME="${ALT_DNS_NAME:-berry.lan}"
BMS_MDNS_NAME="${BMS_MDNS_NAME:-bms.local}"
DNSMASQ_DROPIN="/etc/dnsmasq.d/bms-lan.conf"
AVAHI_HOSTS="/etc/avahi/hosts"
UPSTREAM_ROUTER_DNS="${UPSTREAM_ROUTER_DNS:-192.168.1.1}"

echo "==> Installing dnsmasq and avahi-daemon"
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq dnsmasq avahi-daemon

echo "==> Detecting primary LAN interface / address"
ROUTE_LINE="$(ip -4 route get 1.1.1.1)"
IP="$(awk '{for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit }}' <<<"${ROUTE_LINE}")"
IFACE="$(awk '{for (i = 1; i <= NF; i++) if ($i == "dev") { print $(i + 1); exit }}' <<<"${ROUTE_LINE}")"

if [[ -z "${IP}" || -z "${IFACE}" ]]; then
  echo "error: could not detect LAN IP/interface from: ${ROUTE_LINE}" >&2
  exit 1
fi

echo "    interface=${IFACE}"
echo "    address=${IP}"
echo "    dns names=${BMS_DNS_NAME} ${ALT_DNS_NAME}"
echo "    mDNS name=${BMS_MDNS_NAME}"

echo "==> Writing ${DNSMASQ_DROPIN}"
cat >"${DNSMASQ_DROPIN}" <<EOF
# BMS LAN DNS - managed by Projects/bms-app/scripts/setup-lan-dns.sh
# Resolves ${BMS_DNS_NAME} (and ${ALT_DNS_NAME}) to this Pi for WLAN clients
# that use this host as their DNS server.

domain-needed
bogus-priv
no-resolv
server=${UPSTREAM_ROUTER_DNS}
server=1.1.1.1
server=8.8.8.8

interface=${IFACE}
bind-interfaces

local=/lan/
domain=lan
expand-hosts

address=/${BMS_DNS_NAME}/${IP}
address=/${ALT_DNS_NAME}/${IP}
EOF

echo "==> Ensuring /etc/dnsmasq.conf loads /etc/dnsmasq.d/"
if ! grep -qE '^conf-dir=/etc/dnsmasq\.d/' /etc/dnsmasq.conf; then
  echo 'conf-dir=/etc/dnsmasq.d/,*.conf' >>/etc/dnsmasq.conf
fi

echo "==> Updating /etc/hosts for local resolution"
HOSTS_LINE="${IP} ${BMS_DNS_NAME} ${ALT_DNS_NAME} ${BMS_MDNS_NAME}"
if grep -qE "[[:space:]]${BMS_DNS_NAME//./\\.}([[:space:]]|$)" /etc/hosts; then
  sed -i -E "s/^[0-9.]+[[:space:]]+${BMS_DNS_NAME//./\\.}.*/${HOSTS_LINE}/" /etc/hosts
else
  echo "${HOSTS_LINE}" >>/etc/hosts
fi

echo "==> Publishing ${BMS_MDNS_NAME} via Avahi"
mkdir -p /etc/avahi
touch "${AVAHI_HOSTS}"
# Remove prior managed line(s), then append current mapping
sed -i -E "/[[:space:]]${BMS_MDNS_NAME//./\\.}([[:space:]]|$)/d" "${AVAHI_HOSTS}"
echo "${IP} ${BMS_MDNS_NAME}" >>"${AVAHI_HOSTS}"

echo "==> Enabling and restarting services"
systemctl enable dnsmasq avahi-daemon
systemctl restart dnsmasq
systemctl restart avahi-daemon

echo "==> Verifying"
systemctl --no-pager --full status dnsmasq | sed -n '1,12p'
echo
getent hosts "${BMS_DNS_NAME}" || true
if command -v host >/dev/null 2>&1; then
  host "${BMS_DNS_NAME}" 127.0.0.1 || true
elif command -v dig >/dev/null 2>&1; then
  dig +short @"${IP}" "${BMS_DNS_NAME}" A || true
fi

cat <<EOF

Done.

Use these URLs (include the port):
  http://${BMS_MDNS_NAME}:5173/   ← try this first (mDNS, no router change)
  http://${IP}:5173/              ← always works on the WLAN
  http://${BMS_DNS_NAME}:5173/    ← needs Pi as DNS (see below)

For ${BMS_DNS_NAME} on other devices, set router DHCP DNS to:
  ${IP}
then reconnect Wi‑Fi.

EOF
