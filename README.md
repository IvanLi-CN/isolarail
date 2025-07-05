# Ivan's Isolated USB HUB

3C1A, isolated High Speed USB Hub.

The FreeCAD project file is located at [`models/iso-usb-hub-with-pd.FCStd`](models/iso-usb-hub-with-pd.FCStd).
Exported 3D printable models for the front and back covers are located at:

- [`models/iso-usb-hub-with-pd-Front.step`](models/iso-usb-hub-with-pd-Front.step)
- [`models/iso-usb-hub-with-pd-Back.step`](models/iso-usb-hub-with-pd-Back.step)

## Hardware Architecture

### Port Configuration

- **Port 1**: SW2303 PD controller with USB-C PD support (up to 65W)
- **Port 2**: TPS25810 controller with 5V only
- **Port 3**: TPS25810 controller with 5V only

### Current Sensing Configuration

Each port uses INA226 current sensors with different shunt resistor values:

- **Port 1 (SW2303)**: 5mΩ current sensing resistor
- **Port 2 (TPS25810)**: 10mΩ current sensing resistor
- **Port 3 (TPS25810)**: 10mΩ current sensing resistor

### UFP Detection

- **Port 1**: SW2303 sink device detection via I2C
- **Port 2**: TCA6424 P01 pin (Low Active)
- **Port 3**: TCA6424 P25 pin (Low Active)

## Display Logic for USB Ports

The display on the device shows real-time voltage, current, and power for each USB port. The color of these readings is determined by the connection status of sink devices for all ports:

### All Ports (Port 1, Port 2, and Port 3)

The colors are determined by the connection status of sink devices:

- **Port 1**: Sink device detection via SW2303 PD controller
- **Port 2 and Port 3**: Sink device detection via `P_UFP` signal from TCA6424 I/O expander

**When a sink device is connected**:

- **Voltage Color**: Yellow
- **Current Color**: Red
- **Power Color**: Green

**When no sink device is connected**:

- **Voltage Color**: Gray
- **Current Color**: Gray
- **Power Color**: Gray
