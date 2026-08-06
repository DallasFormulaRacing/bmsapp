# Hardware-free stack validation

Prove layers from bottom to top: **VCAN network -> can-bridge -> GUI**.

```
bms_emulator.py  ->  vcan0  --cangw-->  can0  ->  can-bridge  ->  UI :9002
GUI charging TX  ->  can0   --cangw-->  vcan0 -> bms_emulator.py (prints)
```

## One-time setup

```bash
sudo apt install -y can-utils
cd test
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

**can0 on boot** (installed by `sudo ../scripts/install-boot-services.sh`):
`bms-can0.service` brings `can0` up before `can-bridge`. With an MCP2518FD on SPI0 this is hardware FD at 250k/2M. If not, lab uses **vcan**. See [HARDWARE.md](../HARDWARE.md).

**Emulator network** (opt-in, not on boot):

```bash
sudo systemctl start bms-vcan-lab   # vcan0 + cangw <-> can0
# or: sudo ./setup_vcan.sh --emulator
```

Stop lab net: `sudo systemctl stop bms-vcan-lab`
Full teardown: `sudo ./setup_vcan.sh --teardown`

## Full lab (one program)

`bms_emulator.py` replaces `emit_probe.py`. **Request/response:** can-bridge polls Raspi->BMS (`0xB000` / `B101`-`B106` / `B111`-`B112`). The emulator replies BMS->Raspi with the same command IDs. Connect = answer polls (BMS online). Disconnect = ignore polls (offline ~3s).

```bash
# terminal A - lab net (if not already up) + bridge
sudo systemctl start bms-vcan-lab
# bridge is usually already running via systemd; or:
# cd ../server && CAN_INTERFACE=can0 cargo run

# terminal B - BMS lab (auto-connects on start -> UI BMS online)
cd ../test && source .venv/bin/activate
python bms_emulator.py
#   c / connect     BMS online + telemetry + charging unlocked
#   d / disconnect  BMS offline (~3s) + charging locked
#   s / status
#   t / tick        one telemetry cycle
#   p / pong        lightweight presence only
#   q / quit        (same as kill - BMS offline ~3s)
```

Open the UI:

1. Start emulator -> Status BMS up, Overview/Segments fill (CAN/bridge stay up independently)
2. Charging Session -> Start -> lab prints:
   - `Charging Control: started`
   - `Parameters sent: Voltage / Current / delta`
3. Stop -> `Charging Control: stopped`
4. Stop/kill emulator (or `d`) -> BMS offline ~3s, values clear, charging locked; CAN stays up

Optional sniffer: `python sniff.py --iface can0`

Non-interactive stream: `python bms_emulator.py --auto --interval 0.5`

## Network-only smoke (optional)

```bash
python sniff.py --iface can0 --count 3   # terminal A
python bms_emulator.py                   # terminal B, then: t
```

**Pass:** sniffer on `can0` sees frames from `vcan0`. Gateway needs `cangw -X` (CAN FD).

## Emulator

Start

```
sudo systemctl start bms-vcan-lab
cd ~/Projects/bms-app/test && source .venv/bin/activate
python bms_emulator.py
```

Stop

```
sudo systemctl stop bms-vcan-lab
```

## Notes

- Charging CAN needs BMS online (prod rule). Emulator auto-connects on start. `d` or quit -> offline.
- DAQ Nucleo polling is off by default. Enable again with `CAN_DAQ_POLL=1`.
- `emit_probe.py` is a deprecated alias that starts `bms_emulator.py`.
