# 软件设计文档

本文件为项目的软件设计总文档，收录各子模块的软件规范、调度建议与统一的日志风格。新增或调整任何模块设计，须更新本文件保持单一事实来源。

## 0. 全局约定

### 0.1 全局目标状态

- 定义全局读写锁变量：`PWR_SW_TARGET`，枚举：`Open` / `Closed`。
  - 类型建议（实现细节示例，不具强制性）：`static RwLock<PowerSwitchTarget>`。
  - 语义：目标状态（意图），不等同于实际导通状态。

### 0.2 日志风格

- 统一使用分级日志（`log` → `esp-println`），单行键值对风格，便于机读与筛选。
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

## 1. 固件初始化流程（Boot Init）

说明：初始化阶段在主任务启动后、其他周期性任务（如电源输入资格/状态任务）之前一次性执行。I2C 总线为共享资源，初始化流程需串行访问，避免并发访问。

### 1.1 基础初始化（时钟/日志/时间源/I2C）

- 时钟与外设：调用 `esp_hal::init(esp_hal::Config::default())` 初始化外设；用 `esp_hal_embassy::init()` 绑定 `embassy-time` 时间源。
- 日志：沿用“0.2 日志风格（defmt）”；初始化时采用精简单行键值对输出，便于筛查启动问题。
- USB 通道切换默认态（CH442E）：
  - `UCM_DIN`（`GPIO33`）由外部下拉维持低电平，默认路由到 MCU 通道；
  - `UCM_DCE`（`GPIO34`）由外部下拉维持低电平（`EN#` 低有效，默认使能切换器）；
  - 固件启动阶段暂不主动改写这两个引脚，待更高层路由策略接管后再切换。
- I2C（共享总线）：由 `esp-hal` 提供单实例 I2C，并通过 `Mutex` + `StaticCell` 共享给上行器件与 PCA9545A 通道视图。

初始化期建议日志（示例）：

- `init.hw: chip=ESP32-S3 i2c=ok sda=GPIOx scl=GPIOy`。
- `init.time: embassy-timer=ok`。

### 1.2 I2C 多路开关 PCA9545A（0x70）

- 使用 crate：`xca9548a`（支持 T/PCA954xA 全系），类型：`Xca9545a`。
  - 参考：docs.rs/xca9548a（已覆盖 PCA9545A）。
- 地址：缺省 `0x70`（A2/A1/A0=0）。
- 初始化步骤：
  - 在上行 I2C 上构造 `Xca9545a` 并执行一次寄存器读取/写入以确认 ACK。
  - 调用 `.split()` 或等价通道视图获取四个下行通道的虚拟 I2C，用于模块侧后续 bring-up。
- 成功日志：`i2c.mux: ok addr=0x70 parts=4`；失败日志并 `panic!`：`i2c.mux: err=...`。

### 1.3 上电与扫描时序（强制）

1) 发布电源输入上电意图（希望 VIN 上电）：
   - 将全局目标态 `PWR_SW_TARGET=Closed`（仅表达意图，不等同于实际导通）。
   - 实际导通由“基础电源输入子系统”的资格判定任务依据 `INA226` 电压/电流与 `IN_PG` 决定与等待（详见“2. 基础电源输入子系统”）。

2) 发布上电意图后，立即扫描前面板 `TCA6408A` 是否在线（存在性确认即可）：
   - 该步骤不依赖 VIN 是否已达标，作为独立可并行的存在性检查提前进行；
   - 成功与否仅记录日志，不阻塞后续 VIN 判定。

3) 等待 VIN 上电确认后，再进入模块侧流程：
   - VIN 上电定义：由 `INA226.VBUS` 达到合规范围且 `IN_PG=good`（见第 2 章阈值），二者综合判定；
   - 在确认 VIN 上电后，才允许访问 PCA9545A 下行各通道；
   - 日志：
     - `pwr.in:vin_on=true vin=..V pg=good`（确认上电后打印一次）；
     - 若超时未达标：`pwr.in:vin_on=false vin=..V pg=good|bad`；
     - 模块扫描开始前：`i2c.scan:start vin_on=true backend=ip6557`。

4) 当前 V3 项目口径只保留 IP6557 子板：
   - 每路子板的已确认器件为 `IP6557 + INA226 + TMP112`；
   - 当前固件分支尚未实现该子板的专用寄存器初始化/遥测；
   - 在 dedicated bring-up 任务落地前，模块侧流程至少保留 `PCA9545A` 的通道选择/读回探测；`EN1..EN4` 保持低电平，不再尝试历史子板方案的探测流程；
   - 只有在 `vin_on == true` 且 mux 探测成功后，UI 才显示 `bringup-pending`；若输入电源未就绪，主循环必须立即显示显式的电源阻断状态，而不是等待第一次 VIN 信号；
   - `vin_on` 必须按实时状态驱动 UI，不能把第一次等待结果缓存成永久状态；mux 探测失败也必须允许后续重试，并在 VIN 掉电/恢复后重新确认各通道可达性。

