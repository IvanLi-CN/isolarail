# ESP32-S3 Installation Guide

This guide will help you set up the ESP32-S3 Rust development environment.

## Quick Setup

### Option 1: Automatic Setup (Recommended)

Run the following commands in your terminal:

```bash
# Install espup (ESP Rust toolchain installer)
cargo install espup

# Install ESP32 toolchain
espup install

# Source the environment
source ~/export-esp.sh

# Install espflash (for flashing ESP32 devices)
cargo install espflash
```

### Option 2: Manual Installation

If the automatic setup fails due to network issues:

1. **Download espup manually**:

   For macOS ARM64 (M1/M2):

   ```bash
   curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-apple-darwin -o ~/.cargo/bin/espup
   chmod +x ~/.cargo/bin/espup
   ```

   For macOS Intel:

   ```bash
   curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-apple-darwin -o ~/.cargo/bin/espup
   chmod +x ~/.cargo/bin/espup
   ```

2. **Install ESP32 toolchain**:

   ```bash
   espup install
   source ~/export-esp.sh
   ```

3. **Install espflash**:

   ```bash
   cargo install espflash
   ```

## Verification

After installation, verify everything is working:

```bash
# Check if ESP32-S3 target is available
rustup target list | grep esp32s3

# You should see: xtensa-esp32s3-none-elf

# Check espflash is installed
espflash --version

# Try building the project
cargo build
```

## Environment Setup

Add this line to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
source ~/export-esp.sh
```

## Building and Flashing

Once everything is set up:

```bash
# Build the project
cargo build

# Flash to ESP32-S3 device (connect your board first)
cargo run
```

## Troubleshooting

### "Target not found" error

If you get errors about `xtensa-esp32s3-none-elf` target not found:

1. Make sure you've run `espup install`
2. Source the environment: `source ~/export-esp.sh`
3. Restart your terminal

### Network timeout errors

If you encounter network timeouts:

1. Try the manual installation method above
2. Check your internet connection
3. Try using a VPN if you're behind a firewall

### Permission errors

If you get permission errors:

1. Make sure `~/.cargo/bin` is in your PATH
2. Check that espup has execute permissions: `chmod +x ~/.cargo/bin/espup`

## Hardware Requirements

- ESP32-S3 development board
- USB cable for programming and power
- Computer with macOS, Linux, or Windows

## Next Steps

Once installation is complete, you can:

1. Build the hello world project: `cargo build`
2. Flash to your ESP32-S3: `cargo run`
3. Monitor serial output in the terminal
4. Start developing your own ESP32-S3 applications!

For more information, visit:

- [ESP-RS Book](https://docs.espressif.com/projects/rust/)
- [ESP-HAL Documentation](https://docs.espressif.com/projects/rust/esp-hal/latest/)
- [Embassy Documentation](https://embassy.dev/)
