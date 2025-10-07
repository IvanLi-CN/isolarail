# ESP32-S3 Hello World

A simple hello world project for ESP32-S3 using esp-hal and Embassy async framework.

## Features

- **ESP32-S3 Support**: Built specifically for ESP32-S3 microcontroller
- **Embassy Async**: Uses Embassy async framework for efficient task management
- **Serial Output**: Prints hello world messages via esp-println
- **Periodic Tasks**: Demonstrates async task spawning and timing

## Hardware Requirements

- **ESP32-S3 Development Board**: Any ESP32-S3 based board
- **USB Connection**: For programming and serial output

## Installation Prerequisites

### Method 1: Using espup (Recommended)

```bash
# Install espup
cargo install espup

# Install ESP32 toolchain
espup install

# Source the environment (add to your shell profile)
source ~/export-esp.sh

# Install espflash
cargo install espflash
```

### Method 2: Manual Installation (if espup fails)

If you encounter network issues with espup, you can try:

1. **Download espup manually**:

   ```bash
   # For macOS ARM64
   curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-apple-darwin -o ~/.cargo/bin/espup
   chmod +x ~/.cargo/bin/espup

   # For macOS Intel
   curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-apple-darwin -o ~/.cargo/bin/espup
   chmod +x ~/.cargo/bin/espup
   ```

2. **Run espup install**:

   ```bash
   espup install
   source ~/export-esp.sh
   ```

### Method 3: Alternative Installation

If all else fails, you can try using the ESP-IDF toolchain directly:

```bash
# Install ESP-IDF prerequisites
brew install cmake ninja dfu-util

# Clone ESP-IDF
git clone --recursive https://github.com/espressif/esp-idf.git ~/esp-idf
cd ~/esp-idf
./install.sh esp32s3

# Source ESP-IDF environment
source ~/esp-idf/export.sh
```

### Verify Installation

After installation, verify that the ESP32-S3 target is available:

```bash
rustup target list | grep esp32s3
```

You should see `xtensa-esp32s3-none-elf` in the list.

## Building and Flashing

### Building

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release
```

### Flashing and Monitoring

```bash
# Flash and monitor serial output
cargo run

# Flash release build
cargo run --release
```

### Makefile helpers (recommended)

To simplify common tasks and ensure defmt logs decode correctly, a `Makefile` is provided. Examples:

```bash
# Build (release by default)
make build

# Flash and monitor with defmt decoding
make run PORT=/dev/tty.usbmodem1101 BAUD=115200

# Attach only to the serial monitor (no flashing), with defmt decoding
make attach PORT=/dev/tty.usbmodem1101 BAUD=115200

# List detected serial ports
make ports
```

Notes:
- If you run `espflash monitor` directly and see garbled output, it is because the app logs with `defmt`.
- Use `make attach` which passes `--log-format defmt` and `--elf target/xtensa-esp32s3-none-elf/<profile>/esp32s3-hello-world` so logs are decoded.
- Default baud is `115200`; override with `BAUD=...` if needed.

## Expected Output

Once flashed and running, you should see output similar to:

```text
ESP32-S3 Hello World Starting!
Main task started, spawning hello task...
Hello World from ESP32-S3! Counter: 0
Main task heartbeat
Hello World from ESP32-S3! Counter: 1
Hello World from ESP32-S3! Counter: 2
...
```

## Project Structure

```text
├── src/
│   └── main.rs              # Main application with hello world logic
├── .cargo/
│   └── config.toml          # Cargo configuration for ESP32-S3
└── Cargo.toml               # Project dependencies and configuration
```

## Documentation

- Hardware connection overview: [docs/hardware_connection_overview.md](docs/hardware_connection_overview.md)

## Dependencies

- `esp-hal`: Hardware abstraction layer for ESP32 series
- `esp-hal-embassy`: Embassy integration for esp-hal
- `embassy-executor`: Async task executor
- `embassy-time`: Async time utilities
- `esp-println`: Serial output for ESP32
- `esp-backtrace`: Panic handler and backtrace support

## Troubleshooting

### Target Not Found

If you get "target not found" errors, make sure you've:

1. Installed espup: `cargo install espup`
2. Run espup install: `espup install`
3. Sourced the environment: `source ~/export-esp.sh`

### Build Errors

If you encounter build errors, try:

1. Clean the project: `cargo clean`
2. Update dependencies: `cargo update`
3. Check that the ESP toolchain is properly installed

## CI

This repository's GitHub Actions use the official `esp-rs/xtensa-toolchain` action to install the ESP Xtensa Rust toolchain.

- Target: `xtensa-esp32s3-none-elf`
- Toolchain: `+esp` (installed via the action)
- Workflows: see `.github/workflows/`

## License

This project is licensed under the MIT License.

## Development Notes

- Power input qualification (VIN): during active development the firmware relaxes the undervoltage floor to `VIN_MIN_V = 4.5 V` to allow 5 V bench/USB supplies for fan testing and bring-up. This intentionally bypasses the production undervoltage guardrail. Restore the 9.0 V minimum before release and re-verify input sequencing. See `docs/software_design.md` §2 for details.
