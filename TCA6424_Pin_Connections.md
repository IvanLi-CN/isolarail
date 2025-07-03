# TCA6424 Pin Connection Document

The following is the pin connection information for the TCA6424 chip based on the provided schematic. If the pin function is unclear, it will be left blank.

| Pin No. | Pin Name | Connected Signal Name | Remarks |
|---|---|---|---|
| 1 | P00 | P2_POL | |
| 2 | P01 | P2_UFP | Port 2 UFP Active (Low Active) |
| 3 | P02 | P2_LD_DET | Port 2 Fast Charging Active (Low Active) |
| 4 | P03 | P2_CHG_HL | Port 2 Enable USB-C 3A Current Source (P2_CHG must be High) |
| 5 | P04 | P2_CHG | Port 2 Enable USB-C Current Source (1.5A), 500mA (USB 2.0) or 900mA (USB 3.1) when not enabled |
| 6 | P05 | P2_EN# | Port 2 Enable, High disables port, Low forces enable, not controlled when floating |
| 7 | P06 | P2_FAULT | Port 2 Fault Signal Input, Low indicates fault |
| 8 | P07 | V1OK | Normal signal from the other side of the USB isolator |
| 9 | P10 | P1_DATA_CONN | Port 1 Communication Enable |
| 10 | P11 | P2_DATA_CONN | Port 2 Communication Enable |
| 11 | P12 | P3_DATA_CONN | Port 3 Communication Enable |
| 12 | P13 | HUB_RESET | Reset CH335F, Low active |
| 13 | P14 | P1_FAULT | Port 1 Fault Signal Input, Low indicates fault |
| 14 | P15 | HUB_LED4 |  |
| 15 | P16 | HUB_LED3 |  |
| 16 | P17 | HUB_LED2 |  |
| 17 | P20 | P3_FAULT | Port 3 Fault Signal Input, Low indicates fault |
| 18 | P21 | P3_EN# | Port 3 Enable, High disables port, Low forces enable, not controlled when floating |
| 19 | P22 | P3_CHG | Port 3 Enable USB-C Current Source (1.5A), 500mA (USB 2.0) or 900mA (USB 3.1) when not enabled |
| 20 | P23 | P3_CHG_HL | Port 3 Enable USB-C 3A Current Source (P3_CHG must be High) |
| 21 | P24 | P3_LD_DET | Port 3 Fast Charging Active (Low Active) |
| 22 | P25 | P3_UFP | Port 3 UFP Active (Low Active) |
| 23 | P26 | P3_POL | |
| 24 | P27 | HUB_LED1 |  |
| 25 | GND | GND | Ground |
| 26 | ADDR | | Address pin, connected to ground |
| 27 | VCCP | 3V3 | Core Power |
| 28 | RESET# | 3V3 | Reset pin, pulled up to 3V3 via 10kΩ resistor R56 |
| 29 | SCL | SCL | I2C Clock Line |
| 30 | SDA | SDA | I2C Data Line |
| 31 | VCCI | 3V3 | IO Power |
| 32 | INT# | INT | Interrupt Output |
| 33 | EP | GND | Exposed pad, connected to ground |
