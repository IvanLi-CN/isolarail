---
title: 启动与运行期
description: 固件 boot self-check、门控决策、运行期采样、前面板和风扇链路。
---

<!-- markdownlint-disable MD025 -->

# 启动与运行期

固件运行口径由 `docs/software_design.md` 和对应 specs 持有。本页把启动、门控和运行期链路压缩成一条阅读路径。

## 启动四阶段

启动流程固定为：

```text
Early Bring-up -> Self-Check -> Gate Apply -> Runtime
```

`Self-Check` 阶段同时做三件事：

- 输出 `boot.stage:*`、`boot.check:*`、`boot.summary:*` 日志。
- 驱动 160x50 LCD 的启动自检页。
- 产出 `GateDecision`，决定 runtime task、front panel 和端口是否放行。

状态模型：

- `SelfCheckItemState = Pending / Ok / Warn / Err / Fatal / Skipped`
- `BootOutcome = Ok / Degraded / Fatal`
- `BootFaultCode` 覆盖 mux、输入电源、INA、前面板、风扇和每路端口故障。

启动页、串口日志和门控决策共享同一个 `BootSelfCheckSnapshot`。这避免了 LCD 显示“正常”、串口日志显示“异常”、runtime 又采用第三套判断的分裂状态。

## 固定自检顺序

自检顺序必须稳定，避免多个模块抢总线：

1. 初始化日志、时间源、LCD、两条 I²C 和基础 GPIO。
2. 确认当前 I²C 拓扑。当前验证板记录 `mux=Skipped`。
3. 启动并等待基础电源输入资格。
4. VIN ready 后探测前面板 `TCA6408A@0x21` 与风扇链路。
5. VIN ready 后初始化主板 `TCA6408A@0x20`、CH335F sideband 和四路端口扫描。
6. 汇总 `GateDecision` 并输出 `boot.summary:*`。

典型日志形状：

```text
app.start
init.time: embassy-timer=ok
boot.stage: stage=self-check
boot.check: name=mux state=skip fault=-
boot.check: name=vin state=ok fault=-
boot.check: name=front_panel state=warn fault=FrontPanelOffline
boot.summary: outcome=DEG first_fault=FrontPanelOffline runtime=on front_panel=off
```

串口监视晚接入时，`boot.summary:*` 仍应足够定位启动结论。

## 降级策略

默认策略是只读探测和分级降级：

- 输入电源不安全：`Fatal`，`IN_CE` 保持关闭，runtime 不放行。
- 前面板离线：当前 V3 记为 `Warn/FrontPanelOffline`，禁用前面板输入，继续 dashboard/runtime。
- 风扇链路失败：记为 `Warn/FanUnavailable`，不阻断 dashboard。
- 单路 `INA226/TMP112` 缺失：该路记为 `Err`，影响诊断与遥测，不单独关闭其它端口。
- 主板 `TCA6408A@0x20` 离线：进入 degraded manual mode，sideband fault 必须持续可见。

| 条件 | 结果 | 端口 | Dashboard |
| --- | --- | --- | --- |
| VIN / PG 不安全 | `Fatal` | 全部 skipped / off | 常驻 fatal 自检页 |
| `PCA9545A` 不存在 | `Skipped` | 不受影响 | 继续 |
| 前面板 `0x21` 离线 | `Degraded` | 不受影响 | 继续，禁用按键 |
| 风扇链路失败 | `Degraded` | 不受影响 | 继续 |
| 单路遥测缺失 | `Degraded` | 不单独关断 | 该路诊断为 `Err` |
| 主板 sideband 离线 | `Degraded` | manual mode | sideband fault 可见 |

## 运行期端口门控

每路输出使能由以下条件共同决定：

- `VIN ready`
- 主板 sideband 在线，或 degraded manual mode 明确放行
- `V1OK` 模式：standalone 或 upstream-managed
- 对应 `PWREN#` 状态
- 软件过流 latch 状态
- 前面板手动断开状态

软件过流判定：

- `vbus < 3.0 V && current > 0.1 A`
- 或 `current > 5.3 A`

命中过流后立即关闭对应 `ENx` 并注入 `OVCUR#`。连续 4 个 500 ms 带电安全采样后才释放 latch。

运行期每路状态至少要能解释这些问题：

- 当前是不是 VIN ready？
- `V1OK` 让设备处于 standalone 还是 upstream-managed？
- `PWREN#` 是否允许当前端口？
- owner 有没有手动关闭这一路？
- 是否存在 OCP latch？
- 当前遥测是不是新鲜数据，还是初始化 / 缺失 / 读取失败？

关闭输出后读到的 `0V/0mA` 不能作为 latch 清除证据。恢复必须来自带电探测期间连续安全读数。

## 前面板交互

前面板任务只发布去抖后的按键事件，不直接持有 `EN1..EN4`：

- 左 / 右：在 4 个端口间循环移动选中列。
- 中键短按：切换选中端口的手动输出允许状态。
- 上 / 下：保留给未来显示模式或详情页。

选择变化和手动状态变化必须立即刷新 dashboard，不能只等周期遥测刷新。

手动断开状态不持久化。每次上电后默认四路手动允许，但实际输出仍受 VIN、sideband、OCP 和 `PWREN#` 共同约束。

## 蜂鸣器与告警

V3 `BUZZER` 网络由 `GPIO7` 驱动。固件使用 LEDC PWM 播放无源蜂鸣器音效，空闲和播放完成后必须拉低 GPIO7。

音效规则：

- boot self-check 非 `Fatal` 且进入 `Runtime` 后播放开机音。
- 左 / 右移动选中列播放操作提示音。
- 中键成功启用端口播放上电音，成功禁用播放断电音。
- 控制面触发 `PortPowerSet(enabled=true)` 播放上电音。
- `PortPowerSet(enabled=false)` 或 `PortReplug` 播放断电音。

需要试听时，进入[蜂鸣器音效预览](buzzer-audio-preview)页面比较时序，再回到工具脚本生成
离线试听产物。

告警优先级：

1. `channel_short`
2. `over_temp`
3. `input_over_power`
4. `channel_over_5a`
5. one-shot 操作音

保护关断同一 tick 内必须抑制普通拔出 / 断电提示，只保留告警音。

## 日志风格

日志使用单行键值对，便于串口和诊断导出解析：

- 启动：`boot.stage:*`、`boot.check:*`、`boot.summary:*`
- 输入电源：`pwr.in:*`
- 前面板：`i2c.front:*`、`i2c.front_diag:*`
- sideband：`hub.sideband:*`

新增模块日志应保持单行键值对，避免把同一次状态变化拆成多行叙事日志。诊断导出和 `device-monitor` 都依赖这种结构。

## 验证路径

| 场景 | 期望 |
| --- | --- |
| 正常启动 | LCD 先显示自检页，再进入 dashboard |
| 直连 I²C 板型 | `mux=Skipped`，端口扫描继续 |
| VIN 异常 | `Fatal`，`IN_CE` 关闭，runtime 不启动 |
| 前面板离线 | `Warn/FrontPanelOffline`，dashboard 继续 |
| 单路遥测缺失 | 对应端口 `Err`，其它端口继续 |
| OCP 命中 | 关闭 `ENx`，注入 `OVCUR#`，dashboard 显示 over-current |

## 参考

- `docs/software_design.md`
- `docs/specs/5f74j-firmware-boot-self-check/SPEC.md`
- `docs/specs/h8c4s-ch335f-sideband-power-control/SPEC.md`
- `docs/specs/7gf6b-firmware-buzzer-audio/SPEC.md`
