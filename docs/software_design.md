# 软件设计文档

本文件为项目的软件设计总文档，收录各子模块的软件规范、调度建议与统一的日志风格。新增或调整任何模块设计，须更新本文件保持单一事实来源。

## 0. 全局约定

### 0.1 全局目标状态

- 定义全局读写锁变量：`PWR_SW_TARGET`，枚举：`Open` / `Closed`。
  - 类型建议（实现细节示例，不具强制性）：`static RwLock<PowerSwitchTarget>`。
  - 语义：目标状态（意图），不等同于实际导通状态。

### 0.2 日志风格（defmt）

- 统一使用 defmt，单行键值对风格，便于机读与筛选。
- 模块前缀使用短名加域，如：`pwr.in`（电源输入）。
- SI 单位与后缀：电压 `V`、电流 `A` 等。
- 级别：
  - 周期/状态与条件变化：`info`；
  - 读数失败：`warn`；
  - 明显非法/越界：`error`。

### 0.3 调度与并发（embassy 建议）

- 使用 `embassy-time` 为周期任务建两个独立异步任务；共享测量数据与目标态使用 `RwLock/Mutex` 保护；
- 避免死锁：按“先读目标态、后读测量”的固定顺序获取锁。

---

## 1. 基础电源输入子系统

本节定义基础电源输入子系统的软件行为、数据源、周期性任务、判定逻辑与日志格式，确保行为可重复且无二义性。

### 1.1 硬件与信号约定

- `INA226`（I2C）：读取输入母线电压/电流。
  - 电压：`VBUS`（单位：V）。
  - 电流：`CURRENT` 或由分流电阻计算（单位：A）。
- `VIN_ADC`（ESP32-S3 `GPIO4 / ADC1_CH3`）：输入电压分压采样。
  - 分压比：11:1（100kΩ+10kΩ），母线电压换算：`V_in_from_adc = VIN_ADC * 11`。
- `IN_PG`（ESP32-S3 `GPIO42`，源自 TPS2490 `PG` 引脚）：
  - 极性：高电平表示 Power-Good（良好），低电平表示非良好/故障；
  - 类型：开漏输出（需上拉）；
  - 依据：TI TPS2490 数据手册（Power Good Open-Drain Output）。

### 1.2 判定与周期任务

#### 1.2.1 100 ms 资格判定

- 触发条件：`PWR_SW_TARGET == Closed` 时执行；周期 `100 ms`。
- 计算：
  - `range_ok`：`9.0 V ≤ INA226.VBUS ≤ 24.0 V`；
  - `vin_adc_low`：`VIN_ADC < 2.0 V`（ADC 端，未换算）；
  - `ok_to_close = range_ok && vin_adc_low`。
- 变化日志：仅当上述任一布尔量或原始量（`INA226.VBUS`、`VIN_ADC`）的判定结果发生变化时，打印一行 `info`：
  - 示例：`pwr.in:chg ok_to_close=true range_ok=true vin_adc_low=true vbus=12.1V vin_adc=0.83V`。
- 说明：该判定仅提供资格信息，不直接驱动硬件导通动作。

#### 1.2.2 10 s 状态汇报

- 周期：`10 s`，无条件输出一行状态：
  - 字段：`vin`（INA226.VBUS, V）、`i`（INA226.CURRENT, A）、`sw`（PWR_SW_TARGET: open/closed）、`pg`（good/bad，由 IN_PG）、`ocp`（true/false）、`note`（可空）。
- 判定：
  - `pg`：IN_PG 高为 `good`，低为 `bad`；
  - `ocp`：当 `PWR_SW_TARGET == Closed` 且 `pg == bad`，判定可能触发过流/保护；
  - 异常：若 `PWR_SW_TARGET == Closed` 且 `pg == good`，但 `V_in_from_adc` 明显低于 `INA226.VBUS`，追加 `note` 告警：
    - 比值阈值：`V_in_from_adc / INA226.VBUS < 0.60`；
    - 或差值阈值：`INA226.VBUS - V_in_from_adc > 3.0 V`；
    - 示例：`note="anom: vin_adc<<ina_v (adc=4.2V, ina=12.1V)"`。
- 示例：`pwr.in:stat vin=12.1V i=0.46A sw=closed pg=good ocp=false`。

### 1.3 边界与错误处理

- `INA226`/`VIN_ADC` 读数失败：本周期忽略数值、输出一次 `warn`，维持上次有效判定；
- 越界保护：若 `INA226.VBUS < 0V` 或 `> 80V`（量程外）视为无效，输出 `error`。

### 1.4 精度与阈值

- 电压范围阈值固定为 9.0 V 与 24.0 V；
- `VIN_ADC` 判定阈值固定为 2.0 V（ADC 端）；
- 异常比值/差值阈值如上；修改阈值须同步更新本文档。

### 1.5 调度与并发（实现建议）

- `InputQualificationTask`（100 ms）与 `InputStatusTask`（10 s）两个异步任务；
- 数据共享用 `RwLock/Mutex`；获取顺序：目标态 → 测量值。

### 1.6 观测与验证要点

- 目标从 `Open→Closed` 且电压合规、`VIN_ADC < 2.0 V`，应出现一次 `ok_to_close=true` 的变化日志；
- 正常供电下，每 10 s 一行状态，`pg=good`、`ocp=false`；
- 保护/异常时，`pg=bad` 或 `note` 告警出现。
