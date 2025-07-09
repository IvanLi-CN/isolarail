# W25Q128 Flash Programmer

A tool for programming the W25Q128JVPIQ SPI Flash memory connected to the STM32G431CB microcontroller via ST-Link.

## Overview

This tool allows you to:
- Program data to the W25Q128 Flash memory
- Verify written data
- Erase sectors, blocks, or the entire chip
- Read and dump Flash contents
- Get device information

## Hardware Connections

The W25Q128 Flash is connected to SPI2 on the STM32G431CB:

| W25Q128 Pin | STM32 Pin | Function |
|-------------|-----------|----------|
| CS          | PB12      | SPI2_NSS |
| CLK         | PB13      | SPI2_SCK |
| DI (MOSI)   | PB15      | SPI2_MOSI|
| DO (MISO)   | PA10      | SPI2_MISO|
| VCC         | 3.3V      | Power    |
| GND         | GND       | Ground   |

## Building and Running

### Prerequisites

1. Rust toolchain with `thumbv7em-none-eabihf` target
2. ST-Link debugger/programmer
3. STM32G431CB development board with W25Q128 Flash

### Build

```bash
cd tools/flash_programmer
cargo build --release
```

### Flash and Run

```bash
# Using cargo run (recommended)
cargo run --release

# Or using probe-rs
probe-rs run --chip STM32G431CBUx target/thumbv7em-none-eabihf/release/flash_programmer

# Or using OpenOCD + GDB
openocd -f interface/stlink.cfg -f target/stm32g4x.cfg &
arm-none-eabi-gdb target/thumbv7em-none-eabihf/release/flash_programmer
```

## Usage Examples

### Programming Custom Data

1. Replace the content of `test_data.bin` with your data
2. Modify the `program_address` in `main.rs` if needed
3. Build and run the tool

### Programming Bitmap Data

```rust
// In main.rs, replace the test data with your bitmap
let bitmap_data = include_bytes!("../your_bitmap.bin");
let bitmap_address = 0x000000; // Start address for bitmap

programmer.program_and_verify(bitmap_address, bitmap_data).await?;
```

### Erasing Flash

```rust
// Erase a specific sector (4KB)
programmer.erase_sector(sector_number).await?;

// Erase multiple sectors
programmer.erase_sectors(start_sector, count).await?;

// Erase entire chip (use with caution!)
programmer.erase_chip().await?;
```

### Reading Flash Contents

```rust
// Read data into buffer
let mut buffer = [0u8; 1024];
programmer.read_data(address, &mut buffer).await?;

// Dump Flash contents to debug output
programmer.dump_flash(start_address, length).await?;
```

## Memory Layout

The W25Q128 has the following memory organization:

- **Total Size**: 16MB (0x000000 - 0xFFFFFF)
- **Page Size**: 256 bytes (programming unit)
- **Sector Size**: 4KB (minimum erase unit)
- **Block Size**: 64KB (fast erase unit)

### Recommended Memory Map

| Address Range | Size | Purpose |
|---------------|------|---------|
| 0x000000 - 0x0FFFFF | 1MB | Startup bitmaps and UI graphics |
| 0x100000 - 0x7FFFFF | 7MB | Application data and assets |
| 0x800000 - 0xEFFFFF | 7MB | User data and configuration |
| 0xF00000 - 0xFFFFFF | 1MB | Reserved/backup area |

## Programming Flow

1. **Initialize**: Set up SPI2 and initialize W25Q128 driver
2. **Identify**: Read device ID to verify connection
3. **Erase**: Erase required sectors before programming
4. **Program**: Write data in page-sized chunks (256 bytes)
5. **Verify**: Read back and compare written data
6. **Complete**: Report success or failure

## Error Handling

The tool provides comprehensive error reporting:

- SPI communication errors
- Device identification failures
- Erase/program failures
- Verification mismatches
- Address/length validation errors

## Debugging

Enable defmt logging to see detailed operation progress:

```bash
# View logs in real-time
cargo run --release
```

The tool outputs:
- Device information
- Operation progress
- Memory dumps
- Error details

## Safety Considerations

- **Backup Important Data**: Always backup existing Flash contents before erasing
- **Verify Connections**: Ensure proper SPI connections before programming
- **Power Stability**: Ensure stable power supply during programming
- **Address Validation**: Double-check addresses to avoid overwriting critical data

## Troubleshooting

### Device Not Found
- Check SPI connections
- Verify power supply (3.3V)
- Ensure correct pin assignments

### Programming Failures
- Check for write protection
- Verify erase operations completed
- Ensure sufficient power supply

### Verification Errors
- Check for electrical noise
- Verify SPI timing/frequency
- Test with smaller data chunks

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
