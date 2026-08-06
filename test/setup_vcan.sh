#!/usr/bin/env bash
# Start virtual / lab CAN for BMS stack tests.
#
# Topology (lab):
#   bms_emulator.py  ->  vcan0  --cangw-->  can0  ->  can-bridge
#   GUI charging TX  ->  can0   --cangw-->  vcan0 -> bms_emulator.py
#
# Usage:
#   sudo ./setup_vcan.sh                 # full lab: can0 + vcan0 + gateway
#   sudo ./setup_vcan.sh --can0-only     # boot path: make sure can0 is up
#   sudo ./setup_vcan.sh --emulator      # add vcan0 + gateway (keep can0)
#   sudo ./setup_vcan.sh --teardown-lab  # remove vcan0 + gateway (keep can0)
#   sudo ./setup_vcan.sh --teardown      # remove vcan ifaces + gateway

set -euo pipefail

MODE="lab"
case "${1:-}" in
  --can0-only) MODE="can0-only" ;;
  --emulator) MODE="emulator" ;;
  --teardown-lab) MODE="teardown-lab" ;;
  --teardown) MODE="teardown" ;;
  "" ) MODE="lab" ;;
  -h|--help)
    sed -n '2,16p' "$0" | tr -d '#'
    exit 0
    ;;
  *)
    echo "error: unknown option: $1 (try --help)" >&2
    exit 1
    ;;
esac

need_root() {
  if [[ "$(id -u)" -ne 0 ]]; then
    echo "error: run as root (sudo $0 $*)" >&2
    exit 1
  fi
}

iface_exists() {
  ip link show "$1" &>/dev/null
}

is_vcan() {
  [[ "$(ip -details link show "$1" 2>/dev/null | awk '/vcan/ {print; exit}')" == *vcan* ]] \
    || ip -details link show "$1" 2>/dev/null | grep -q 'vcan'
}

ensure_vcan_module() {
  if ! lsmod | grep -q '^vcan'; then
    modprobe vcan
  fi
  if ! lsmod | grep -q '^can_gw'; then
    modprobe can-gw 2>/dev/null || modprobe can_gw 2>/dev/null || true
  fi
}

create_vcan() {
  local name=$1
  if iface_exists "$name"; then
    if is_vcan "$name"; then
      echo "ok: $name already exists (vcan)"
    else
      echo "error: $name exists but is not vcan — refusing to overwrite hardware iface" >&2
      exit 1
    fi
  else
    ip link add dev "$name" type vcan
    echo "created: $name (vcan)"
  fi
  ip link set up "$name"
  # CAN FD frames need a larger MTU on vcan
  ip link set "$name" mtu 72 2>/dev/null || true
  echo "up: $name"
}

# Boot / prod path: real can0 -> FD 250k/2M (BMS FDCAN2 GUI); missing iface -> lab vcan can0.
ensure_can0() {
  if iface_exists can0; then
    if is_vcan can0; then
      ensure_vcan_module
      ip link set up can0
      ip link set can0 mtu 72 2>/dev/null || true
      echo "up: can0 (vcan)"
    else
      # MCP2518FD / hardware SocketCAN - match BMS GUI bus (FDCAN_FRAME_FD_BRS).
      ip link set can0 down 2>/dev/null || true
      ip link set can0 type can bitrate 250000 dbitrate 2000000 fd on
      ip link set can0 up
      ip link set can0 mtu 72 2>/dev/null || true
      echo "up: can0 (hardware FD 250k/2M)"
    fi
  else
    ensure_vcan_module
    create_vcan can0
  fi
}

clear_cangw_rules() {
  if command -v cangw >/dev/null 2>&1; then
    cangw -F 2>/dev/null || true
  fi
}

setup_gateway() {
  if ! command -v cangw >/dev/null 2>&1; then
    echo "error: cangw not found — install can-utils" >&2
    exit 1
  fi
  clear_cangw_rules
  # -X is required for CAN FD frames (can-bridge only accepts FD).
  # -e echoes on the source so local sniffers also see TX.
  cangw -A -s vcan0 -d can0 -X -e
  cangw -A -s can0 -d vcan0 -X -e
  echo "gateway: vcan0 ↔ can0 (CAN FD + extended IDs)"
}

teardown_lab() {
  clear_cangw_rules
  if iface_exists vcan0 && is_vcan vcan0; then
    ip link set down vcan0 2>/dev/null || true
    ip link delete vcan0 2>/dev/null || true
    echo "deleted: vcan0"
  fi
  echo "lab teardown complete (can0 left in place)"
}

teardown_all() {
  clear_cangw_rules
  for name in vcan0 can0; do
    if iface_exists "$name" && is_vcan "$name"; then
      ip link set down "$name" 2>/dev/null || true
      ip link delete "$name" 2>/dev/null || true
      echo "deleted: $name"
    fi
  done
  echo "teardown complete"
}

main() {
  need_root "$@"

  case "$MODE" in
    can0-only)
      ensure_can0
      echo
      echo "can0 ready for can-bridge"
      ip -brief link show can0
      ;;
    emulator)
      ensure_can0
      ensure_vcan_module
      create_vcan vcan0
      setup_gateway
      echo
      echo "Emulator network ready:"
      echo "  emit on:  vcan0"
      echo "  bridge:   can0"
      ip -brief link show vcan0
      ip -brief link show can0
      ;;
    lab)
      ensure_vcan_module
      create_vcan vcan0
      create_vcan can0
      setup_gateway
      echo
      echo "Network layer ready:"
      echo "  emit on:  vcan0"
      echo "  sniff on: can0   (or CAN_INTERFACE=can0 for can-bridge)"
      ip -brief link show vcan0
      ip -brief link show can0
      ;;
    teardown-lab)
      teardown_lab
      ;;
    teardown)
      teardown_all
      ;;
  esac
}

main "$@"
