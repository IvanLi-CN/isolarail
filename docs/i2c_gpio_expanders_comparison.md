# I2C GPIO 扩展器对比（直接横向，不分等级）

> 作者：白羽（Augment Agent）
> 更新时间：2025-08-19
> 适用：USB Hub 与通用 MCU 项目器件选型

## 一、快速要点（先看这里）

- 所有器件均支持 I²C 100/400 kHz、上电默认输入、开漏 INT（需上拉）
- 953x 系列（TCA9534/9535）：不带内部弱上拉，待机更低、外部网络可完全自定
- 955x/PCAx 系列（TCA9554/9555、PCA9555）：输入含典型 ~100 kΩ 弱上拉，更省外围
- “A” 后缀（9534A/9554A）：功能电气一致，固定 I²C 地址不同，便于与非 A 混挂扩容
- 若需跨电压域或要硬件复位脚：选 TCA6408A（8-bit，VCCI/VCCP 分离，带 RESET）

## 二、核心参数横向对比

| 项目 | TCA9534 (8b) | TCA9554 (8b) | TCA9535 (16b) | TCA9555 (16b) | PCA9555 (16b) | TCA6408A (8b, level-shift) |
|---|---|---|---|---|---|---|
| GPIO 数 | 8 | 8 | 16 | 16 | 16 | 8 |
| 输入弱上拉 | 无 | 有（~100 kΩ） | 无 | 有（~100 kΩ） | 有（~100 kΩ） | 无（推挽端口） |
| 供电范围 | 1.65–5.5 V | 1.65–5.5 V | 1.65–5.5 V | 1.65–5.5 V | 2.3–5.5 V | VCCI 1.65–5.5 V；VCCP 1.65–5.5 V |
| I²C 速率 | 100/400 kHz | 100/400 kHz | 100/400 kHz | 100/400 kHz | 100/400 kHz | 100/400 kHz |
| 地址脚 | A0–A2（8 地址）| A0–A2（8 地址）| A0–A2（8 地址）| A0–A2（8 地址）| A0–A2（8 地址）| ADDR（2 地址）|
| INT | 开漏，需上拉 | 开漏，需上拉 | 开漏，需上拉 | 开漏，需上拉 | 开漏，需上拉 | 开漏，需上拉；另带 RESET |
| I/O 电气 | 5V 容忍；外接上拉自定 | 5V 容忍；弱上拉默认高 | 5V 容忍；外接上拉自定 | 5V 容忍；弱上拉默认高 | 5V 容忍；弱上拉默认高 | 5V 容忍；推挽输出可直接驱动 LED（见手册曲线） |
| 上电默认 | 输入 | 输入（被弱上拉拉高）| 输入 | 输入（被弱上拉拉高）| 输入（被弱上拉拉高）| 输入；无毛刺上电；带 RESET |
| 封装（常见） | SOIC/TSSOP | SOIC/TSSOP/SSOP | TSSOP/QFN/SSOP | TSSOP/QFN/SSOP | TSSOP/QFN/SSOP | TSSOP/QFN/UQFN |

注：电流能力、VOL/VOH 曲线、ΔICC 等请以各自数据手册为准。

## 三、TCA9534 vs TCA9554（直观差异）

- 相同点：1.65–5.5 V、100/400 kHz、A0–A2 编址（8 地址）、INT 开漏、寄存器/时序一致、5V 容忍
- 不同点：
  - 9534：无内部弱上拉；未用输入需外部上拉/下拉避免漂浮；待机更低，网络可自定
  - 9554：输入带 ~100 kΩ 弱上拉；外围更省，但某些拓扑（长线/按钮/LED 反偏）需注意误触发与 ICC；按手册做去抖与电阻分配
- A/非A：仅 I²C 地址不同（便于与非 A 同挂，加倍地址资源）

## 四、选型建议（按需求）

- 8 路 GPIO：
  - 省外围、快速落地：TCA9554
  - 追求低待机/自控上拉：TCA9534
  - 跨电压域/要硬复位：TCA6408A
- 16 路 GPIO：
  - 省外围：TCA9555 / PCA9555（注意 PCA9555 供电 2.3–5.5 V）
  - 低待机/自控上拉：TCA9535
- 地址紧张：混用 A 与非 A（9534/9534A 或 9554/9554A），并合理配置 A0–A2

## 五、工程注意事项

- INT/输入悬空：
  - 无弱上拉（9534/9535）：未用输入务必上拉/下拉；按钮/长线建议 RC 去抖
  - 有弱上拉（9554/9555/PCA9555）：默认高电平，外部仍需根据应用优化分压/抗干扰
- LED 与 ΔICC：I/O 作为输入且外侧挂 LED 时，LED 反偏可能使 I/O < VCC 导致 ICC 增加（见 TI 手册“Minimizing ICC when I/Os control LEDs”），按推荐拓扑并联大阻或调整供电侧
- I²C 上拉：按总线电容计算 Rp；400 kHz 建议评估 2.2–4.7 kΩ 量级（视布线/负载而定）
- 版图：就近去耦、INT 单点上拉、避免 INT/SCL/SDA 贴近高速差分对

