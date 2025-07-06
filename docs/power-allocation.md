# Power Allocation System

## Overview

The ISO USB Hub implements a dynamic power allocation system that distributes available power across three USB ports based on real-time consumption and device connection status. This system ensures optimal power utilization while preventing overcurrent conditions.

## Power Budget Configuration

### Total Power Budget
- **Default**: 100W (configurable via `TOTAL_POWER_BUDGET` environment variable)
- **Configuration**: Set in `.cargo/config.toml` or environment variables
- **Runtime**: Program rebuilds automatically when environment variables change

### Port Specifications
- **Port 1**: SW2303 PD controller (up to 65W, negotiated power)
- **Port 2**: TPS25810 controller (5V, up to 3A/15W)
- **Port 3**: TPS25810 controller (5V, up to 1.5A/7.5W)

## Dynamic Allocation Algorithm

### Core Logic
The system uses real-time power consumption from Port 1 (P1_real) to calculate limits:

```
If Pt - P1_real > 25W:
  P1_limit = Pt - 25W
  P23_limit = 25W (hardware), display as 25W

If Pt - P1_real <= 25W:
  P1_limit = Pt - 25W (forced limit)
  P23_limit = 25W (hardware), display as 15W (conservative)
```

Where:
- `Pt` = Total power budget (default 100W)
- `P1_real` = Real-time power consumption from Port 1
- `P1_limit` = Power limit for Port 1
- `P23_limit` = Combined power limit for Ports 2&3

### Port 2&3 Distribution
When P23 has available power:
- **Port 2**: Gets priority, up to 15W (3A)
- **Port 3**: Gets remaining power, up to 7.5W (1.5A)
- **Minimum**: Each port reserves 10W for basic operation

## Display Modes

### Power Mode (Default)
Shows real-time measurements:
- **Row 1**: Voltage (V) - Yellow color
- **Row 2**: Current (A) - Red color  
- **Row 3**: Power consumption (W) - Green color, or "OCP" if overcurrent

### Power Allocation Mode
Shows calculated limits (press DOWN to toggle):
- **Row 1**: Voltage (V) - Yellow color
- **Row 2**: Current (A) - Red color
- **Row 3**: Power limits (W) - White color
  - **Port 1**: Power limit in Watts
  - **Ports 2&3**: Power limits in Watts (converted from current limits)

## Implementation Details

### Hardware Configuration
```rust
// Apply calculated power allocation to hardware
pub async fn apply_power_allocation(&mut self, allocation: [f32; 3]) -> Result<(), HardwareError> {
    // Configure SW2303 for Port 1
    self.configure_sw2303_power(allocation[0]).await?;
    
    // Configure TPS25810 controllers for Ports 2&3
    self.configure_tps25810_current(2, allocation[1] / 5.0).await?; // Convert W to A
    self.configure_tps25810_current(3, allocation[2] / 5.0).await?;
    
    Ok(())
}
```

### Real-time Monitoring
- **INA226 sensors**: Monitor actual power consumption on all ports
- **Update frequency**: 100ms for responsive allocation adjustments
- **Connection detection**: SW2303 DEVICE_ONLINE flag for Port 1, TCA6424 for Ports 2&3

### Safety Features
- **Overcurrent protection**: Hardware-level protection via TPS25810 controllers
- **Conservative limits**: Display shows 15W for P23 when total budget is constrained
- **Minimum allocation**: 10W reserved per port for stable operation

## Usage Examples

### Scenario 1: No Devices Connected
```
Total Budget: 100W
Port 1: 75W limit (no device, default allocation)
Port 2: 15W limit (3A capability)
Port 3: 7.5W limit (1.5A capability)
```

### Scenario 2: Port 1 High Power Device (45W actual)
```
Total Budget: 100W
Port 1: 75W limit (45W actual consumption)
Port 2: 15W limit (remaining budget allows full allocation)
Port 3: 7.5W limit
```

### Scenario 3: Port 1 Very High Power (80W actual)
```
Total Budget: 100W
Port 1: 75W limit (80W actual, but limited by allocation)
Port 2: 15W limit (displayed as 15W conservative)
Port 3: 7.5W limit (displayed as 15W conservative)
Hardware P23 limit: 25W actual
```

## Benefits

1. **Dynamic Optimization**: Adjusts allocation based on real-time usage
2. **Safety First**: Hardware-enforced limits prevent damage
3. **User Visibility**: Clear display of both consumption and limits
4. **Flexible Configuration**: Environment variable control of total budget
5. **Conservative Display**: Shows safe limits to users while maintaining hardware protection

## Technical Notes

### Power Calculation Accuracy
- **INA226 precision**: 16-bit ADC with configurable averaging
- **Update rate**: 100ms for responsive allocation
- **Filtering**: Moving average to reduce noise in power readings

### Hardware Limits
- **SW2303**: Supports up to 65W PD negotiation
- **TPS25810**: 5V fixed output, current-limited operation
- **Total system**: Limited by input power supply capacity

## Future Enhancements

1. **Predictive allocation**: Learn device power patterns
2. **Priority settings**: User-configurable port priorities
3. **Power scheduling**: Time-based allocation profiles
4. **Remote monitoring**: Network-based power management interface
