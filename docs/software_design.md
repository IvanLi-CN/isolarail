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

说明：初始化阶段在主任务启动后、其他周期性任务（如电源输入资格/状态任务）之前一次性执行。I²C 总线为共享资源，初始化流程需串行访问，避免并发访问。

### 1.1 启动阶段与自检状态机

- 启动流程分为 `Early Bring-up -> Self-Check -> Gate Apply -> Runtime` 四阶段，由统一的 boot self-check 快照驱动。
- `Self-Check` 阶段同时承担三件事：
  - 输出 `boot.stage:*`、`boot.check:*`、`boot.summary:*` 日志；
  - 驱动 160x50 LCD 的启动自检页；
  - 产出 `GateDecision`，决定是否放行 runtime task、front panel 和各路端口。
- 统一状态口径：
  - `SelfCheckItemState = Pending / Ok / Warn / Err / Fatal / Skipped`
  - `BootOutcome = Ok / Degraded / Fatal`
  - `BootFaultCode` 覆盖 `MuxOffline`、`PowerInUnavailable`、`PowerInPgBad`、`InaUnavailable`、`FrontPanelOffline`、`FanUnavailable` 以及按端口编号的配对/保护/超时故障。
- 默认策略是“只读探测 + 分级降级”：
  - 缺模块、单路异常、front panel 离线都优先降级；
  - 只有输入电源不安全、或明确无法保证通道安全关闭时才进入 `Fatal`。

### 1.2 基础初始化（时钟/日志/时间源/I²C/LCD）

- 时钟与外设：调用 `esp_hal::init(esp_hal::Config::default())` 初始化外设；用 `esp_hal_embassy::init()` 绑定 `embassy-time` 时间源。
- 日志：沿用“0.2 日志风格（defmt）”；启动关键路径必须保持单行键值对输出，便于错过串口首帧时仍能从后续摘要定位问题。
- I²C（共享总线）：单实例 I²C 由 `esp-hal` 提供，使用 `embassy_sync::Mutex` + `static_cell::StaticCell` 构建共享访问；启动自检与运行期任务都通过同一总线序列化访问。
- LCD：显示初始化成功后，先进入 boot self-check 页，不再把 dashboard 首帧当作“系统一定正常”的信号。

初始化期建议日志（示例）：

- `app.start`
- `init.time: embassy-timer=ok`
- `boot.stage: stage=self-check`

### 1.3 自检顺序与门控顺序（强制）

boot self-check 采用固定顺序，避免不同模块各自抢总线与各说各话：

1) 完成日志、时间源、LCD、共享 I²C 与基础 GPIO 的早期 bring-up；
2) 确认当前 I²C 拓扑：
   - 当前工程验证板为直连共享总线，记录 `boot.check: name=mux state=skip fault=-`；
   - 若后续硬件版本恢复 `PCA9545A`，再切回真实 MUX 探测与门控。
3) 启动并等待基础电源输入资格结果：
   - `INA226` 初始化失败、VIN 不可用或 `PG` 不良都由 `power_in::bootstrap_signal()` 返回；
   - 若输入资格失败，`IN_EN` 必须保持关闭，runtime 与端口初始化均不放行。
4) 在 VIN ready 时探测前面板 `TCA6408A` 与风扇链路：
   - front panel 离线记为 `Warn/FrontPanelOffline`，仅关闭面板功能；
   - 风扇链路初始化失败记为 `Warn/FanUnavailable`，不阻断 dashboard。
5) 仅在 `VIN ready` 时进入四路端口扫描：
   - 每路检查 V3 输出模块金手指下挂的 `INA226/TMP112` 是否都可达；
   - 当前验证基线下，通道 4 的正式地址组合为 `INA226(0x43)` + `TMP112(0x4B)`；
   - 单路失败只记录该路 `Err`，不再阻断该路 `ENx` 放行。
6) 汇总 `GateDecision`，输出 `boot.summary:*`：
   - `BootOutcome=Fatal`：LCD 常驻自检页；
   - `BootOutcome=Degraded`：短暂展示摘要后进入 dashboard；
   - `BootOutcome=Ok`：直接进入 runtime。

### 1.4 I²C 多路开关 PCA9545A（0x70）

- 当前工程验证板未接入 `PCA9545A`，固件按“直连共享总线”运行，并在启动日志中输出：
  - `i2c.topo: direct shared bus; mux probe skipped`
  - `boot.check: name=mux state=skip fault=-`
