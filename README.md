# Battery Management System GUI

## How to start

List or install OS and toolchain dependencies (not Python pip packages):

```bash
./install.sh --list
sudo ./install.sh                 # apt + Node + Rust + client npm ci
sudo ./install.sh --with-tauri    # + Tauri Linux desktop libs
sudo ./install.sh --with-dns      # + dnsmasq / avahi
```

Install Python lab packages from `test/requirements.txt` (venv + `pip install -r`).

## Hardware (Pi CAN)

MCP2518FD on SPI0 -> `can0` (FD 250k/2M). See [HARDWARE.md](HARDWARE.md) and run:

```bash
sudo ./scripts/setup-mcp2518fd-spi.sh          # permanent boot overlay
sudo ./scripts/setup-mcp2518fd-spi.sh --status
sudo ./scripts/setup-mcp2518fd-spi.sh --probe  # SPI TX burst for logic analyzer
```