实现参考的伪代码（仅示意，非约束）：

```text
set(PWR_SW_TARGET, Closed)              // 发布上电意图
spawn(scan_front_panel_tca6408a)        // 立即扫描前面板存在性

wait_until(vin_ok_by_ina226_and_pg())   // 基于 INA226 + PG 的资格确认

for ch in mux.channels():               // VIN 确认后再处理模块侧
    select_mux_channel(ch)
    log_info("pwr.mod", ch, backend="ip6557", init="deferred")
    keep_en_low(ch)
```

### 1.4 设备发现/初始化（四路模块：IP6557 子板）

- 当前项目文档只维护 IP6557 子板口径，不再混入历史子板方案描述。
- 当前分支对四路模块的行为：
  1) 通过 PCA9545A 对对应通道做最小化选择/读回探测；
  2) 若 `vin_on == true` 且 mux 探测成功，则记录模块侧 bring-up 尚未完成：`pwr.mod: ch=X backend=ip6557 init=deferred reason="bringup-pending"`；
  3) 若 `vin_on == false`，记录 `power_blocked=true reason="vin-not-ready"`，UI 不得伪装成模块已进入 bring-up 阶段；
  4) 保持 `ENx` 为低电平，防止在驱动与保护策略未完成前误上电；
  5) 只有 mux 探测连续失败达到阈值时，才把该通道映射为 `Disconnected/Error`；后续轮询仍应允许重新探测恢复，已进入 `bringup-pending` 的通道也要周期性轻量复检。
- dedicated bring-up 任务应覆盖：
  - IP6557 输出策略与保护寄存器初始化；
  - 子板 INA226/TMP112 的地址确认、遥测采样与异常告警；
  - 子板 `INT` 事件与主板策略联动。

### 1.5 前面板 TCA6408ARGTR（0x21）发现与存在性确认

- 使用 crate：`port-expander`，类型：`Tca6408a`（支持阻塞 embedded-hal）。
- 地址：`0x21`（ADDR=1，接 3V3）。
- 说明：主板 U43 的 TCA6408A 地址为 `0x20`（`PWREN#/OVCUR#`），不在本节“前面板存在性检查”流程内。
- 步骤：在上行 I2C 直接构造 `Tca6408a`，读取输入寄存器以确认 ACK。扫描时机：
  - 于“发布电源输入上电意图”之后立即进行；
  - 成功/失败仅记录日志，不阻塞后续 VIN 判定与模块流程。
- 成功日志：`i2c.front: tca6408a=online addr=0x21`；失败：`i2c.front: tca6408a=offline addr=0x21`（仅报告，不 `panic!`）。

### 1.6 设备初始化（仅对已具备稳定口径的设备执行）

初始化失败策略：打印错误日志并 `panic!`（除“不可达/不存在”的发现阶段外）。每个设备初始化成功都需打印一行成功日志。

- 前面板 TCA6408A（存在时才初始化）：
  - 方向配置：将五向开关对应引脚配置为输入（其余引脚按硬件用途配置为输入/输出）；
  - 清中断（若接入 INT）：读取输入寄存器一次；
  - 成功日志：`front.gpio: tca6408a init=ok`；失败则 `panic!`。
- 四路 IP6557 子板：
  - 当前任务内不执行专用初始化；
  - 固件必须保持 `EN1..EN4` 为低电平，并通过日志明确 `bringup-pending`；
  - 若后续 dedicated bring-up 落地，应在新的规格中单独定义初始化顺序与验收日志。

### 1.7 运行期资源交接（可选）

- 若运行期采用 `embedded-hal-async`：在完成上述初始化后，可继续复用共享上行 I2C；
- 通过 `embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice` 共享上行 I2C；
- 模块侧 dedicated bring-up 落地后，应避免通道切换与并发任务竞争，并在驱动层封装清晰的通道独占策略。

### 1.8 失败与异常处理约定

- 器件“不可达/不存在”（发现阶段）：仅打印报告，不 `panic!`；
- 器件“初始化失败”（初始化阶段）：打印 `error` 并 `panic!`；
- 模块侧 dedicated bring-up 尚未落地：打印 `info/warn` 明确说明 `bringup-pending`，而不是尝试旧方案探测；
- 统一错误日志示例：`init.err: comp=tca6408a op=configure code=I2cNack detail="..."`。

### 1.9 参考 crate 与接口（事实依据）

- PCA9545A：`xca9548a`（`Xca9545a`，基于 embedded-hal v1，阻塞接口；支持 `.split()` 提供 4 路虚拟 I2C）。
- TCA6408A：`port-expander`（`Tca6408a`，基于 embedded-hal v1，阻塞接口）。
- 子板 dedicated bring-up：当前仓库尚未冻结 IP6557/INA226/TMP112 的统一驱动层接口；完成选型后需在新任务中登记 crate/API 与初始化顺序。

---

