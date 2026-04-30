# CH335F EEPROM 初始化固件历史

## 决策

- 使用独立 bin，而不是合入常规运行固件默认启动路径。
- EEPROM 写入策略采用读比对后写入，降低 EEPROM 擦写次数。
- 首轮验证只修改 Product String，保持 VID/PID 为 `0x1A86:0x8094`。
- CH334/CH335 外部 EEPROM 布局在不同文档版本中对 Vendor String 字段定义不同；固件写入 legacy-compatible `Ivan` 字段，但不承诺 host 一定采用该字段。
- `/dev/cu.usbmodem2122301` 后续被确认为 unrelated hardware；该端口上的失败日志不得作为目标板 EEPROM 访问结论。
- 正确目标更正为 `/dev/cu.usbmodem212301`；该目标 MAC 为 `a0:f2:62:f1:fb:44`。
- 过时项目网表曾显示 `HUB_SDA/HUB_SCL = GPIO45/GPIO46`；该结论已被 Rev2.3 网表替换。
- 正确目标曾在过时 `GPIO45/GPIO46` 正向与反向诊断下均未发现 `0x50..0x57` ACK，因此未执行 EEPROM 写入。
- Rev2.3 网表替换后，正确目标在 `GPIO36/GPIO37` 正向与反向诊断下仍未发现 `0x50..0x57` ACK，因此仍未执行 EEPROM 写入。
- 后续重新焊接后，目标板在 `GPIO36/GPIO37` 上可见 EEPROM `0x50` ACK，且 ESP32-S3 能写入并读回校验 EEPROM 镜像。
- 即使 EEPROM readback 匹配目标镜像，CH335F 重新枚举后仍保持默认 `USB HUB` 字符串，说明 0 Ω 并联 EEPROM 总线无法作为可控初始化方案。
- 下一版硬件采用 CH442E 切换 EEPROM 连接方向：编程模式连接 ESP32-S3，运行模式连接 CH335F。

## 依据

- Rev2.3 网表显示 CH335F `LED3/SCL`、`LED4/SDA` 经 0 Ω 连接至 EEPROM，并经 0 Ω 连接至 ESP32-S3 `GPIO37/GPIO36`。
- V3 网表显示 `SDA_ROM/SCL_ROM` 经 R114/R113 接到 `HUB_LED4/HUB_LED3`，再经 R115/R116 接到 ESP32-S3；`HUB_LED4` 同时连接 LED 支路。
- Rev2.3 网表显示 EEPROM `V_ROM` 经 R60 0 Ω 接 `3V3`，`E0/E1/E2` 接 GND，`WC` 经 R59 上拉到 `3V3`。
- V3 pin assignment 指定 `HUB_RESET#=GPIO5`，可用于在 ESP 写入 EEPROM 时保持 CH335F 复位。
- CH334/CH335 V2.4 数据手册定义外部 EEPROM `00h..0Ah` 配置、Product String 与 Serial Number String 布局。
