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

### 1.1 基础初始化（时钟/日志/时间源/I²C）

- 时钟与外设：调用 `esp_hal::init(esp_hal::Config::default())` 初始化外设；用 `esp_hal_embassy::init()` 绑定 `embassy-time` 时间源。
- 日志：沿用“0.2 日志风格（defmt）”；初始化时采用精简单行键值对输出，便于筛查启动问题。
- I²C（共享总线）：单实例 I²C 由 `esp-hal` 提供。参考示例（ESP32‑C3）`/Users/ivan/Projects/Ivan/esp32c3-sc8815-sw2303-demo/src/main.rs:61-69` 的做法，使用 `embassy_sync::Mutex` + `static_cell::StaticCell` 构建共享访问，再视具体驱动选择阻塞或异步封装：
  - 阶段一（初始化阶段）：优先使用“阻塞版 embedded-hal”驱动完成器件发现与一次性配置，避免 `async`/`blocking` 混用带来的 trait 兼容性问题。
  - 阶段二（运行阶段）：如需异步访问，再将 I²C 迁移为 `embedded-hal-async` 使用（丢弃初始化阶段的临时驱动实例，重新以 async 句柄构造运行期驱动）。

初始化期建议日志（示例）：

- `init.hw: chip=ESP32-S3 i2c=ok sda=GPIOx scl=GPIOy`。
- `init.time: embassy-timer=ok`。

### 1.2 I²C 多路开关 PCA9545A（0x70）

- 使用 crate：`xca9548a`（支持 T/PCA954xA 全系），类型：`Xca9545a`。
  - 参考：docs.rs/xca9548a（已覆盖 PCA9545A）。
- 地址：缺省 `0x70`（A2/A1/A0=0）。
- 初始化步骤：
  - 在上行 I²C 上构造 `Xca9545a` 并执行一次寄存器读取/写入以确认 ACK。
  - 调用 `.split()` 获取四个下行通道的虚拟 I²C（`i2c0..i2c3`）；用于各通道扫描与后续设备初始化时的通道自动选择。
- 成功日志：`i2c.mux: ok addr=0x70 parts=4`；失败日志并 `panic!`：`i2c.mux: err=...`。

### 1.3 上电与扫描时序（强制）

本小节对“上电与外设扫描”的业务流程作出明确、不可二义的顺序要求，用于修正并统一实现行为：

1) 发布电源输入上电意图（希望 VIN 上电）：
   - 将全局目标态 `PWR_SW_TARGET=Closed`（仅表达意图，不等同于实际导通）。
   - 实际导通由“基础电源输入子系统”的资格判定任务依据 `INA226` 电压/电流与 `IN_PG` 决定与等待，条件满足后才真正闭合输入开关（详见“2. 基础电源输入子系统”）。

2) 发布上电意图后，立即扫描前面板 `TCA6408A` 是否在线（存在性确认即可）：
   - 该步骤不依赖 VIN 是否已达标，作为独立可并行的存在性检查提前进行；
   - 成功与否仅记录日志，不阻塞后续 VIN 判定与模块初始化。

3) 等待 VIN 上电确认后，再开始模块侧扫描：
   - VIN 上电定义：由 `INA226.VBUS` 达到合规范围且 `IN_PG=good`（见第 2 章阈值），二者综合判定；
   - 在确认 VIN 上电后，才开始扫描与初始化 `SC8815`（逐通道）。
   - 日志：
     - `pwr.in:vin_on=true vin=..V pg=good`（确认上电后打印一次）；
     - 若超时未达标：`pwr.in:vin_on=false vin=..V pg=good|bad`；
     - 模块扫描开始前：`i2c.scan:start vin_on=true`。

4) 对“存在 SC8815”的 USB 电源模块，按以下顺序进行检查与初始化：
   - 读取 `SC8815` 的 `VBUS` 电压；要求 `VBUS ≥ 4.0 V` 且“连续两次扫描均达标”（建议采样间隔 ≥ 50 ms）；
   - 连续达标后，检查对应模块的 `SW2303` 是否在线（按驱动定义的在线检测方式，例如寄存器/ACK 检测）；
     - 若在线：执行 `SW2303` 初始化；
     - 若不在线：判定该模块“非完整”，记录日志并跳过其初始化阶段。