## 2. 基础电源输入子系统

本节定义基础电源输入子系统的软件行为、数据源、周期性任务、判定逻辑与日志格式，确保行为可重复且无二义性。

### 2.1 硬件与信号约定

- `INA226`（I2C）：读取输入母线电压/电流。
  - 电压：`VBUS`（单位：V）。
  - 电流：`CURRENT` 或由分流电阻计算（单位：A）。
- `VIN_ADC`（ESP32-S3 `GPIO4 / ADC1_CH3`）：输入电压分压采样。
  - 分压比：11:1（100kΩ+10kΩ），母线电压换算：`V_in_from_adc = VIN_ADC * 11`。
- `IN_PG`（ESP32-S3 `GPIO42`，源自 TPS2490 `PG` 引脚）：
  - 极性：高电平表示 Power-Good（良好），低电平表示非良好/故障；
  - 类型：开漏输出（需上拉）；
  - 依据：TI TPS2490 数据手册（Power Good Open-Drain Output）。

### 2.2 判定与周期任务

#### 2.2.1 启动期资格判定（基于 INA226）

> 开发阶段特别说明（当前分支生效）：
>
> - 为便于在实验室以 5V 台式电源/USB 供电对风扇等功能做独立验证，固件将 VIN 下限阈值临时放宽为 **4.5 V**（`VIN_MIN_V=4.5`）。
> - 该设置会绕过原本的欠压防护，可能导致在 5–8 V 区间触发上电流程。仅限开发/联调使用，量产前请恢复到 **9.0 V** 并完成回归验证。

- 触发条件：上电启动阶段、闭合 `IN_EN` 之前执行，短暂重试（~5 次，20 ms 间隔）。
- 计算：
  - 生产阈值（规范）：`9.0 V ≤ INA226.VBUS ≤ 24.0 V`；
  - 当前固件（开发分支）：`4.5 V ≤ INA226.VBUS ≤ 24.0 V`；
  - `current_ok`：`|INA226.CURRENT| ≤ 10 mA`；
  - `ok_to_close = range_ok && current_ok`。
- 日志：每次读取打印一行 `info`：`pwr.in:qual vbus=..V i=..A range_ok= current_ok=`。
- 成功后：闭合 `IN_EN`，并将本次 `VBUS/CURRENT` 写入共享测量，避免状态上报出现 `vin=n/a`。
- `VIN_ADC` 不参与资格判定；仅在 `PG` 超时（100 ms）时单次读取用于诊断。

#### 2.2.2 10 s 状态汇报

- 周期：`10 s`，无条件输出一行状态：
  - 字段：
    - `vin`（INA226.VBUS, V）
    - `i`（INA226.CURRENT, A）
    - `sw_intent`（on/off，来自 `PWR_SW_TARGET`，表示期望状态）
    - `sw_actual`（on/off，由 `IN_PG` 推导：PG=good→on，PG=bad→off）
    - `pg`（good/bad，由 IN_PG）
    - `note`（可空）
- 判定：
  - `pg`：IN_PG 高为 `good`，低为 `bad`；
  - 异常：若 `PWR_SW_TARGET == Closed` 且 `pg == good`，但 `V_in_from_adc` 明显低于 `INA226.VBUS`，追加 `note` 告警：
    - 比值阈值：`V_in_from_adc / INA226.VBUS < 0.60`；
    - 或差值阈值：`INA226.VBUS - V_in_from_adc > 3.0 V`；
    - 示例：`note="anom: vin_adc<<ina_v (adc=4.2V, ina=12.1V)"`。
- 示例：`pwr.in:stat vin=12.1V i=0.46A sw_intent=on sw_actual=on pg=good`。

### 2.3 边界与错误处理

- `INA226`/`VIN_ADC` 读数失败：本周期忽略数值、输出一次 `warn`，维持上次有效判定；
- 越界保护：若 `INA226.VBUS < 0V` 或 `> 80V`（量程外）视为无效，输出 `error`。

### 2.4 精度与阈值

- 生产电压范围阈值：9.0 V 与 24.0 V；
- 开发阶段（当前分支）临时阈值：`VIN_MIN_V = 4.5 V`（为 5V 台架测试放宽）；
  - `VIN_ADC` 判定阈值固定为 2.0 V（ADC 端）；
  - 异常比值/差值阈值如上；修改阈值须同步更新本文档。

### 2.5 调度与并发（实现建议）

- `InputQualificationTask`（100 ms）与 `InputStatusTask`（10 s）两个异步任务；
- 数据共享用 `RwLock/Mutex`；获取顺序：目标态 → 测量值。

### 2.6 观测与验证要点

- 目标从 `Open→Closed` 且电压合规、`VIN_ADC < 2.0 V`，应出现一次 `ok_to_close=true` 的变化日志；
- 正常供电下，每 10 s 一行状态，`pg=good`、`ocp=false`；
- 保护/异常时，`pg=bad` 或 `note` 告警出现。
