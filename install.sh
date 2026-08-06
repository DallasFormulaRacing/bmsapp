#!/usr/bin/env bash
# install.sh - list and install BMS project dependencies (Debian/Ubuntu/Raspberry Pi OS)
#
# Installs OS packages, Rust (rustup), Node.js/npm, and client JS deps (npm ci).
# Does NOT install Python packages - use test/requirements.txt in a venv:
#   cd test && python3 -m venv .venv && source .venv/bin/activate && pip install -r requirements.txt
#
# Usage:
#   ./install.sh              # list what will be installed, then install
#   ./install.sh --list       # list only (no changes)
#   sudo ./install.sh         # needed for apt; rustup/npm run as invoking user when possible
#   ./install.sh --with-tauri # also install Linux Tauri desktop build libs
#   ./install.sh --with-dns   # also install dnsmasq + avahi-daemon (LAN hostname helpers)

set -euo pipefail

LIST_ONLY=0
WITH_TAURI=0
WITH_DNS=0

for arg in "$@"; do
  case "$arg" in
    --list|-l) LIST_ONLY=1 ;;
    --with-tauri) WITH_TAURI=1 ;;
    --with-dns) WITH_DNS=1 ;;
    -h|--help)
      sed -n '2,16p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)
      echo "error: unknown option: $arg (try --help)" >&2
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_USER="${SUDO_USER:-${USER}}"
export DEBIAN_FRONTEND=noninteractive

# --- dependency catalog -------------------------------------------------------

# Core apt packages for can-bridge, VCAN lab, UI build/serve, and scripts.
APT_CORE=(
  build-essential
  pkg-config
  curl
  ca-certificates
  git
  rsync
  can-utils          # candump, cangw (lab gateway)
  python3            # venv host only - pip packages NOT installed here
  python3-venv
  caddy              # static UI on :80 (prod)
)

# Optional: Tauri 2 Linux desktop build (client/src-tauri).
APT_TAURI=(
  libwebkit2gtk-4.1-dev
  libayatana-appindicator3-dev
  librsvg2-dev
  patchelf
  libssl-dev
)

# Optional: scripts/setup-lan-dns.sh helpers.
APT_DNS=(
  dnsmasq
  avahi-daemon
)

# Toolchains installed outside apt when missing.
TOOLCHAIN_NOTES=(
  "Rust (rustup): rustc + cargo — required to build server/ (can-bridge)"
  "Node.js >= 18 + npm — required to build client/ (Vite UI)"
)

# Project-local installs (not Python).
PROJECT_NOTES=(
  "client/: npm ci  (package-lock.json)"
  "test/:   skipped — use pip install -r test/requirements.txt in a venv"
)

# --- helpers ------------------------------------------------------------------

