#!/usr/bin/env python3
"""Watch can-bridge WebSocket until BMS (nodeId 28 / 0x1C) is online.

Usage:
  python ws_watch.py
  python ws_watch.py --url ws://127.0.0.1:9002 --timeout 15
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys

try:
    import websockets
except ImportError:
    print("error: websockets not installed. pip install websockets", file=sys.stderr)
    sys.exit(1)

BMS_NODE_ID = 0x1C


async def main() -> int:
    parser = argparse.ArgumentParser(description="Watch bridge WS for BMS online")
    parser.add_argument("--url", default="ws://127.0.0.1:9002")
    parser.add_argument("--timeout", type=float, default=20.0)
    args = parser.parse_args()

    print(f"connecting {args.url}", flush=True)
    async with websockets.connect(args.url) as ws:
        deadline = asyncio.get_event_loop().time() + args.timeout
        bms_online = False

        while asyncio.get_event_loop().time() < deadline:
            remaining = deadline - asyncio.get_event_loop().time()
            try:
                raw = await asyncio.wait_for(ws.recv(), timeout=max(0.1, remaining))
            except asyncio.TimeoutError:
                break

            msg = json.loads(raw)
            data = msg.get("data") or {}
            dtype = data.get("type")
            print(f"← {dtype}", flush=True)

            if dtype == "deviceRegistrySnapshot":
                for d in data.get("devices", []):
                    if d.get("nodeId") == BMS_NODE_ID:
                        print(f"  BMS snapshot: online={d.get('online')} name={d.get('name')}", flush=True)
                        bms_online = bool(d.get("online"))
            elif dtype == "deviceStatusChanged":
                device = data.get("device") or {}
                if device.get("nodeId") == BMS_NODE_ID:
                    print(
                        f"  BMS status: online={device.get('online')} "
                        f"lastSeenMsAgo={device.get('lastSeenMsAgo')}",
                        flush=True,
                    )
                    bms_online = bool(device.get("online"))

            if bms_online:
                print("VERIFY: BMS online via can-bridge WS", flush=True)
                return 0

        print("VERIFY: FAIL — BMS never went online", flush=True)
        return 1


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
