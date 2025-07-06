# Environment Variables Configuration

This document explains how to configure and update environment variables for the ISO USB Hub project, specifically focusing on the `TOTAL_POWER_BUDGET` configuration.

## Overview

The project uses environment variables that are embedded at compile time. When you change environment variables, you need to ensure the program is rebuilt to use the new values.

## TOTAL_POWER_BUDGET Configuration

### Default Configuration

The total power budget is configured in `.cargo/config.toml`:

```toml
[env]
DEFMT_LOG = "info"
TOTAL_POWER_BUDGET = "100"
```

### Changing the Power Budget

1. **Edit the configuration file**:
   ```bash
   # Edit .cargo/config.toml
   # Change TOTAL_POWER_BUDGET = "100" to your desired value
   # Example: TOTAL_POWER_BUDGET = "120"
   ```

2. **Force a rebuild**:
   ```bash
   # Method 1: Clean and build
   cargo clean
   cargo build --bin iso-usb-hub
   
   # Method 2: Use the provided alias
   cargo clean-build
   
   # Method 3: Clean and run directly
   cargo clean
   cargo run --bin iso-usb-hub
   ```

### Automatic Rebuild Detection

The project includes a `build.rs` script that automatically detects when:
- The `TOTAL_POWER_BUDGET` environment variable changes
- The `.cargo/config.toml` file is modified

When you run `cargo build` or `cargo run`, the build script will:
- Display the current power budget configuration
- Show whether environment variables override config file values
- Automatically trigger a rebuild if changes are detected

### Build Output Example

When building, you'll see output like:
```
warning: iso-usb-hub@0.1.0: Config file TOTAL_POWER_BUDGET=120, Environment TOTAL_POWER_BUDGET=120
```

This confirms the current power budget being used.

## Runtime Behavior

### Power Allocation Logic

The configured power budget affects the dynamic power allocation:

- **Total Budget**: Set by `TOTAL_POWER_BUDGET` (default: 100W)
- **Minimum Reserved**: 10W per port for Ports 2&3
- **Dynamic Distribution**: Remaining power allocated based on connection status

### Example Allocations

With `TOTAL_POWER_BUDGET = "120"`:
- **No devices**: Port 1: 95W, Port 2: 10W, Port 3: 10W
- **Port 1 only (65W PD)**: Port 1: 65W, Port 2: 27.5W, Port 3: 27.5W
- **All ports connected**: Port 1: 65W, Port 2: 27.5W, Port 3: 27.5W

With `TOTAL_POWER_BUDGET = "80"`:
- **No devices**: Port 1: 55W, Port 2: 10W, Port 3: 10W
- **Port 1 only (65W PD)**: Port 1: 65W, Port 2: 7.5W, Port 3: 7.5W
- **All ports connected**: Port 1: 65W, Port 2: 7.5W, Port 3: 7.5W

## Troubleshooting

### Changes Not Taking Effect

If your environment variable changes aren't reflected:

1. **Verify the config file**:
   ```bash
   grep "TOTAL_POWER_BUDGET" .cargo/config.toml
   ```

2. **Force a clean rebuild**:
   ```bash
   cargo clean
   cargo build --bin iso-usb-hub
   ```

3. **Check build output** for the power budget warning message

### Invalid Values

The system will use default values if:
- The environment variable is not set
- The value cannot be parsed as a number
- The config file is malformed

Valid range: 20W - 200W (recommended)

## Best Practices

1. **Always clean build** after changing environment variables
2. **Verify build output** shows the expected power budget
3. **Test allocation logic** with different device configurations
4. **Document changes** when modifying power budgets for specific deployments

## Integration with Development Workflow

### Quick Commands

```bash
# Check current configuration
grep "TOTAL_POWER_BUDGET" .cargo/config.toml

# Update and rebuild (manual process)
# 1. Edit .cargo/config.toml
# 2. Run clean build
cargo clean && cargo build --bin iso-usb-hub

# Deploy with new configuration
cargo clean && cargo run --bin iso-usb-hub
```

### Cargo Aliases

The project provides convenient aliases in `.cargo/config.toml`:

```bash
# Clean build
cargo clean-build

# Clean run
cargo clean-run
```

These ensure a fresh build when environment variables change.
