# DFR CAN Standard

## CAN ID Structure (29-bit extended ID)

| Bits    | Field    | Width   | Description            |
| ------- | -------- | ------- | ---------------------- |
| [28:26] | Priority | 3 bits  | Message priority (0-7) |
| [25:21] | Target   | 5 bits  | Destination device ID  |
| [20:5]  | Command  | 16 bits | Command/message type   |
| [4:0]   | Source   | 5 bits  | Originating device ID  |

---

## Device IDs (5 bits, range 0x00-0x1F)

| Name            | Hex    | Binary  | Description                    |
| --------------- | ------ | ------- | ------------------------------ |
| `NODE_UNKNOWN`  | `0x00` | `00000` | Unknown / uninitialized device |
| `NODE_ALL`      | `0x01` | `00001` | Broadcast - targets all nodes  |
| `NODE_FL`       | `0x02` | `00010` | Front-left wheel node          |
| `NODE_FR`       | `0x03` | `00011` | Front-right wheel node         |
| `NODE_RL`       | `0x04` | `00100` | Rear-left wheel node           |
| `NODE_RR`       | `0x05` | `00101` | Rear-right wheel node          |
| `NODE_NUCLEO_1` | `0x06` | `00110` | Nucleo board 1                 |
| `NODE_NUCLEO_2` | `0x07` | `00111` | Nucleo board 2                 |
| `NODE_VCU`      | `0x1B` | `?`     | Vehicle Control Unit           |
| `NODE_BMS`      | `0x1C` | `11111` | Battery Management System      |
| `NODE_DASH`     | `0x1D` | `11101` | Dashboard                      |
| `NODE_RASPI`    | `0x1E` | `11110` | Raspberry Pi (main controller) |

---

## Commands (16 bits, range 0x0000-0xFFFF)

### Common Commands in all states

| Name       | Hex      | Command Binary     | Description                          | DLC | Data Description                       |
| ---------- | -------- | ------------------ | ------------------------------------ | --- | -------------------------------------- |
| `CMD_PING` | `0xA001` | `1010000000000001` | Ping a device to check presence/mode | `0` | No data                                |
| `CMD_PONG` | `0xA002` | `1010000000000010` | Ping response                        | `1` | Byte 0: mode (`0` bootloader, `1` app) |

### DAQ Application Commands

| Name                       | Hex      | Command Binary     | Description                        | DLC   | Data Description                    |
| -------------------------- | -------- | ------------------ | ---------------------------------- | ----- | ----------------------------------- |
| `CMD_REQ_IMU_DATA`         | `0xD101` | `1101000100000001` | Request IMU data from a device     | `0`   | No data                             |
| `CMD_REQ_TEMP_DATA`        | `0xD102` | `1101000100000010` | Request Temperature data           | `0`   | No data                             |
| `CMD_REQ_SPEED_DATA`       | `0xD103` | `1101000100000011` | Request Wheel Speed data           | `0`   | No data                             |
| `CMD_REQ_RIDE_HEIGHT_DATA` | `0xD104` | `1101000100000100` | Request Ride Height data           | `0`   | No data                             |
| `CMD_IMU_DATA`             | `0xD201` | `1101001000000001` | Response that carries IMU data     | `TBD` | TBD: IMU sample payload             |
| `CMD_TEMP_DATA`            | `0xD202` | `1101001000000010` | Response that carries Temperature data | `64`  | Bytes `0..31`: 16 tire temp samples as `i16` little-endian deci-C. Bytes `32..63`: 16 brake temp samples as `i16` little-endian deci-C |
| `CMD_SPEED_DATA`           | `0xD203` | `1101001000000011` | Response that carries Wheel Speed data | `TBD` | TBD: wheel speed sample payload     |
| `CMD_RIDE_HEIGHT_DATA`     | `0xD204` | `1101001000000100` | Response that carries Ride Height data | `TBD` | TBD: ride height sample payload     |
| `CMD_SET_LED`              | `0xD301` | `1101001100000001` | Set LED state on a device          | `1`   | Byte 0: LED state (`0` off, `1` on) |
| `CMD_RESET_NODE`           | `0xDF01` | `1101111100000001` | Soft reset a node                  | `0`   | No data                             |
| `CMD_REQ_UUID`             | `0xDF02` | `1101111100000010` | Request node UUID                  | `0`   | No data                             |
| `CMD_REQ_FW_VER`           | `0xDF03` | `1101111100000011` | Request node firmware version      | `0`   | No data                             |

### BMS GUI Application Commands

