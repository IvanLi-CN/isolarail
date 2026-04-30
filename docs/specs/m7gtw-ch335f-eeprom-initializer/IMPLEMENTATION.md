# CH335F EEPROM 初始化固件实现状态

## 当前状态

- 新增独立 bin `ch335f_eeprom_init`。
- 固件使用 `GPIO5` 控制 CH335F reset，使用 `GPIO36/GPIO37` 访问 M24C64 `0x50`。
- 固件生成 CH334/CH335 外部 EEPROM 镜像，Product String 为 `ISO USB Hub`，legacy-compatible Vendor String 为 `Ivan`，VID/PID 保持 `0x1A86:0x8094`。
- 固件会先扫描 `0x50..0x57`；若正常 `GPIO36=SDA/GPIO37=SCL` 无 ACK，会在同一对物理引脚上尝试一次 `GPIO37=SDA/GPIO36=SCL` 诊断。
- 写入策略为读比对后写入差异页，完成后全量读回校验。
- 未发现 EEPROM ACK 时不写入，并释放 `HUB_RESET#` 让 CH335F 回到默认枚举。
- I2C 外设释放后，`GPIO36/GPIO37` 保持无内部上下拉的输入状态。
- 实机结论是当前 0 Ω 并联 EEPROM 拓扑不可作为可控初始化方案；下一版硬件应使用 CH442E 切换 EEPROM 连接方向。

## Coverage

- EEPROM 前 256 bytes 镜像生成。
- M24C64 16-bit address 读取。
- 32-byte page 写入。
- ACK polling。
- 写后 readback verification。
- CH335F reset 时序。

## Remaining Gaps

- 需要下一版硬件加入 EEPROM 方向切换后，再确认 macOS USB 枚举 Product String 变为 `ISO USB Hub`。
- 不同 CH334/CH335 文档版本对 Vendor String 字段定义不同；host vendor name 仍必须以实际 USB 枚举为准。

## Hardware Validation

- Build: `cargo +esp check` passed.
- Build: `cargo +esp build --release --bin ch335f_eeprom_init` passed.
- Hardware: `/dev/cu.usbmodem212301`, MAC `a0:f2:62:f1:fb:44`, flashed and booted with the outdated `GPIO45/GPIO46` build.
- Firmware log: outdated normal `GPIO45=SDA/GPIO46=SCL` scan found no EEPROM ACK in `0x50..0x57`.
- Firmware log: outdated swapped `GPIO46=SDA/GPIO45=SCL` scan also found no EEPROM ACK in `0x50..0x57`.
- Firmware log: no write was attempted; `HUB_RESET#` was released after the no-ACK diagnostic.
- Hardware: `/dev/cu.usbmodem212301`, MAC `a0:f2:62:f1:fb:44`, flashed and booted with the Rev2.3 `GPIO36/GPIO37` build.
- Firmware log: normal `GPIO36=SDA/GPIO37=SCL` scan found no EEPROM ACK in `0x50..0x57`.
- Firmware log: swapped `GPIO37=SDA/GPIO36=SCL` scan also found no EEPROM ACK in `0x50..0x57`.
- Firmware log: no write was attempted; `HUB_RESET#` was released after the no-ACK diagnostic.
- macOS USB tree after release: target Hub remains `USB HUB`, `idVendor=0x1A86`, `idProduct=0x8094`.
- Later hardware rework made EEPROM `0x50` visible on `GPIO36/GPIO37` while CH335F was not held in reset; firmware log showed `i2c.scan: addr=0x50 sample0=0x86`.
- Firmware log showed EEPROM write/readback success for the target image during that diagnostic state: `eeprom.write: page_offset=0x00 len=32 ok` and `eeprom.verify: readback match`.
- A subsequent run showed `eeprom.compare: match; write skipped`, proving the EEPROM contents matched the generated image.
- macOS USB tree after CH335F reset still showed target Hub as default `USB HUB`, `idVendor=0x1A86`, `idProduct=0x8094`.
- Conclusion: ESP-side EEPROM readback is not sufficient proof of CH335F descriptor consumption on the shared 0 ohm bus, and the mergeable initializer must keep CH335F reset asserted before any write attempt.
- A previous run targeted `/dev/cu.usbmodem2122301`, which was later identified as unrelated hardware and must not be used as evidence for this board.

## Related Changes

- `src/bin/ch335f_eeprom_init.rs`
- `README.md`
- `docs/solutions/hardware/ch335f-eeprom-bus-isolation.md`
