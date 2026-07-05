---
title: 接口与本机工具
description: USB JSONL、HTTP、isohub CLI、isohub-devd daemon 与 Web companion 的边界。
---

<!-- markdownlint-disable MD025 -->

# 接口与本机工具

ISO USB Hub 的控制面由设备固件、本机 daemon、CLI 和 Web app 共同组成。当前 owner-facing 入口固定为 `isohub` CLI；`isohub-devd` 是本机服务，不要求普通用户手动管理。

## 分层结构

```text
user
  ├─ isohub CLI
  │    └─ native IPC -> isohub-devd serve
  │          └─ USB JSONL / flash / reset / monitor
  ├─ Web app
  │    ├─ Wi-Fi / LAN HTTP
  │    ├─ Web Serial
  │    └─ explicit Local USB bridge from isohub-devd web
  └─ direct firmware links
       ├─ USB CDC JSONL
       └─ HTTP / LAN v1
```

默认路径是 `isohub` CLI 自动发现或启动 `isohub-devd serve`。`isohub-devd web` 只在浏览器开发和 same-origin Web hosting 中显式启用。

## 命名与身份

| 范围 | 当前名称 |
| --- | --- |
| 固件身份 | `iso-usb-hub` |
| CLI | `isohub` |
| daemon | `isohub-devd` |
| companion 源码 | `tools/isohub-companion/` |
| 设备 hostname | `isohub-<shortid>` |
| 端口 ID | `port1`、`port2`、`port3`、`port4` |

这些名称由 `docs/specs/pw97u-control-plane-alignment/SPEC.md` 持有。Web、CLI、README 和 diagnostics 不应重新发明产品名。

## USB JSONL

固件通过 USB Serial/JTAG JSONL 暴露基本操作：

- `info`
- `ports.get`
- `port.power_set`
- `port.replug`
- `wifi.get`
- `wifi.set`
- `wifi.clear`
- `reboot`

`info` 响应必须包含设备身份、MAC、固件名、版本和 uptime。companion 用这些字段做身份校验，避免把烧录或控制命令发给无关板子。

推荐的命令语义：

| 命令 | 类型 | 说明 |
| --- | --- | --- |
| `info` | read | 固件身份、hostname、MAC、版本、uptime |
| `ports.get` | read | 返回 `port1..port4` 的电源、sideband、OCP 与遥测状态 |
| `port.power_set` | write | 设置某一路 owner 手动输出允许状态 |
| `port.replug` | write | 对某一路做受控断电再恢复 |
| `wifi.get` | read | 读取 EEPROM 中的 Wi-Fi 配置状态，不泄露 PSK |
| `wifi.set` | write | 通过 USB-backed 路径写入 Wi-Fi 凭据 |
| `wifi.clear` | write | 清除 Wi-Fi 凭据 |
| `reboot` | write | 整机重启 |

`port.power_set` 和 `port.replug` 都必须以 `port1..port4` 为唯一端口 ID，不接受历史 `USB-A` / `USB-C` 或 `route` 模型。

## HTTP / LAN

设备端 HTTP v1 的目标接口包括：

- `GET /api/v1/health`
- `GET /api/v1/info`
- `GET /api/v1/ports`
- `GET /api/v1/ports/{portId}`
- `POST /api/v1/ports/{portId}/power`
- `POST /api/v1/ports/{portId}/actions/replug`
- `GET /api/v1/wifi`
- `POST /api/v1/reboot`

当前不把设备 HTTP v1 做成账号或云鉴权面。Wi-Fi 写操作仍要求 USB-backed 设备路径。

HTTP v1 的用途是 LAN 可见状态和有限维护动作。它不是默认 daemon transport，也不是云鉴权面。

| 路径 | 推荐用途 | 写入限制 |
| --- | --- | --- |
| `/api/v1/health` | 存活探测 | 无状态 |
| `/api/v1/info` | 设备身份、hostname、版本、网络状态 | 只读 |
| `/api/v1/ports` | 四路端口总览 | 只读 |
| `/api/v1/ports/{portId}` | 单路详情 | 只读 |
| `/api/v1/ports/{portId}/power` | 维护期手动开关 | 需要明确动作 |
| `/api/v1/ports/{portId}/actions/replug` | 受控断电再上电 | 需要明确动作 |
| `/api/v1/wifi` | Wi-Fi 状态 | 只读 |
| `/api/v1/reboot` | 维护重启 | 需要明确动作 |

