#!/usr/bin/env python3
"""Single BMS lab tool: online/offline, request/response telemetry, charging RX.

Replaces emit_probe.py. When connected, this tool answers Raspi telemetry polls
so the GUI sees a real BMS node (charging Start/Stop under the BMS-online gate).

  can-bridge polls Raspi->BMS (0xB000 / B101-B106 / B111-B112)
  bms_emulator.py replies BMS->Raspi with the same command IDs + payloads
  GUI charging TX on can0 --cangw--> vcan0 -> this process (printed)

Usage:
  sudo systemctl start bms-vcan-lab   # or: sudo ./setup_vcan.sh --emulator
  source .venv/bin/activate
  python bms_emulator.py              # interactive (auto-connects)
  python bms_emulator.py --auto       # connected until Ctrl-C
"""

from __future__ import annotations

import argparse
import random
import struct
import sys
import threading
import time

try:
    import can
except ImportError:
    print("error: python-can not installed. Run: pip install -r requirements.txt", file=sys.stderr)
    sys.exit(1)

from dfr_can import (
    BMS_BATTERY_PACK_DATA,
    BMS_GUI_CHARGING_REQUEST,
    BMS_GUI_CHARGING_SOFTSTOP,
    BMS_GUI_CONNECT,
    BMS_GUI_DISCONNECT,
    BMS_SEGMENT_TEMPS_HALF_1,
    BMS_SEGMENT_TEMPS_HALF_2,
    CELL_VOLTAGE_CMDS,
    CELLS_PER_SEGMENT,
    CMD_PONG,
    NODE_BMS,
    NODE_RASPI,
    SEGMENT_COUNT,
    THERM_CAN_OFFSET,
    THERMISTORS_PER_SEGMENT,
    bms_to_raspi_id,
    decode_id,
    encode_id,
    format_id,
    raspi_to_bms_id,
)

NOMINAL_CELL_V = 3.70
CELL_V_MIN = 3.45
CELL_V_MAX = 4.15
NOMINAL_TEMP_C = 28.0

TELEMETRY_REQUEST_CMDS = frozenset(
    (BMS_BATTERY_PACK_DATA, BMS_SEGMENT_TEMPS_HALF_1, BMS_SEGMENT_TEMPS_HALF_2, *CELL_VOLTAGE_CMDS)
)


