# can-bridge

Rust WebSocket bridge (from DFR pie backend). Listens on `0.0.0.0:9002` and sends CAN-derived events to the BMS UI.

## Run

```bash
cd server
cargo run
```

Optional env:

- `CAN_INTERFACE` - SocketCAN iface (default `can0`). If open fails, the WebSocket server still starts.
- `CAN_DAQ_POLL=1` - enable Nucleo DAQ IMU/temp polling (off by default).

When CAN is open, the bridge always polls BMS GUI telemetry. Every 500 ms the bridge sends empty Raspi->BMS requests for `0xB000`, `0xB101`-`0xB106`, `0xB111`-`0xB112`. The bridge only decodes BMS->Raspi replies (same command IDs).

Client default URL: `ws://<hostname>:9002` (for example `ws://berry.local:9002`).

## Install on boot (Pi) - bridge + website

One-shot install (builds release bridge, builds UI, enables systemd units):

```bash
sudo ../scripts/install-boot-services.sh
```

That starts on every boot:

| Service | Port | Role |
|---|---|---|
| `can-bridge` | `:9002` | WebSocket <-> CAN |
| `caddy` | `:80` | Static BMS UI (`/var/www/bms-ui`) |

Bridge only (no UI):

```bash
cargo build --release
sudo install -m 755 target/release/can-bridge /usr/local/bin/can-bridge
sudo cp ../scripts/can-bridge.service /etc/systemd/system/can-bridge.service
sudo systemctl daemon-reload
sudo systemctl enable --now can-bridge
```

## Protocol

JSON envelopes match `client/src/types/bridge.ts` (`deviceRegistrySnapshot`, `deviceStatusChanged`, `bmsTelemetry`, ...).
