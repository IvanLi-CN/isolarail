# 文档 Web 站点（#r6wfr）

## 状态

- Lifecycle: active
- Implementation: 首版实现中

## 背景 / 问题陈述

仓库已有大量 Markdown、PDF 和规格文档，但没有独立可构建、可预览、可发布的文档 Web 站点。读者需要先理解产品边界、bring-up 路径、硬件拓扑、固件运行、控制面和长期工程真相源，而不是直接浏览仓库目录。

## 目标 / 非目标

### Goals

- 新增独立 `docs-site/` 子工程，作为可发布的双语文档站点。
- 首版覆盖首页、快速开始、硬件拓扑、固件运行、蜂鸣器音效预览、控制面、Dashboard 和规格索引。
- 使用 Rspress + Bun，提供 `docs:dev`、`docs:build` 和 `docs:preview`。
- 接入 GitHub Pages workflow，默认适配仓库 project-pages 子路径，并支持 `DOCS_BASE`
  覆盖到自定义域名根路径。
- 将站点视觉证据写回本 spec。

### Non-goals

- 不实现设备控制台、在线配置、串口烧录或运行时硬件控制。
- 不全量迁移 `docs/plan/**`。
- 不伪造实物照片或生成不可信产品渲染。
- 不修改固件行为、硬件引脚定义或 release 产物规则。

## 范围

### In scope

- `PRODUCT.md` / `DESIGN.md` 站点上下文。
- `docs-site/` Rspress 配置、样式、双语内容与静态资产。
- 根 `package.json` docs scripts 与 Bun lockfile。
- `.github/workflows/docs-pages.yml` 发布 workflow。
- README 文档入口同步。

### Out of scope

- Firmware source behavior changes.
- Hardware netlist or BOM changes.
- Custom domain DNS / CNAME 具体域名配置。

## 需求

### MUST

- 文档站必须能通过 `bun run docs:build` 构建。
- 文档站必须提供中文和英文等价的精选首版内容。
- 本地站点默认 `DOCS_BASE=/`；GitHub Pages workflow 默认使用仓库 project-pages 子路径，并能通过
  `DOCS_BASE` 覆盖。
- 站内手写链接和图片引用不得依赖固定 `/isolarail/` 路径。
- 本地预览必须使用端口租约，不直接抢默认端口。
- UI 视觉证据必须在合入前回传给主人，并写入本 spec。

### SHOULD

- README 保持快速入口，把完整阅读路径交给 docs-site。
- 内容应明确 canonical sources，避免站点漂移成第二套真相源。
- 视觉风格应体现产品官网入口与工程文档的混合形态，但信息架构按主题组织，不按用户类型拆分页面。

## 功能与行为规格

- 首页解释 IsolaRail 的产品定位，并提供快速开始、硬件拓扑和规格索引入口。
- 快速开始页给出 ESP Rust 工具链、固件构建、文档站构建和本机控制面基础命令。
- 硬件拓扑页总结当前 V3 canonical 命名、电源输入、CH335F sideband、两条 I²C 总线、四路遥测与前面板连接。
- 固件运行页总结 boot init、自检、降级策略、输出门控、前面板输入、蜂鸣器告警和日志风格。
- 蜂鸣器音效预览页迁入 `tools/buzzer_audio_preview/` 的试听能力，使用站点主题提供
  Web Audio 播放、分组筛选和循环告警试听。
- 控制面页总结 USB JSONL、HTTP、`isolarail`、`isolarail-devd`、CLI 选择器和 Web app 边界。
- Dashboard 页展示 160x50 布局、状态、输入映射、刷新规则和正常态/混合状态 SVG。
- 规格索引页链接关键 specs，并说明 specs/current-truth 文档才是长期规范源。

## 验收标准

- `bun install --frozen-lockfile` 成功。
- `bun run docs:build` 成功。
- `DOCS_BASE=/preview/ bun run docs:build` 成功。
- 预览中 `/zh/`、`/en/`、quick-start、hardware、firmware、control-plane、dashboard、reference 页面可访问。
- 桌面和移动端截图证明首页与至少一个内容页无明显布局溢出。
- `cargo +esp check --target xtensa-esp32s3-none-elf` 成功。
- `cargo +esp build --release --target xtensa-esp32s3-none-elf` 成功。

## 非功能性验收 / 质量门槛

### Testing

- Rspress build and preview smoke test.
- ESP32-S3 firmware check and release build.
- GitHub Pages workflow uses the same docs build command.

### UI / Visual Evidence

预览服务：`http://127.0.0.1:57850/`，端口租约 `isolarail--0dd2c3c0--docs-site / docs-site`。

验证覆盖：

- `/`、`/zh/`、`/en/`
- `/zh/start/quick-start`、`/en/start/quick-start`
- `/zh/hardware/topology`、`/en/hardware/topology`
- `/zh/firmware/boot-runtime`、`/en/firmware/boot-runtime`
- `/zh/firmware/buzzer-audio-preview`、`/en/firmware/buzzer-audio-preview`
- `/zh/control-plane/interfaces`、`/en/control-plane/interfaces`
- `/zh/dashboard/front-panel`、`/en/dashboard/front-panel`
- `/zh/reference/specs`、`/en/reference/specs`

结果：全部返回 200；无 failed request；无 console warning/error；检查视口无横向溢出；旧的按用户分流标题未出现在页面正文中。

桌面首页：

![Docs site home desktop](assets/docs-site-home-desktop.png)

移动首页：

![Docs site home mobile](assets/docs-site-home-mobile.png)

控制面内容页：

![Docs site control plane desktop](assets/docs-site-control-plane-desktop.png)

中文页语言稳定导航：

![Docs site locale-stable Chinese nav](assets/docs-site-locale-nav-zh.png)

蜂鸣器音效预览页：

![Docs site buzzer audio preview](assets/docs-site-buzzer-audio-preview-en.png)

蜂鸣器音效预览响应式回归：

![Docs site buzzer audio preview responsive 900px](assets/docs-site-buzzer-audio-preview-responsive-900.png)

![Docs site buzzer audio preview responsive mobile](assets/docs-site-buzzer-audio-preview-responsive-mobile.png)

## 文档更新

- `README.md`
- `PRODUCT.md`
- `DESIGN.md`
- `.github/workflows/docs-pages.yml`

## 风险 / 开放问题 / 假设

- 自定义域名的实际域名暂未写入仓库，因此首版不提交 `CNAME`。
- 没有真实设备照片，首页只保留照片位，不生成替代渲染图。
- Rspress i18n 和 route behavior 以本地 build/preview 验证为准。
