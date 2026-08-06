#!/usr/bin/env python3
"""Deprecated alias - use bms_emulator.py (combined lab tool)."""

from __future__ import annotations

import sys

print(
    "note: emit_probe.py is folded into bms_emulator.py\n"
    "      running: python bms_emulator.py\n",
    file=sys.stderr,
)

from bms_emulator import main

if __name__ == "__main__":
    raise SystemExit(main())