5) 初始化代码可参考：`/Users/ivan/Projects/Ivan/esp32c3-sc8815-sw2303-demo/src/main.rs` 中的示例流程与寄存器配置。

实现参考的伪代码（仅示意，非约束）：

```text
set(PWR_SW_TARGET, Closed)              // 发布上电意图
spawn(scan_front_panel_tca6408a)        // 立即扫描前面板存在性

wait_until(vin_ok_by_ina226_and_pg())   // 基于 INA226 + PG 的资格确认

for ch in mux.channels():               // VIN 确认后再扫描模块
    if sc8815.present(ch):
        sc8815.init(ch)
        if sc8815.vbus_ok_twice(ch, th=4.0V):
            if sw2303.present(ch):
                sw2303.init(ch)
            else:
                log_warn("module-incomplete", ch)
        else:
            log_warn("vbus<4V", ch)
```

### 1.4 设备发现/初始化（四路模块：SC8815 → SW2303）

- 使用的设备驱动 crate：
  - `sc8815`（Git：IvanLi-CN/sc8815-rs，支持 blocking/async，默认地址参见 `sc8815::registers::constants::DEFAULT_ADDRESS`）。
  - `sw2303`（Git：IvanLi-CN/sw2303-rs，支持 blocking/async，默认地址参见 `sw2303::registers::constants::DEFAULT_ADDRESS`）。
- 初始化/扫描策略（逐通道，串行）：
  1) 通过 `Xca9545a::split()` 获得的 `i2c[ch]` 访问该通道，仅对 SC8815 做存在性 ACK 判定；
  2) 若 SC8815 在线：完成初始化（外部电阻 `RS1/RS2=5mΩ`，`IBUS=5A`、`IBAT=6A`，`OTG`，`450kHz`，目标 `VBUS=5V`，使能 `ADC`/`OTG`），随后 MCU 拉高该路 `PSTOP_CTL`，经板上反相使模块侧 `PSTOP` 为低电平以使能输出；
  3) 轮询 SC8815 的 ADC，要求 `VBUS ≥ 4.0V` 且“连续两次扫描均达标”（建议采样间隔 ≥ 50 ms）；
     - 连续达标后先确认 `SW2303` 在线（按驱动定义的在线检测方式），在线则执行初始化；
     - 若不在线：记录 `anomaly=module-incomplete`，该通道不再继续 SW2303 初始化；
  4) 打印 `i2c.scan: ch=X sc8815=... sw2303=...`。若 SW2303 不在线，额外扫描 `0x30..0x3F` 打印 `sw2303_range=...` 辅助定位。
- 发现结果日志（每通道一行）：
  - 正常：`i2c.scan: ch=0 sc8815=online sw2303=online`；
  - 异常（缺失其一）：`i2c.scan: ch=0 sc8815=online sw2303=offline`。
- 通道一致性要求：SC8815 与 SW2303 成对出现。若非同时在线，视为“异常通道”，需打印错误日志但不中断其他通道的扫描与初始化：
  - `i2c.scan: ch=0 anomaly=true reason="pair-mismatch"`。

### 1.5 前面板 TCA6408ARGTR（0x20）发现与存在性确认

- 使用 crate：`port-expander`，类型：`Tca6408a`（支持阻塞 embedded‑hal）。
- 地址：`0x20`（ADDR=0，接地）。
- 步骤：在上行 I²C 直接构造 `Tca6408a`，读取输入寄存器以确认 ACK。扫描时机：
  - 于“发布电源输入上电意图”之后立即进行；不等待 VIN 达标；
  - 成功/失败仅记录日志，不阻塞后续 VIN 判定与模块初始化。
- 成功日志：`i2c.front: tca6408a=online addr=0x20`；失败：`i2c.front: tca6408a=offline addr=0x20`（仅报告，不 `panic!`）。

