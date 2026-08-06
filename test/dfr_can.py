"""DFR 29-bit extended CAN ID helpers and BMS GUI command constants.

Matches server/src/can/message.rs and the BMS firmware can_id_build layout:
  [28:26] Priority | [25:21] Target | [20:5] Command | [4:0] Source
"""

from __future__ import annotations

NODE_UNKNOWN = 0x00
NODE_ALL = 0x01
NODE_BMS = 0x1C
NODE_RASPI = 0x1E

CMD_PING = 0xA001
CMD_PONG = 0xA002

# BMS GUI application commands (firmware <-> Raspi)
BMS_BATTERY_PACK_DATA = 0xB000
BMS_CELL_VOLTAGES_PACK_1 = 0xB101
BMS_CELL_VOLTAGES_PACK_2 = 0xB102
BMS_CELL_VOLTAGES_PACK_3 = 0xB103
BMS_CELL_VOLTAGES_PACK_4 = 0xB104
BMS_CELL_VOLTAGES_PACK_5 = 0xB105
BMS_CELL_VOLTAGES_PACK_6 = 0xB106
BMS_SEGMENT_TEMPS_HALF_1 = 0xB111
BMS_SEGMENT_TEMPS_HALF_2 = 0xB112
BMS_IMD_DATA = 0xBA01
BMS_GUI_CONNECT = 0xBB01
BMS_GUI_CHARGING_REQUEST = 0xBB02
BMS_GUI_DISCONNECT = 0xBB03
BMS_GUI_CHARGING_SOFTSTOP = 0xBBFF

# Pack layout (matches GUI / firmware pack framing)
SEGMENT_COUNT = 6
CELLS_PER_SEGMENT = 24
THERMISTORS_PER_SEGMENT = 20
THERM_CAN_OFFSET = 4  # first 4 bytes of therm frames are zero

CELL_VOLTAGE_CMDS = (
    BMS_CELL_VOLTAGES_PACK_1,
    BMS_CELL_VOLTAGES_PACK_2,
    BMS_CELL_VOLTAGES_PACK_3,
    BMS_CELL_VOLTAGES_PACK_4,
    BMS_CELL_VOLTAGES_PACK_5,
    BMS_CELL_VOLTAGES_PACK_6,
)

# Payload scale factors (provisional; shared by emulator + bridge decode)
# Cell voltages: int16 BE millivolts (3700 -> 3.700 V)
# Pack voltage:  uint16 BE deci-volts (5328 -> 532.8 V)
# SOC:           uint16 BE in 0.1% units (752 -> 75.2%)
# Current:       int16 BE milliamps (signed; discharge negative)
# Thermistors:   uint8 = high byte of uint16 aux; emulator stores deg C in that byte


def encode_id(
    priority: int,
    target: int,
    command: int,
    source: int,
) -> int:
    if not 0 <= priority <= 0x07:
        raise ValueError(f"priority out of range: {priority}")
    if not 0 <= target <= 0x1F:
        raise ValueError(f"target out of range: {target}")
    if not 0 <= command <= 0xFFFF:
        raise ValueError(f"command out of range: {command}")
    if not 0 <= source <= 0x1F:
        raise ValueError(f"source out of range: {source}")
    return (priority << 26) | (target << 21) | (command << 5) | source


def decode_id(raw: int) -> dict[str, int]:
    return {
        "priority": (raw >> 26) & 0x07,
        "target": (raw >> 21) & 0x1F,
        "command": (raw >> 5) & 0xFFFF,
        "source": raw & 0x1F,
    }


def format_id(raw: int) -> str:
    parts = decode_id(raw)
    return (
        f"0x{raw:08X} "
        f"(prio={parts['priority']} tgt=0x{parts['target']:02X} "
        f"cmd=0x{parts['command']:04X} src=0x{parts['source']:02X})"
    )


def bms_to_raspi_id(command: int, priority: int = 0) -> int:
    """Build a BMS -> Raspi response/telemetry ID (firmware send path)."""
    return encode_id(priority, NODE_RASPI, command, NODE_BMS)


def raspi_to_bms_id(command: int, priority: int = 0) -> int:
    """Build a Raspi -> BMS request ID (same command; target/source swapped)."""
    return encode_id(priority, NODE_BMS, command, NODE_RASPI)