run_as_build_user() {
  if [[ "${EUID}" -eq 0 && -n "${SUDO_USER:-}" && "${SUDO_USER}" != "root" ]]; then
    sudo -u "${SUDO_USER}" -H -- "$@"
  else
    "$@"
  fi
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

node_major() {
  node -p "process.versions.node.split('.')[0]" 2>/dev/null || echo 0
}

list_deps() {
  echo "=== BMS project dependencies ==="
  echo
  echo "APT (always):"
  for p in "${APT_CORE[@]}"; do
    printf '  - %s\n' "$p"
  done
  if [[ "${WITH_TAURI}" -eq 1 ]]; then
    echo
    echo "APT (--with-tauri):"
    for p in "${APT_TAURI[@]}"; do
      printf '  - %s\n' "$p"
    done
  else
    echo
    echo "APT Tauri libs: skipped (pass --with-tauri to include)"
  fi
  if [[ "${WITH_DNS}" -eq 1 ]]; then
    echo
    echo "APT (--with-dns):"
    for p in "${APT_DNS[@]}"; do
      printf '  - %s\n' "$p"
    done
  else
    echo
    echo "APT LAN DNS: skipped (pass --with-dns to include)"
  fi
  echo
  echo "Toolchains:"
  for note in "${TOOLCHAIN_NOTES[@]}"; do
    printf '  - %s\n' "$note"
  done
  echo
  echo "Project installs:"
  for note in "${PROJECT_NOTES[@]}"; do
    printf '  - %s\n' "$note"
  done
  echo
  echo "Explicitly NOT installed by this script:"
  echo "  - Python pip packages (see test/requirements.txt)"
  echo
}

install_apt() {
  if [[ "${EUID}" -ne 0 ]]; then
    echo "error: apt install requires root — re-run with: sudo $0 $*" >&2
    exit 1
  fi

  local pkgs=("${APT_CORE[@]}")
  if [[ "${WITH_TAURI}" -eq 1 ]]; then
    pkgs+=("${APT_TAURI[@]}")
  fi
  if [[ "${WITH_DNS}" -eq 1 ]]; then
    pkgs+=("${APT_DNS[@]}")
  fi

  echo "==> apt-get update"
  apt-get update -qq

  echo "==> apt-get install (${#pkgs[@]} packages)"
  apt-get install -y "${pkgs[@]}"
}

install_rust() {
  if have_cmd rustc && have_cmd cargo; then
    echo "==> Rust already installed: $(run_as_build_user rustc --version)"
    return
  fi
  echo "==> Installing Rust via rustup (user: ${BUILD_USER})"
  run_as_build_user bash -lc \
    'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
  # shellcheck disable=SC1091
  if [[ -f "${HOME}/.cargo/env" ]]; then
    # When run under sudo, cargo lives in SUDO_USER home.
    :
  fi
  run_as_build_user bash -lc 'source "$HOME/.cargo/env" && rustc --version && cargo --version'
}

install_node() {
  if have_cmd node && have_cmd npm && [[ "$(node_major)" -ge 18 ]]; then
    echo "==> Node already installed: $(node -v) / npm $(npm -v)"
    return
  fi

  if [[ "${EUID}" -ne 0 ]]; then
    echo "error: installing Node.js requires root (or install Node >= 18 yourself)" >&2
    exit 1
  fi

  echo "==> Installing Node.js 20.x (NodeSource)"
  # NodeSource setup for Debian/Ubuntu/Raspberry Pi OS
  apt-get install -y ca-certificates curl gnupg
  mkdir -p /etc/apt/keyrings
  curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
    | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg
  echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_20.x nodistro main" \
    > /etc/apt/sources.list.d/nodesource.list
  apt-get update -qq
  apt-get install -y nodejs

  echo "==> Node: $(node -v) / npm $(npm -v)"
  if [[ "$(node_major)" -lt 18 ]]; then
    echo "error: Node $(node -v) is too old (need >= 18 for Vite 6)" >&2
    exit 1
  fi
}

install_client_npm() {
  echo "==> client/: npm ci"
  run_as_build_user bash -lc "cd '${SCRIPT_DIR}/client' && npm ci"
}

# --- main ---------------------------------------------------------------------

list_deps

if [[ "${LIST_ONLY}" -eq 1 ]]; then
  exit 0
fi

echo "Installing…"
echo

install_apt "$@"
install_node
install_rust
install_client_npm

echo
echo "Done."
echo
echo "Next steps:"
echo "  # Python lab tools (separate):"
echo "  cd ${SCRIPT_DIR}/test && python3 -m venv .venv && source .venv/bin/activate && pip install -r requirements.txt"
echo
echo "  # Build & run bridge:"
echo "  cd ${SCRIPT_DIR}/server && cargo build --release"
echo
echo "  # Optional: enable boot services (bridge + UI):"
echo "  sudo ${SCRIPT_DIR}/scripts/install-boot-services.sh"
echo
echo "  # Optional: lab CAN:"
echo "  sudo ${SCRIPT_DIR}/test/setup_vcan.sh --can0-only   # or --emulator for vcan0"
echo
