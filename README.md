# Ivan's Isolated USB HUB

3C1A, isolated High Speed USB Hub.

The FreeCAD project file is located at [`models/iso-usb-hub-with-pd.FCStd`](models/iso-usb-hub-with-pd.FCStd).
Exported 3D printable models for the front and back covers are located at:

- [`models/iso-usb-hub-with-pd-Front.step`](models/iso-usb-hub-with-pd-Front.step)
- [`models/iso-usb-hub-with-pd-Back.step`](models/iso-usb-hub-with-pd-Back.step)

## Display Logic for USB Ports

The display on the device shows real-time voltage, current, and power for each USB port. The color of these readings is determined as follows:

### Port 1

For Port 1, the colors of voltage, current, and power readings are dynamically determined based on their measured values:
- **Voltage Color**: Orange if > 6.0V, Yellow if > 2.0V, otherwise Gray.
- **Current Color**: Red if power > 0.05W, otherwise Gray.
- **Power Color**: Blue if > 5.0W, Green if > 0.05W, otherwise Gray.

### Port 2 and Port 3

For Port 2 and Port 3, the colors are primarily determined by the connection status of a sink device, indicated by the `P_UFP` signal from the TCA6424 I/O expander.
- **When a sink device is connected (`P_UFP` is Low Active)**:
  - **Voltage Color**: Yellow
  - **Current Color**: Red
  - **Power Color**: Green
- **When no sink device is connected (`P_UFP` is High Active)**:
  - **Voltage Color**: Gray
  - **Current Color**: Gray
  - **Power Color**: Gray