class PackState:
    """Slow-walk RNG pack state - realistic drift, not pure noise each tick."""

    def __init__(self, seed: int | None = None) -> None:
        self.rng = random.Random(seed)
        self.cell_mv: list[list[int]] = []
        self.therm_c: list[list[int]] = []
        self.soc_tenths = 720
        self.current_ma = -15_000
        self._init_cells()

    def _init_cells(self) -> None:
        self.cell_mv = []
        self.therm_c = []
        for seg in range(SEGMENT_COUNT):
            bias_mv = int((seg - 2.5) * 8)
            cells = [
                int(
                    (NOMINAL_CELL_V + self.rng.uniform(-0.04, 0.04)) * 1000
                    + bias_mv
                    + self.rng.randint(-15, 15)
                )
                for _ in range(CELLS_PER_SEGMENT)
            ]
            self.cell_mv.append(cells)
            temps = [
                int(NOMINAL_TEMP_C + (seg - 2.5) * 1.2 + self.rng.uniform(-2.0, 3.0))
                for _ in range(THERMISTORS_PER_SEGMENT)
            ]
            self.therm_c.append([max(15, min(55, t)) for t in temps])

    def tick(self) -> None:
        self.soc_tenths += int(-self.current_ma / 80_000) + self.rng.choice([-1, 0, 0, 0, 1])
        self.soc_tenths = max(50, min(980, self.soc_tenths))

        self.current_ma += self.rng.randint(-800, 800)
        mode = self.rng.random()
        if mode < 0.05:
            self.current_ma = self.rng.randint(5_000, 25_000)
        elif mode < 0.12:
            self.current_ma = self.rng.randint(-2_000, 2_000)
        else:
            self.current_ma = max(-120_000, min(-3_000, self.current_ma))

        for seg in range(SEGMENT_COUNT):
            for i in range(CELLS_PER_SEGMENT):
                v = self.cell_mv[seg][i] + self.rng.randint(-3, 3)
                mean = sum(self.cell_mv[seg]) // CELLS_PER_SEGMENT
                if abs(v - mean) > 40:
                    v += 1 if v < mean else -1
                self.cell_mv[seg][i] = max(
                    int(CELL_V_MIN * 1000),
                    min(int(CELL_V_MAX * 1000), v),
                )
            for i in range(THERMISTORS_PER_SEGMENT):
                t = self.therm_c[seg][i] + self.rng.choice([-1, 0, 0, 0, 1])
                self.therm_c[seg][i] = max(18, min(48, t))

    @property
    def pack_voltage_deci(self) -> int:
        total_mv = sum(sum(seg) for seg in self.cell_mv)
        return max(0, min(0xFFFF, total_mv // 100))

    def encode_pack_data(self) -> bytes:
        buf = bytearray(24)
        struct.pack_into(
            ">HHh",
            buf,
            0,
            self.pack_voltage_deci & 0xFFFF,
            self.soc_tenths & 0xFFFF,
            self.current_ma,
        )
        return bytes(buf)

    def encode_cell_voltages(self, segment: int) -> bytes:
        cells = self.cell_mv[segment]
        return struct.pack(">" + "h" * CELLS_PER_SEGMENT, *cells)

    def encode_therm_half(self, half: int) -> bytes:
        buf = bytearray(64)
        start = 0 if half == 0 else 3
        offset = THERM_CAN_OFFSET
        for seg in range(start, start + 3):
            for temp_c in self.therm_c[seg]:
                buf[offset] = temp_c & 0xFF
                offset += 1
        return bytes(buf)


def describe_charging_rx(command: int, data: bytes) -> list[str]:
    if command == BMS_GUI_CONNECT:
        return ["Charging Control: started"]
    if command == BMS_GUI_CHARGING_SOFTSTOP:
        return ["Charging Control: stopped"]
    if command == BMS_GUI_DISCONNECT:
        return ["Charging Control: stopped (disconnect)"]
    if command == BMS_GUI_CHARGING_REQUEST and len(data) >= 4:
        volts_deci, amps_deci = struct.unpack_from(">HH", data, 0)
        lines = [
            f"Parameters sent: Voltage = {volts_deci / 10.0:.1f} V",
            f"Parameters sent: Current = {amps_deci / 10.0:.1f} A",
        ]
        if len(data) >= 6:
            (delta_mv,) = struct.unpack_from(">H", data, 4)
            lines.append(f"Parameters sent: delta = {delta_mv} mV")
        return lines
    return []


class BmsLab:
    """Request/response telemetry TX + charging-command RX."""

    def __init__(self, iface: str, interval: float, seed: int | None) -> None:
        self.iface = iface
        self.interval = interval  # kept for status / manual tick pacing only
        self.pack = PackState(seed=seed)
        self._bus: can.Bus | None = None
        self._connected = False
        self._stop = threading.Event()
        self._rx_thread: threading.Thread | None = None
        self._cycles = 0
        self._lock = threading.Lock()
        self._pong_id = encode_id(1, NODE_RASPI, CMD_PONG, NODE_BMS)

    @property
    def connected(self) -> bool:
        return self._connected

    def open(self) -> None:
        self._bus = can.Bus(channel=self.iface, interface="socketcan", fd=True)
        self._stop.clear()
        self._rx_thread = threading.Thread(target=self._rx_loop, name="bms-rx", daemon=True)
        self._rx_thread.start()
        print(f"BMS lab on {self.iface}", flush=True)
        print(
            f"  reply TX example={format_id(bms_to_raspi_id(BMS_BATTERY_PACK_DATA))}",
            flush=True,
        )
        print(
            f"  expect polls={format_id(raspi_to_bms_id(BMS_BATTERY_PACK_DATA))}",
            flush=True,
        )
        print("  RX: telemetry polls (B000/B101–B106/B111–B112) + charging cmds", flush=True)

    def close(self) -> None:
        with self._lock:
            self._connected = False
        self._stop.set()
        if self._rx_thread and self._rx_thread.is_alive():
            self._rx_thread.join(timeout=2.0)
        if self._bus is not None:
            self._bus.shutdown()
            self._bus = None

    def connect(self) -> None:
        with self._lock:
            if self._connected:
                print("already connected (answering polls)", flush=True)
                return
            self._connected = True
        print(
            "CONNECTED — answering Raspi telemetry polls "
            "(GUI BMS online once replies arrive; charging unlocked)",
            flush=True,
        )

    def disconnect(self) -> None:
        with self._lock:
            if not self._connected:
                print("already disconnected", flush=True)
                return
            self._connected = False
        print(
            "DISCONNECTED — ignoring telemetry polls "
            "(GUI BMS offline ~3s; charging locked)",
            flush=True,
        )

    def status(self) -> None:
        state = "connected" if self._connected else "disconnected"
        pv = self.pack.pack_voltage_deci / 10.0
        soc = self.pack.soc_tenths / 10.0
        amps = self.pack.current_ma / 1000.0
        print(
            f"status: {state}  reply_cycles={self._cycles}  "
            f"Vpack={pv:.1f}V  SOC={soc:.1f}%  I={amps:+.1f}A",
            flush=True,
        )

    def _send(self, arbitration_id: int, data: bytes) -> None:
        if self._bus is None:
            raise RuntimeError("bus not open")
        self._bus.send(
            can.Message(
                arbitration_id=arbitration_id,
                data=data,
                is_extended_id=True,
                is_fd=True,
                bitrate_switch=False,
            )
        )

    def reply_telemetry(self, command: int) -> None:
        """Answer one Raspi poll with the matching BMS->Raspi payload."""
        if command == BMS_BATTERY_PACK_DATA:
            self.pack.tick()
            self._send(bms_to_raspi_id(command), self.pack.encode_pack_data())
            self._cycles += 1
            if self._cycles == 1 or self._cycles % 10 == 0:
                self.status()
            return
        if command in CELL_VOLTAGE_CMDS:
            seg = CELL_VOLTAGE_CMDS.index(command)
            self._send(bms_to_raspi_id(command), self.pack.encode_cell_voltages(seg))
            return
        if command == BMS_SEGMENT_TEMPS_HALF_1:
            self._send(bms_to_raspi_id(command), self.pack.encode_therm_half(0))
            return
        if command == BMS_SEGMENT_TEMPS_HALF_2:
            self._send(bms_to_raspi_id(command), self.pack.encode_therm_half(1))
            return

    def send_cycle(self) -> None:
        """Debug: unsolicited full burst (normal path is request/response)."""
        self.pack.tick()
        self._send(bms_to_raspi_id(BMS_BATTERY_PACK_DATA), self.pack.encode_pack_data())
        for seg, cmd in enumerate(CELL_VOLTAGE_CMDS):
            self._send(bms_to_raspi_id(cmd), self.pack.encode_cell_voltages(seg))
        self._send(bms_to_raspi_id(BMS_SEGMENT_TEMPS_HALF_1), self.pack.encode_therm_half(0))
        self._send(bms_to_raspi_id(BMS_SEGMENT_TEMPS_HALF_2), self.pack.encode_therm_half(1))
        self._cycles += 1

    def send_pong(self) -> None:
        """Lightweight presence frame (old emit_probe behavior)."""
        self._send(self._pong_id, bytes([0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]))

    def _rx_loop(self) -> None:
        assert self._bus is not None
        while not self._stop.is_set():
            try:
                msg = self._bus.recv(timeout=0.2)
            except Exception as exc:  # noqa: BLE001
                print(f"rx error: {exc}", flush=True)
                continue
            if msg is None or not msg.is_extended_id:
                continue
            try:
                parts = decode_id(msg.arbitration_id)
            except Exception:
                continue
            if parts["target"] != NODE_BMS or parts["source"] != NODE_RASPI:
                continue
            cmd = parts["command"]

            lines = describe_charging_rx(cmd, bytes(msg.data))
            if lines:
                print(f"\n<< GUI->BMS {format_id(msg.arbitration_id)}", flush=True)
                for line in lines:
                    print(f"   -> {line}", flush=True)
                print("bms> ", end="", flush=True)
                continue

            if cmd not in TELEMETRY_REQUEST_CMDS:
                continue
            with self._lock:
                answering = self._connected
            if not answering:
                continue
            try:
                self.reply_telemetry(cmd)
            except Exception as exc:  # noqa: BLE001
                print(f"reply error: {exc}", flush=True)


def run_interactive(iface: str, interval: float, seed: int | None) -> int:
    lab = BmsLab(iface, interval, seed)
    lab.open()
    # Ready to answer polls immediately (process up <-> BMS online once bridge polls).
    lab.connect()
    print(
        "\nBMS lab (request/response telemetry + charging RX):\n"
        "  (auto-connected — answers can-bridge polls; Ctrl+C / q -> offline ~3s)\n"
        "  c / connect     answer telemetry polls\n"
        "  d / disconnect  ignore telemetry polls\n"
        "  s / status      pack snapshot\n"
        "  t / tick        unsolicited full cycle (debug)\n"
        "  p / pong        one lightweight presence frame\n"
        "  q / quit\n",
        flush=True,
    )
    try:
        while True:
            try:
                line = input("bms> ").strip().lower()
            except EOFError:
                print(flush=True)
                break
            if line in ("c", "connect", "on", "start"):
                lab.connect()
            elif line in ("d", "disconnect", "off", "stop"):
                lab.disconnect()
            elif line in ("s", "status"):
                lab.status()
            elif line in ("t", "tick", "once"):
                try:
                    lab.send_cycle()
                    print(f"sent unsolicited cycle #{lab._cycles}", flush=True)
                except Exception as exc:  # noqa: BLE001
                    print(f"tick error: {exc}", flush=True)
            elif line in ("p", "pong", "probe"):
                try:
                    lab.send_pong()
                    print("sent pong", flush=True)
                except Exception as exc:  # noqa: BLE001
                    print(f"pong error: {exc}", flush=True)
            elif line in ("q", "quit", "exit"):
                break
            elif line == "":
                continue
            else:
                print("unknown — try c/d/s/t/p/q", flush=True)
    except KeyboardInterrupt:
        print("\ninterrupted", flush=True)
    finally:
        lab.close()
    return 0


def run_auto(iface: str, interval: float, seed: int | None) -> int:
    lab = BmsLab(iface, interval, seed)
    lab.open()
    lab.connect()
    try:
        while True:
            time.sleep(1.0)
    except KeyboardInterrupt:
        print("\ninterrupted", flush=True)
    finally:
        lab.close()
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="DFR BMS lab: request/response telemetry + charging RX"
    )
    parser.add_argument("--iface", default="vcan0", help="Interface (default: vcan0)")
    parser.add_argument(
        "--interval",
        type=float,
        default=0.5,
        help="Unused for TX pacing (bridge polls); kept for CLI compat",
    )
    parser.add_argument("--seed", type=int, default=None, help="RNG seed")
    parser.add_argument(
        "--auto",
        action="store_true",
        help="Connect and answer polls until Ctrl-C (no prompt)",
    )
    args = parser.parse_args()

    if args.auto:
        return run_auto(args.iface, args.interval, args.seed)
    return run_interactive(args.iface, args.interval, args.seed)


if __name__ == "__main__":
    raise SystemExit(main())