### 1.6 设备初始化（仅对“已发现在线”的设备执行）

初始化失败策略：打印错误日志并 `panic!`（除“不可达/不存在”的发现阶段外）。每个设备初始化成功都需打印一行成功日志。

- 四路模块（逐通道 ch=0..3）：
  - 跳过条件：若该通道 SC8815 或 SW2303 其一离线，跳过此通道初始化，输出：`pwr.mod: ch=0 init=skipped reason="pair-mismatch"`。
  - 顺序与要点：
    1) SC8815（电源路径/OTG 管理）：
       - 使用该通道的 `i2c[ch]` 构造 `SC8815`；
       - `init()` → 成功即打印：`pwr.sc8815: ch=0 init=ok`；失败：`pwr.sc8815: ch=0 init=err=...` 并 `panic!`；
       - 按项目配置进行基础参数设定（电池串数、限流、频率、OTG 设定等）；
       - 进入待机安全态，开启 ADC 转换，日志：`pwr.sc8815: ch=0 cfg=ok`；
    2) 等待 VBUS 就绪：轮询 SC8815 ADC，要求 `VBUS ≥ 4.0V` 且“连续两次达标”（阈值与次数可调）；达标日志：`pwr.sc8815: ch=0 vbus_ready=true vbus=xxxxmV`；超时记错误并 `panic!`；
    3) SW2303（PD/快充控制）：
       - 先确认 `SW2303` 在线（按驱动定义的在线检测方式，例如寄存器/ACK 检测）；
       - 若在线：同通道 `i2c[ch]` 构造 `SW2303`，执行 `init()`/解锁写使能 → 配置功率/协议（PD/PPS 等）；成功日志：`pwr.sw2303: ch=0 init=ok proto=pd+pps`；
       - 若离线：判定该通道“模块不完整”，记录 `pwr.sw2303: ch=0 online=false` 并跳过初始化；
  - 完成：`pwr.mod: ch=0 init=ok`。

- 前面板 TCA6408A（存在时才初始化）：
  - 方向配置：将五向开关对应引脚配置为输入（其余引脚按硬件用途配置为输入/输出）；
  - 清中断（若接入 INT）：读取输入寄存器一次；
  - 成功日志：`front.gpio: tca6408a init=ok`；失败则 `panic!`。

### 1.7 运行期资源交接（可选）

- 若运行期采用 `embedded-hal-async`：在完成上述阻塞式初始化后，销毁初始化阶段的 I²C/驱动实例，将底层 I²C 以 async 形式重建；
- 通过 `embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice` 共享上行 I²C；
- 运行期不再使用 `Xca9545a::split()` 的部件实例（仅初始化阶段使用），运行中由通道选择器件的“驱动层”显式切换通道，或继续复用 xca9548a 驱动在需要时切换（注意避免与 async 任务竞争）。

### 1.8 失败与异常处理约定

- 器件“不可达/不存在”（发现阶段）：仅打印报告，不 `panic!`；
- 器件“初始化失败”（初始化阶段）：打印 `error` 并 `panic!`；
- 通道配对异常（SC8815/SW2303 非成对）：打印 `error`，跳过该通道初始化；
- 统一错误日志示例：`init.err: comp=sc8815 ch=2 op=configure code=I2cNack detail="..."`。

### 1.9 参考 crate 与接口（事实依据）

- PCA9545A：`xca9548a`（`Xca9545a`，基于 embedded‑hal v1，阻塞接口；支持 `.split()` 提供 4 路虚拟 I²C）。
- TCA6408A：`port-expander`（`Tca6408a`，基于 embedded‑hal v1，阻塞接口）。
- SC8815：`sc8815-rs`（支持阻塞与 `embedded-hal-async`，已在 ESP32‑C3 演示中验证；可读取 `STATUS`/`ADC` 等）。
- SW2303：`sw2303-rs`（支持阻塞与 `embedded-hal-async`，提供协议配置与 ADC 读取等）。

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