## 六、权威参考

- [TI TCA9534](https://www.ti.com/lit/ds/symlink/tca9534.pdf)
- [TI TCA9534A](https://www.ti.com/lit/ds/symlink/tca9534a.pdf)
- [TI TCA9554](https://www.ti.com/lit/ds/symlink/tca9554.pdf)
- [TI TCA9554A](https://www.ti.com/lit/ds/symlink/tca9554a.pdf)
- [TI TCA9535](https://www.ti.com/product/TCA9535)
- [TI TCA9555](https://www.ti.com/product/TCA9555)
- [NXP PCA9555](https://www.nxp.com/docs/en/data-sheet/PCA9555.pdf)
- [TI TCA6408A](https://www.ti.com/lit/gpn/tca6408a)

## 七、最终选型：TCA6408ARGTR（TI，VQFN-16 RGT）

- 最终选择：TCA6408ARGTR（RGT-16，3.0×3.0 mm，带热焊盘）
- 选择理由（摘要）：
  - VCCI/VCCP 双电源，天然支持 I²C 与外设不同电压域（1.8/2.5/3.3/5V 组合），省电平转换器
  - 带硬件 RESET，异常时可快速复位寄存器与状态机，无需断电
  - P 口推挽、直接驱动 LED 能力较好；上电无毛刺，默认全部为输入
  - 面积小、布板友好；开漏 INT 便于中断汇聚
- 地址/扩展：ADDR 单脚，最多 2 颗并挂；若需更多，配合 TCA9548A 等 I²C MUX
- 渠道与价格（参考）：优信 1.52 元/pcs（2025-08-19，随行就市，请以实际下单为准）
- BOM 备注：可建立封装备选 TCA6408APWR（TSSOP-16），以便产能/封装切换

## 八、TCA6408ARGTR 详解

- 核心特性
  - I²C GPIO 扩展（8-bit），100/400 kHz；开漏 INT；硬件 RESET
  - 供电：VCCI（I²C 侧）1.65–5.5 V；VCCP（P 口侧）1.65–5.5 V；5V 兼容 I/O
  - 电平转换：VCCI/VCCP 组合可在 1.8/2.5/3.3/5V 之间跨域（见数据手册表格）
  - 上电行为：默认全部为输入；“No glitch on power up”
  - ESD/Latch‑up：HBM ±2 kV、CDM ±1 kV；JESD78 100 mA
- I²C 地址
  - ADDR=0：7‑bit 地址 0x20；ADDR=1：7‑bit 地址 0x21（TI DS：表 8‑3）
- 寄存器映射（命令字）
  - 0x00 Input Port（只读）
  - 0x01 Output Port（读写，输出锁存）
  - 0x02 Polarity Inversion（读写）
  - 0x03 Configuration（读写；1=输入，0=输出）
- 关键电气能力（典型/限值以 DS 为准）
  - P 口灌电流 IOL ≤ 25 mA；拉电流 IOH ≤ 10 mA
  - VOH/VOL 与 I/V 曲线详见手册“Typical Characteristics”
  - INT/RESET 连接至 VCCI 侧；INT 为开漏，需上拉
- 时序/复位
  - I²C：Standard 100 kHz / Fast 400 kHz；典型 INT 有效/复位延时 tiv/tir≈4 µs
  - RESET：低有效，tW≥4 ns（最小脉宽），tRESET≈600 ns；建议上拉至 VCCI
- 典型用法
  - LED 与继电器控制、开关/按键输入、下游器件的 EN/RESET/选择信号
  - 若用作 LED 控制，注意 ΔICC：LED 熄灭时 I/O 电位不得低于 VCCP，否则待机电流上升（并联高阻/调整供电侧，见 DS 图 9‑2/9‑3）
- 布局与硬件建议
  - VCCI、VCCP 就近 0.1 µF 去耦；RGT 封装热焊盘可靠焊接并接地
  - INT/SCL/SDA 远离高速差分线；INT 上拉电阻按系统电容与干扰选取（常用 4.7–10 kΩ）
  - 供电时序：优先拉升 VCCP 再拉升 VCCI，可避免 SDA 潜在拉低（见 DS 10.1）
- 初始化示例（伪代码）
  - 设定某 4 位为输出，其余为输入：
    1) 写入 0x03（Configuration）：例如 0b11110000（高 4 位输入，低 4 位输出）
    2) 写入 0x01（Output）：例如 0b00000101（P0/P2 输出高，其它输出低）
    3) 读 0x00（Input）获取输入状态；必要时读取后清除 INT
- 订货信息/封装
  - 料号：TCA6408ARGTR（VQFN‑16 RGT，3000/卷）；备选：TCA6408APWR（TSSOP‑16）/TCA6408ARSVR（UQFN‑16）