- 兼容策略：
  - boot self-check 快照继续保留 `mux` 槽位，用于兼容下个硬件版本恢复 `PCA9545A`；
  - 当前版本 `mux=Skipped` 不参与 fatal/degraded 判定，也不阻断端口扫描；
  - 后续若恢复 `PCA9545A`，应在本节恢复真实探测、故障码与门控要求。

### 1.5 四路输出模块发现与初始化（V3: INA226 + TMP112）

- V3 输出模块的金手指暴露 `SDA/SCL/INT/EN`，模块网表可见两颗 I2C 器件：`INA226` 与 `TMP112`。
- 当前验证基线的正式地址表：
  - 通道 1：`INA226(0x40)` + `TMP112(0x48)`
  - 通道 2：`INA226(0x41)` + `TMP112(0x49)`
  - 通道 3：`INA226(0x42)` + `TMP112(0x4A)`
  - 通道 4：`INA226(0x43)` + `TMP112(0x4B)`
- 逐通道初始化策略（串行）：
  1) 在共享 I²C 上对该路地址对执行只读 ACK/寄存器探测；
  2) 两颗都在线则将通道标记为 `Ok`；
  3) 任一探测失败则将通道标记为 `Err`，但不改变“总输入 OK 后统一放行输出”的总门控策略。
- 结果与故障码约定：
  - `INA226 + TMP112` 都成功：端口记为 `Ok`，允许该路 dashboard/runtime 读取；
  - 两者都离线：`Err/PortModuleOffline(ch)`；
  - 仅 `INA226` 离线：`Err/PortInaOffline(ch)`；
  - 仅 `TMP112` 离线：`Err/PortTempOffline(ch)`。
- 安全要求：
  - 单路异常不会阻断其它通道扫描；
  - 只要总输入资格为 `OK`，自检结束后统一拉高 `EN1..EN4`；
  - 单路 `Err` 仅影响诊断与运行期该路测量读取，不再单独关断该路输出。
  - 端口传感器缺失按 degraded 处理，不单独提升整机为 fatal。

### 1.6 前面板与风扇链路

- 前面板 `TCA6408A` 地址为 `0x21`，仅在输入电源 ready 后探测。
- 成功日志：`i2c.front: tca6408a=online addr=0x21`
- 失败日志：`i2c.front: tca6408a=offline addr=0x21`
- 门控要求：
  - `FrontPanelOffline` 仅设置 `allow_front_panel=false`，不阻断 dashboard 与其它链路；
  - 风扇链路初始化失败仅记录 `Warn/FanUnavailable`，不将整机提升为 fatal。

### 1.7 启动失败与异常处理约定

- 器件“不可达/不存在”优先记录为 `Warn / Err / Skipped`，不允许直接因为单个探测失败而 `panic!`；
- 输入电源资格失败时：
  - `IN_EN` 必须保持关闭；
  - 四路端口全部 `Skipped`；
  - `GateDecision.allow_runtime_tasks=false`；
  - LCD 停留在 fatal 自检页。
- 通道传感器缺失（`INA226/TMP112` 不齐）：
  - 打印 `warn`；
  - 标记对应通道 `Err`；
  - 继续扫描其它通道，并在总输入 `OK` 时保持统一放行输出。
- 统一摘要日志示例：
  - `boot.check: name=vin state=FATAL fault=PG BAD ...`
  - `boot.summary: outcome=DEG first_fault=FAN runtime=on front_panel=on`

### 1.8 运行期资源交接

- 运行期继续复用共享 async I²C；
- 只有 `GateDecision.allow_runtime_tasks=true` 时，才允许进入 dashboard 与周期刷新；
- 只有 `GateDecision.allow_front_panel=true` 时，才允许启动前面板任务；
- `BootSelfCheckSnapshot` 是日志、自检页与门控决策的单一事实来源。

### 1.9 参考 crate 与接口（事实依据）

- PCA9545A：未来恢复该硬件版本时再接回 `xca9548a` / 等价驱动；当前直连共享总线版本未启用。
- TCA6408A：`port-expander`（`Tca6408a`，基于 embedded‑hal v1，阻塞接口）。
- INA226：`ina226-tp`（async 驱动，既用于输入电源链路，也用于 V3 输出模块运行期读数）。

以上 crate 均可通过 GitHub/Docs.rs 查验 API 与用法；若后续替换为 crates.io 正式版本，请在 `Cargo.toml` 中同步更新来源与版本并在变更说明中登记。

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