LAN 上能看到设备，不等于可以写 Wi-Fi 凭据。Wi-Fi 写入必须有 USB-backed 当前设备路径。

## `isohub-devd` 模式

`isohub-devd` 有两个模式：

- `serve`：默认 native IPC daemon，只供本机 CLI/桌面路径使用。
- `web`：显式 localhost Web companion，用于浏览器开发和 same-origin Web hosting。

不要把 localhost HTTP 当作默认 daemon transport。Web runtime 也不得扫描 localhost 端口；可用 origin 必须来自同源 bootstrap 或显式 `DEVD_ORIGINS`。

| 模式 | 默认暴露 | 适用场景 |
| --- | --- | --- |
| `isohub-devd serve` | Unix domain socket / Windows named pipe | CLI、未来 desktop、本机单例 daemon |
| `isohub-devd web` | 显式 localhost Web companion | 浏览器开发、same-origin Web hosting |

普通用户不需要先手动启动 daemon。CLI 负责查找已运行实例，必要时启动 `serve` 模式。

## CLI 选择器

CLI 区分临时设备和已保存硬件：

- `--device <device-id>`：当前已连接的 USB 临时目标。
- `--hardware <saved-id>`：保存过的硬件 profile。

Wi-Fi 写入和清除要求 `--device` 或 USB-backed `--hardware`。`--url` 和 Wi-Fi/LAN saved hardware 保持只读。

选择器决策表：

| selector | 来源 | 可写 Wi-Fi | 可改端口 | 备注 |
| --- | --- | --- | --- | --- |
| `--device <device-id>` | 当前 USB 设备 | 是 | 是 | bring-up 和维修优先使用 |
| `--hardware <saved-id>` | 保存的硬件 profile | 仅 USB-backed | 是 | 需要确认当前通道 |
| `--url <http-url>` | LAN HTTP | 否 | 有限维护动作 | 不允许 Wi-Fi 写 |

多块设备同时在线时，不要省略 selector。状态变更命令应总是让目标可审计。

## 常用命令

只读检查：

```bash
just discover
just devices
just hardware-available
SELECTOR='--device <device-id>' just status
SELECTOR='--device <device-id>' just device-ports
SELECTOR='--device <device-id>' just wifi-show
```

设备动作：

```bash
SELECTOR='--device <device-id>' PORT=port1 ENABLED=true just port-power
SELECTOR='--device <device-id>' PORT=port1 just port-replug
SELECTOR='--device <device-id>' just device-reset
```

诊断：

```bash
SELECTOR='--device <device-id>' TAIL=200 just device-monitor
SELECTOR='--device <device-id>' just diagnostics-export
```

变更状态的命令需要串行执行。companion 会对同一串口路径做互斥，重叠请求可能返回 `device busy`。

## 诊断导出内容

`just diagnostics-export` 应聚合当前设备的：

- `status`
- `ports`
- `wifi`
- 最近 Local USB serial session traces
- daemon 看到的设备身份与选择器信息

导出目标是“复现一块板当时处于什么状态”，不是只保存最后一条错误消息。

## Web app 边界

Web app 统一仲裁三类通道：

- Wi-Fi / LAN
- Web Serial
- Local USB bridge

它不扫描 localhost，不把用户引向隐式端口发现，也不绕过 `isohub-devd` 的身份校验。

Web runtime 的通道仲裁规则：

- Wi-Fi / LAN、Web Serial、Local USB bridge 可能同时可用。
- 同一设备不能因为通道不同而重复出现。
- 最近成功通道优先；当前通道失效时切到其它可用通道。
- unsupported、busy、offline、USB-only 场景必须明确显示后续动作。

## 安全边界

- 烧录、reset、monitor 走 `isohub` / `isohub-devd` 路径，先做固件身份校验。
- Wi-Fi 写入只能走 USB-backed 路径。
- `port.replug` 是电源动作，不伪装成 USB 数据断开。
- localhost HTTP 不是默认 IPC；默认 daemon 不能因为 CLI 操作而暴露浏览器 HTTP 面。
- 状态变更要串行执行，避免并发写导致实际硬件状态和 UI 状态分叉。

## 参考

- `README.md`
- `docs/specs/pw97u-control-plane-alignment/SPEC.md`
- `docs/specs/q9d7h-cli-devd-flash-migration/SPEC.md`
