# Hardware - Raspberry Pi CAN (MCP2518FD on SPI0)

The BMS GUI talks to the car over **CAN FD** (`can-bridge` uses SocketCAN FD). On the Pi this is a bare **MCP2518FD** on **SPI0.0**, shown as `can0`.

Match BMS **FDCAN2** (GUI / Raspi FD bus): **250 kbit/s** nominal / **2 Mbit/s** data, BRS on.
(FDCAN1 classic 500 kbit/s is a different bus. Do not use that bus here.)

## Wiring (SPI0.0)

| MCP2518FD | Pi GPIO (BCM) | Board pin |
| --------- | ------------- | --------- |
| MOSI / SI | 10            | 19        |
| MISO / SO | 9             | 21        |
| SCLK      | 11            | 23        |
| CS        | 8 (CE0)       | 24        |
| INT       | 25            | 22        |
| 3V3 / GND | 3V3 / GND     | 1 or 17 / 6 |

- Crystal on the breakout: **40 MHz** (must match the overlay `oscillator=`).
- Chip **SDO -> Pi MISO**, **SDI -> Pi MOSI** (do not swap).
- If the module has `/RESET`, hold it high (3V3).
- CANH / CANL -> BMS GUI FD bus when ready. Terminate only if this node is at an end of the stub.

## One-time / reproducible setup

```bash
# Idempotent: writes /boot/firmware/config.txt overlay
sudo ./scripts/setup-mcp2518fd-spi.sh

# Apply device tree
sudo reboot

# After reboot
sudo ./scripts/setup-mcp2518fd-spi.sh --status
```

Overlay installed (under `[all]` in `/boot/firmware/config.txt`):

```ini
dtparam=spi=on
dtoverlay=mcp251xfd,spi0-0,interrupt=25,oscillator=40000000,speed=10000000
```

Script helpers:

```bash
sudo ./scripts/setup-mcp2518fd-spi.sh --probe   # rebind driver -> SPI TX burst (Saleae)
sudo ./scripts/setup-mcp2518fd-spi.sh --status  # pinmux, dmesg, can0 type
```

## Bring-up bitrate (boot)

[`test/setup_vcan.sh`](test/setup_vcan.sh) (installed as `/usr/local/sbin/bms-setup-vcan` by [`scripts/install-boot-services.sh`](scripts/install-boot-services.sh)) brings hardware `can0` up as:

```bash
ip link set can0 type can bitrate 250000 dbitrate 2000000 fd on
ip link set can0 up
```

`bms-can0.service` runs that on boot before `can-bridge`. If the MCP2518FD is not present, boot uses **vcan** `can0` for lab use.

## Validation

### SPI only (no chip on the bus)

1. Overlay is loaded. `pinctrl` shows GPIO9-11 as SPI0.
2. Run `sudo ./scripts/setup-mcp2518fd-spi.sh --probe` while a logic analyzer watches **CE0 / SCLK / MOSI** (board pins 24 / 23 / 19).
3. Expect a short SPI burst. **MISO idle is normal** when nothing is connected.
4. `dmesg` will report `osc=0x00000000` / failed detect. This is expected without the MCP2518FD.
5. `can0` stays **vcan** until the chip probes successfully.

### With MCP2518FD wired

```bash
sudo ./scripts/setup-mcp2518fd-spi.sh --status
# expect: mcp251xfd bound, can0 is NOT vcan
sudo /usr/local/sbin/bms-setup-vcan --can0-only
ip -details link show can0   # FD, 250000 / 2000000, UP
sudo systemctl restart can-bridge
```

Stop the lab gateway when you use the real bus: `sudo systemctl stop bms-vcan-lab`.

## Saleae / logic analyzer (SPI TX check)

| Channel | Signal | Board pin |
| ------- | ------ | --------- |
| 0       | SCLK   | 23        |
| 1       | MOSI   | 19        |
| 2       | MISO   | 21        |
| 3       | CE0    | 24        |
| 4 (opt) | INT    | 22        |

SPI Mode 0, MSB first, sample rate >= 20 MS/s. Arm capture, then run `--probe`.
