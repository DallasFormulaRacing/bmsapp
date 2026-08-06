#!/usr/bin/env python3
"""Sniff SocketCAN / VCAN (including CAN FD + extended IDs).

Default interface is can0 - the side can-bridge listens on after setup_vcan.sh
forwards vcan0 -> can0.

Usage:
  python sniff.py              # can0
  python sniff.py --iface vcan0
  python sniff.py --count 5
"""

from __future__ import annotations

import argparse
import struct
import sys
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
    decode_id,
    format_id,
)

CMD_LABELS = {
    BMS_BATTERY_PACK_DATA: "BMS pack data",
    BMS_SEGMENT_TEMPS_HALF_1: "BMS temps half 1",
    BMS_SEGMENT_TEMPS_HALF_2: "BMS temps half 2",
    BMS_GUI_CONNECT: "Charging Control: started",
    BMS_GUI_CHARGING_REQUEST: "BMS charging request",
    BMS_GUI_DISCONNECT: "Charging Control: disconnect",
    BMS_GUI_CHARGING_SOFTSTOP: "Charging Control: stopped",
    **{cmd: f"BMS cell voltages pack {i + 1}" for i, cmd in enumerate(CELL_VOLTAGE_CMDS)},
}


def describe_payload(command: int, data: bytes) -> list[str]:
    """Human-readable lines for known GUI / charging frames."""
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


def main() -> int:
    parser = argparse.ArgumentParser(description="Sniff CAN/VCAN frames")
    parser.add_argument("--iface", default="can0", help="SocketCAN interface (default: can0)")
    parser.add_argument("--count", type=int, default=0, help="Exit after N frames (0 = forever)")
    parser.add_argument("--timeout", type=float, default=None, help="Exit after N seconds with no RX")
    args = parser.parse_args()

    bus = can.Bus(channel=args.iface, interface="socketcan", fd=True)
    print(f"sniffing {args.iface} (fd=True) — Ctrl+C to stop", flush=True)

    seen = 0
    deadline = time.monotonic() + args.timeout if args.timeout else None

    try:
        while True:
            remaining = None
            if deadline is not None:
                remaining = max(0.0, deadline - time.monotonic())
                if remaining == 0.0:
                    print("timeout: no more wait", flush=True)
                    break

            msg = bus.recv(timeout=remaining if remaining is not None else 1.0)
            if msg is None:
                if deadline is not None and time.monotonic() >= deadline:
                    print("timeout: no frames", flush=True)
                    return 1
                continue

            seen += 1
            raw = msg.arbitration_id
            command = None
            try:
                id_str = format_id(raw) if msg.is_extended_id else f"0x{raw:03X} (std)"
                parts = decode_id(raw)
                command = parts["command"]
            except Exception:
                id_str = f"0x{raw:08X}"

            data_hex = msg.data.hex(" ")
            flags = []
            if msg.is_fd:
                flags.append("FD")
            if msg.is_extended_id:
                flags.append("EXT")
            if msg.is_remote_frame:
                flags.append("RTR")
            flag_s = ",".join(flags) if flags else "-"

            label = CMD_LABELS.get(command, "") if command is not None else ""
            label_s = f"  {label}" if label else ""

            print(
                f"[{seen}] {args.iface} {id_str} dlc={msg.dlc} [{flag_s}] "
                f"data={data_hex}{label_s}",
                flush=True,
            )

            if command is not None:
                for line in describe_payload(command, bytes(msg.data)):
                    print(f"       -> {line}", flush=True)

            if args.count and seen >= args.count:
                break
    except KeyboardInterrupt:
        print("\nstopped", flush=True)
    finally:
        bus.shutdown()

    return 0 if seen > 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
