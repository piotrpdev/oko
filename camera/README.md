# Oko Camera

## Prerequisites

- Windows to WSL2 USB Passthrough (if using Windows)
  - <https://learn.microsoft.com/en-us/windows/wsl/connect-usb>
- RISC-V and Xtensa Tooling
  - <https://docs.esp-rs.org/book/installation/riscv-and-xtensa.html>
- `std` Development Requirements
  - <https://docs.esp-rs.org/book/installation/std-requirements.html>
- `espflash`
  - <https://docs.esp-rs.org/book/tooling/espflash.html>

## Build and Flash/Run

```bash
source ~/export-esp.sh
cargo build
cargo run
```