| Name                        | Hex      | Command Binary | Description                                                   | DLC   | Data Description                             |
| --------------------------- | -------- | -------------- | ------------------------------------------------------------- | ----- | -------------------------------------------- |
| `BMS_CELL_VOLTAGES_PACK_1`  | `0xB101` | ``             | All cell voltages of segment # 1 (closest to BMS Controller)  | `48`  |                                              |
| `BMS_CELL_VOLTAGES_PACK_2`  | `0xB102` | ``             | All cell voltages of segment # 2                              | `48`  |                                              |
| `BMS_CELL_VOLTAGES_PACK_3`  | `0xB103` | ``             | All cell voltages of segment # 3                              | `48`  |                                              |
| `BMS_CELL_VOLTAGES_PACK_4`  | `0xB104` | ``             | All cell voltages of segment # 4                              | `48`  |                                              |
| `BMS_CELL_VOLTAGES_PACK_5`  | `0xB105` | ``             | All cell voltages of segment # 5                              | `48`  |                                              |
| `BMS_CELL_VOLTAGES_PACK_6`  | `0xB106` | ``             | All cell voltages of segment # 6 (furthest from BMS Controller) | `48`  |                                              |
| `BMS_SEGMENT_TEMPS_HALF_1`  | `0xB111` | ``             | Thermistors for first 3 segments (20 x 3 = 60 readings)       | `???` |                                              |
| `BMS_SEGMENT_TEMPS_HALF_2`  | `0xB112` | ``             | Thermistors for last 3 segments (20 x 3 = 60 readings)        | `???` |                                              |
| `BMS_BATTERY_PACK_DATA`     | `0xB000` | ``             | pack volt, soc est, current, cell balancing stats             | `24`  |                                              |
| `BMS_IMD_DATA`              | `0xBA01` | `???`          | IMD data                                                      | `???` | ???                                          |
| `BMS_GUI_CONNECT`           | `0xBB01` | `???`          | Connect to laptop and start charger FSM                       | `0`   | ???                                          |
| `BMS_GUI_CHARGING_REQUEST`  | `0xBB02` | `???`          | Send power details to ELCON                                   | `6`   | `u16 BE` volts, `u16 BE` amps, `u16 BE` delta mV  |
| `BMS_GUI_DISCONNECT`        | `0xBB03` | `???`          | Disconnect from laptop and stop charger FSM                   | `0`   | ???                                          |
| `BMS_GUI_CHARGING_SOFTSTOP` | `0xBBFF` | `???`          | Cut power to the battery pack in a controlled way             | `0`   | ???                                          |

### BMS Normal Commands

| Name                    | Hex      | Command Binary | Description            | DLC  | Data Description                          |
| ----------------------- | -------- | -------------- | ---------------------- | ---- | ----------------------------------------- |
| `BMS_CURRENT_SENSOR`    | `0xBEEF` | ``             | Current Sensor reading | `48` | (Motorola) Current that leaves the pack in mA |
| `BMS_IMD_REQUEST`       | `0xBF22` | ``             | IMD Request            | `8`  | (Intel)                                   |
| `BMS_IMD_RESPONSE`      | `0xBF23` | ``             | IMD Response           | `8`  | (Intel)                                   |
| `BMS_IMD_GENERAL`       | `0xBF37` | ``             | IMD General            | `8`  | (Intel)                                   |
| `BMS_IMD_ISO_DETAIL`    | `0xBF38` | ``             | IMD Isolation detail   | `8`  | (Intel)                                   |
| `BMS_IMD_VOLTAGE`       | `0xBF39` | ``             | IMD Voltage            | `8`  | (Intel)                                   |
| `BMS_IMD_IT_SYSTEM`     | `0xBF3A` | ``             | IMD IT system          | `8`  | (Intel)                                   |
| `BMS_PRECHARGE_FAILURE` | `0xBFAA` | ``             | Precharge              | `8`  | (Intel)                                   |

### Bootloader Commands

| Name               | Hex      | Command Binary     | Description                                  | DLC    | Data Description                    |
| ------------------ | -------- | ------------------ | -------------------------------------------- | ------ | ----------------------------------- |
| `BL_CMD_ERASE`     | `0xF001` | `1111000000000001` | Erase flash before you write new firmware    | `0`    | No data                             |
| `BL_CMD_ERASE_OK`  | `0xF002` | `1111000000000010` | Acknowledge successful flash erase           | `0`    | No data                             |
| `BL_CMD_WRITE`     | `0xF003` | `1111000000000011` | Write firmware chunk to device               | `1-64` | Firmware chunk bytes                |
| `BL_CMD_WRITE_OK`  | `0xF004` | `1111000000000100` | Acknowledge successful firmware chunk write  | `0`    | No data                             |
| `BL_CMD_ADDR_SIZE` | `0xF005` | `1111000000000101` | Send target address and data size to device  | `8`    | Bytes 0-3: address, bytes 4-7: size |
| `BL_CMD_FW_QUERY`  | `0xF006` | `1111000000000110` | Query current firmware version on device     | `0`    | No data                             |
| `BL_CMD_FW_RESP`   | `0xF007` | `1111000000000111` | Response that carries firmware version info  | `TBD`  | TBD: firmware version payload       |
| `BL_CMD_REBOOT`    | `0xF008` | `1111000000001000` | Reboot a device (stay in bootloader)         | `0`    | No data                             |
| `BL_CMD_JUMP`      | `0xFAAA` | `1111101010101010` | Jump from bootloader to application firmware | `0`    | No data                             |
