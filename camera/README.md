# Oko Camera

> [!WARNING]
> Building/Running has only been tested on Windows 11 23H2 in WSL2.

## Prerequisites

- RISC-V and Xtensa Tooling
  - <https://docs.esp-rs.org/book/installation/riscv-and-xtensa.html>
- `std` Development Requirements
  - <https://docs.esp-rs.org/book/installation/std-requirements.html>
- `espflash`
  - <https://docs.esp-rs.org/book/tooling/espflash.html>

### Windows

> [!WARNING]
> The default CH340 drivers on Windows 11 are likely to cause issues, see:
>
> <https://forum.arduino.cc/t/a-device-attached-to-the-system-is-not-functioning/1165392/9>

- Windows to WSL2 USB Passthrough
  - <https://learn.microsoft.com/en-us/windows/wsl/connect-usb>

## Build and Flash/Run

> [!NOTE]
> The baud rate is set to `921600` in [`espflash.toml`](./espflash.toml), try
> lowering this value to `115200` if you're experiencing flashing issues.

```bash
source ~/export-esp.sh # Needed to download components in "components_esp32.lock"
cargo build
cargo run
```
