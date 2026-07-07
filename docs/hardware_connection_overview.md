# 硬件连接概览（IsolaRail V3）

本文件汇总当前 `isolarail` V3 基线板的关键硬件连接关系，作为 README、控制面 spec 与软件设计文档之间共享的 current truth。owner-facing 设备模型固定为四路 `port1..port4`；历史 V2 `SC8815 + SW2303` / `PSTOP*` 方案不再作为本文件事实源。

## 适用范围

- 当前软件落地基线：`ESP32-S3 + CH335F + M24C64@0x50 + EN1..EN4 + PWREN#/OVCUR# + 160x50 LCD + front panel`
- 端口模型：`Port 1`、`Port 2`、`Port 3`、`Port 4`
- 当前控制面维护动作：
  - `port.replug` = 受控断电再上电
  - `hub.reset` = 整机维护复位动作

## 总体结构

- 主控：`ESP32-S3`
- USB Hub 控制器：`CH335F`
- Wi-Fi / 网络配置 EEPROM：`M24C64@0x50`
- 输入电源热插拔 / 保护：`TPS2490`
- 输入电源监测：`Input INA226@0x44`
- 主板 sideband expander：`Mainboard TCA6408A@0x20`
- 前面板 / LCD 控制 expander：`Front-panel TCA6408A@0x21`
- 预留 / 兼容命名槽位：`PCA9545A@0x70`

## 电源输入（VIN_UNSAFE → VIN）

- 直流输入网络名：`VIN_UNSAFE`
- 主系统母线：`VIN`
- 输入路径：`VIN_UNSAFE -> 5 mΩ shunt -> TPS2490 / 输入开关 -> VIN`
- 关键控制/观测网络：
  - `IN_EN`：MCU 控制输入开关资格
  - `IN_PG`：高电平表示 power-good
  - `VIN_ADC`：MCU ADC 分压采样点
- 输入电源测量器件：
  - `Input INA226@0x44`：输入电压 / 电流 / 功率观测

## 四路输出与端口门控

- 四路 owner-facing 端口由 `EN1..EN4` 门控：
  - `EN1 -> Port 1`
  - `EN2 -> Port 2`
  - `EN3 -> Port 3`
  - `EN4 -> Port 4`
- 当前实现里 MCU 直驱的四路 enable GPIO 为：
  - `EN1 = GPIO17`
  - `EN2 = GPIO18`
  - `EN3 = GPIO39`
  - `EN4 = GPIO40`
- 当前 V3 板的软件语义：
  - `port.power_set` 直接影响对应 `ENx`
  - `port.replug` 通过临时拉低再恢复对应 `ENx` 实现
  - 不承诺真 per-port data disconnect

## CH335F sideband 与上游联动

- `CH335F` 的 sideband 通过 `Mainboard TCA6408A@0x20` 连接到 MCU
- 低有效输入：
  - `PWREN1#`
  - `PWREN2#`
  - `PWREN3#`
  - `PWREN4#`
- 低有效注入：
  - `OVCUR1#`
  - `OVCUR2#`
  - `OVCUR3#`
  - `OVCUR4#`
- 上游隔离状态：
  - `ISOUSB211 V1OK` 为 owner-facing 运行时状态
  - 板级网络名：`ISO_OK`
- `V1OK=low` 时，设备按 standalone/no-upstream 处理
- `V1OK=high` 时，设备按 upstream-managed 模式读取 `PWREN#`

## 两条 I²C 总线

### Sensor / front-panel I²C

- `I2C_SDA = GPIO8`
- `I2C_SCL = GPIO9`
- 当前挂载：
  - `Input INA226@0x44`
  - `Front-panel TCA6408A@0x21`
  - `Port 3 INA226@0x42 + TMP112@0x4A`
  - `Port 4 INA226@0x43 + TMP112@0x4B`

### Hub-sideband / output I²C

- `HUB_SDA = GPIO14`
- `HUB_SCL = GPIO13`
- 当前挂载：
  - `Mainboard TCA6408A@0x20`
  - `Port 1 INA226@0x40 + TMP112@0x48`
  - `Port 2 INA226@0x41 + TMP112@0x49`
  - `M24C64@0x50`

