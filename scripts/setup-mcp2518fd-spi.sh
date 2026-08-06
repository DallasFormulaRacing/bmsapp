#!/usr/bin/env bash
# setup-mcp2518fd-spi.sh - reproducible MCP2518FD -> SPI0.0 -> SocketCAN can0
#
# Permanent boot config for a bare MCP2518FD breakout on the Pi (BMS GUI FD bus).
# Bitrate bring-up (250k / 2M FD) is handled by test/setup_vcan.sh / bms-can0.service.
#
# Usage:
#   sudo ./scripts/setup-mcp2518fd-spi.sh           # make sure /boot overlay is set (idempotent)
#   sudo ./scripts/setup-mcp2518fd-spi.sh --status  # pinmux / dmesg / can0
#   sudo ./scripts/setup-mcp2518fd-spi.sh --probe   # rebind driver (Saleae SPI TX burst)
#   sudo ./scripts/setup-mcp2518fd-spi.sh --reboot  # make sure overlay is set, then reboot
#
# Wiring (SPI0.0):
#   MOSI -> GPIO10 / board 19
#   MISO -> GPIO9  / board 21
#   SCLK -> GPIO11 / board 23
#   CS   -> GPIO8  / board 24 (CE0)
#   INT  -> GPIO25 / board 22
#   3V3 + GND
#   Crystal: 40 MHz
#   Bus: BMS FDCAN2 GUI - 250000 / 2000000 FD (BRS)

set -euo pipefail

CONFIG="${BOOT_CONFIG:-/boot/firmware/config.txt}"
MARKER="# BMS GUI CAN: MCP2518FD on SPI0.0"
OVERLAY_LINE="dtoverlay=mcp251xfd,spi0-0,interrupt=25,oscillator=40000000,speed=10000000"

need_root() {
  if [[ "$(id -u)" -ne 0 ]]; then
    echo "error: run as root (sudo $0 $*)" >&2
    exit 1
  fi
}

ensure_overlay() {
  need_root
  if [[ ! -f "${CONFIG}" ]]; then
    echo "error: missing ${CONFIG}" >&2
    exit 1
  fi

  if grep -qF "${OVERLAY_LINE}" "${CONFIG}"; then
    echo "ok: overlay already in ${CONFIG}"
  else
    # Drop a prior partial block we own, then append a clean one under [all].
    if grep -qF "${MARKER}" "${CONFIG}"; then
      local tmp
      tmp="$(mktemp)"
      awk -v marker="${MARKER}" '
        $0 == marker { skip=1; next }
        skip && /^dtparam=spi=on$/ { next }
        skip && /^dtoverlay=mcp251xfd,/ { skip=0; next }
        skip && /^$/ { skip=0; next }
        { print }
      ' "${CONFIG}" >"${tmp}"
      cat "${tmp}" >"${CONFIG}"
      rm -f "${tmp}"
    fi

    if ! grep -q '^\[all\]' "${CONFIG}"; then
      printf '\n[all]\n' >>"${CONFIG}"
    fi

    cat >>"${CONFIG}" <<EOF

${MARKER}
dtparam=spi=on
${OVERLAY_LINE}
EOF
    echo "updated: ${CONFIG} (reboot required for overlay)"
  fi

  if grep -qF "${OVERLAY_LINE}" "${CONFIG}" && grep -q '^dtparam=spi=on' "${CONFIG}"; then
    echo "config:"
    grep -nE "^(dtparam=spi=on|dtoverlay=mcp251xfd)" "${CONFIG}" || true
  fi
}

show_status() {
  echo "=== boot overlay ==="
  if [[ -f "${CONFIG}" ]]; then
    grep -nE "${MARKER}|^dtparam=spi=on|^dtoverlay=mcp251xfd" "${CONFIG}" || echo "(not present)"
  else
    echo "missing ${CONFIG}"
  fi

  echo
  echo "=== pinmux (expect SPI0 on 9/10/11; INT 25 input) ==="
  if command -v pinctrl >/dev/null 2>&1; then
    pinctrl get 8,9,10,11,25 || true
  else
    echo "pinctrl not available"
  fi

  echo
  echo "=== modules / spi ==="
  lsmod | grep -E 'mcp251xfd|spi_bcm' || echo "(mcp251xfd / spi_bcm not loaded)"
  ls -la /dev/spidev* 2>/dev/null || echo "(no spidev nodes — ok when mcp251xfd owns spi0.0)"
  if [[ -e /sys/bus/spi/devices/spi0.0 ]]; then
    echo "spi0.0 modalias=$(cat /sys/bus/spi/devices/spi0.0/modalias 2>/dev/null)"
    echo "spi0.0 driver=$(basename "$(readlink /sys/bus/spi/devices/spi0.0/driver 2>/dev/null || echo none)")"
  else
    echo "spi0.0: absent (reboot after installing overlay?)"
  fi

  echo
  echo "=== dmesg (mcp251) ==="
  dmesg 2>/dev/null | grep -i mcp251 | tail -10 || echo "(no mcp251 lines)"

  echo
  echo "=== can0 ==="
  if ip link show can0 &>/dev/null; then
    ip -details link show can0
    if ip -details link show can0 2>/dev/null | grep -q vcan; then
      echo "note: can0 is vcan (lab fallback). Real MCP2518FD not detected."
    fi
  else
    echo "can0: absent"
  fi
}

probe_spi() {
  need_root
  if [[ ! -d /sys/bus/spi/drivers/mcp251xfd ]]; then
    modprobe mcp251xfd 2>/dev/null || true
  fi
  if [[ ! -d /sys/bus/spi/drivers/mcp251xfd ]]; then
    echo "error: mcp251xfd driver not available" >&2
    exit 1
  fi
  if [[ ! -e /sys/bus/spi/devices/spi0.0 ]]; then
    echo "error: spi0.0 missing — install overlay and reboot first" >&2
    exit 1
  fi

  echo "Rebinding mcp251xfd on spi0.0 (watch CE0/SCLK/MOSI on Saleae)..."
  echo spi0.0 > /sys/bus/spi/drivers/mcp251xfd/unbind 2>/dev/null || true
  set +e
  echo spi0.0 > /sys/bus/spi/drivers/mcp251xfd/bind
  local rc=$?
  set -e
  sleep 0.2
  dmesg | grep -i mcp251 | tail -5
  if [[ "${rc}" -ne 0 ]]; then
    echo
    echo "probe failed (expected if MCP2518FD is not wired). SPI TX should still appear on the analyzer."
    exit 0
  fi
  echo "probe ok"
}

usage() {
  sed -n '2,22p' "$0" | sed 's/^# \?//'
}

MODE="${1:-ensure}"
case "${MODE}" in
  ""|ensure)
    ensure_overlay
    echo
    echo "Next: reboot if overlay was newly added, then:"
    echo "  sudo $0 --status"
    echo "  sudo $0 --probe    # optional Saleae SPI TX check"
    echo "  sudo /usr/local/sbin/bms-setup-vcan --can0-only   # after chip detects"
    ;;
  --status|status)
    show_status
    ;;
  --probe|probe)
    probe_spi
    ;;
  --reboot|reboot)
    ensure_overlay
    echo "Rebooting..."
    reboot
    ;;
  -h|--help)
    usage
    ;;
  *)
    echo "error: unknown option: ${MODE} (try --help)" >&2
    exit 1
    ;;
esac