## EEPROM 与 Wi-Fi provisioning

- 配置 EEPROM：`M24C64@0x50`
- 当前固件通过板级路由信号访问该 EEPROM：
  - `ROM_WC = GPIO37`，运行期保持 low 允许写入
  - `ROM_ROUTE = GPIO38`，当前固件设为 high 选择 firmware/EERPOM 路径
- 用途：
  - 保存 Wi-Fi SSID / PSK 与网络配置
  - 作为 USB / LAN / companion / web 统一消费的设备持久化入口

## 每路端口遥测地址

- `Port 1`：`INA226@0x40` + `TMP112@0x48`
- `Port 2`：`INA226@0x41` + `TMP112@0x49`
- `Port 3`：`INA226@0x42` + `TMP112@0x4A`
- `Port 4`：`INA226@0x43` + `TMP112@0x4B`

这些地址用于运行期电压/电流/功率/温度观测，不改变 owner-facing 四路端口语义。

## 前面板与显示

- 前面板 expander：`Front-panel TCA6408A@0x21`
- 五向开关：
  - `P0 = Center`
  - `P1 = Right`
  - `P2 = Down`
  - `P3 = Left`
  - `P4 = Up`
- LCD 控制：
  - `P5 = LCD_RES`
  - `P6 = LCD_CS`
- LCD 直连 MCU 的 SPI / 控制网络：
  - `LCD_DC = GPIO10`
  - `LCD_MOSI = GPIO11`
  - `LCD_SCLK = GPIO12`
  - `LCD_BLK = GPIO15`
- 显示模块：`160x50 LCD`
- 驱动 IC：`GC9D01`
- 背光 `LCD_BLK` 当前为低有效

## Reset / 中断 / 其他板级网络

- `Mainboard RESET# = GPIO35`
- `I2C_INT`：共享 I²C 外设中断汇总输入
- `HUB_RESET#`：CH335F 复位控制网络
- `USB D+` / `USB D-`：ESP32-S3 原生 USB 差分对
- `BUZZER`：蜂鸣器控制网络
- `FAN_PWM` / `FAN_EN` / `FAN_TACH`：风扇控制与反馈网络

## 当前 V3 与历史 V2 的边界

- 本文件只描述当前 V3 current truth
- 下列术语只允许出现在历史/迁移语境，不再代表当前控制面硬件：
  - `SC8815 + SW2303`
  - `PSTOP_CTL1..4`
  - `PSTOP1..4`
  - 历史 `USB-C route` / 双口产品抽象
- `PCA9545A@0x70` 在当前文档中只作为兼容命名槽位保留；当前软件运行基线不是依赖它完成端口寻址

## 简化拓扑示意

```text
DC IN
  └─ VIN_UNSAFE -> shunt -> TPS2490 / input gate -> VIN
                      └─ Input INA226@0x44

ESP32-S3
  ├─ Sensor I2C (GPIO8/GPIO9)
  │   ├─ Front-panel TCA6408A@0x21
  │   ├─ Input INA226@0x44
  │   ├─ Port 3 INA226@0x42 + TMP112@0x4A
  │   └─ Port 4 INA226@0x43 + TMP112@0x4B
  ├─ Hub I2C (GPIO14/GPIO13)
  │   ├─ Mainboard TCA6408A@0x20
  │   ├─ M24C64@0x50
  │   ├─ Port 1 INA226@0x40 + TMP112@0x48
  │   └─ Port 2 INA226@0x41 + TMP112@0x49
  ├─ EN1..EN4 -> Port 1..4 power gate
  ├─ ISO_OK / V1OK <- upstream isolation status
  ├─ USB D+ / D- <-> native USB
  └─ LCD + front panel + buzzer + fan

CH335F
  ├─ PWREN1#..4# -> Mainboard TCA6408A@0x20 -> MCU
  └─ OVCUR1#..4# <- Mainboard TCA6408A@0x20 <- MCU
```

## 参考

- `docs/specs/pw97u-control-plane-alignment/SPEC.md`
- `docs/specs/j6nvw-hardware-v3-pin-assignment/SPEC.md`
- `docs/software_design.md`
