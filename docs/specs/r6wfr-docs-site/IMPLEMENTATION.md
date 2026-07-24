# 文档 Web 站点实现状态

## 当前状态

首版已实现到本地验证状态：新增 Rspress/Bun 文档站、主题式双语精选内容、组合静态发布 workflow、产品/设计上下文与视觉证据。主题页已从薄导览扩展为基于 `README.md`、`docs/software_design.md`、`docs/hardware_connection_overview.md`、`docs/dashboard_spec.md` 与相关 specs 的可操作文档；首页则进一步收敛为语言 switchboard + locale manual front door 的组合结构。最近一轮整改又补上了深层内容页控件收口：蜂鸣器音效预览页的 CTA、筛选下拉和状态块统一回硬边模块语法，并移除了首页旧版失效样式层，避免后续串回过时视觉。

本轮 docs-site 收口继续聚焦首屏可信度：中文首页 hero 标题的移动端断裂被修正，route/meta 微标签从过量 uppercase 收回到更稳的阅读语气；同时把移动端首屏顺序改回“先产品图、后标题和入口动作”，继续沿用已批准的产品配图与 dashboard proof 保持 same-project 家族感。

主题切换通过本地 theme entry 接管，内部基础组件从 `@rspress/core/theme-original` 引用以避免 alias 递归。切换动画以真实点击点或开关中心为原点；结束后主动清理 root animation，防止后续轮次复用残留的 `forwards` 填充状态。首页亮色与暗色产品图由 CSS 按当前主题互斥显示。

## Coverage

- `docs-site/`: 首版站点交付面。
- `docs-site/docs/{zh,en}/start/quick-start.md`: 工具链、`just` 入口规则、单板 HIL 序列、Wi-Fi 写入边界、质量门禁和本机控制面起步路径。
- `docs-site/docs/{zh,en}/hardware/topology.md`: 当前 V3 电源、系统拓扑、CH335F sideband、I²C、端口遥测、前面板显示和维护动作能力边界。
- `docs-site/docs/{zh,en}/firmware/boot-runtime.md`: boot self-check、日志形状、降级策略、运行期门控、前面板输入、蜂鸣器告警和验证路径。
- `docs-site/docs/{zh,en}/firmware/buzzer-audio-preview.mdx`: 站内 Web Audio 蜂鸣器试听工作台、分组筛选和循环告警试听。
- `docs-site/src/components/BuzzerAudioPreview.tsx`: 从 `tools/buzzer_audio_preview/` 迁入的提示音数据和浏览器播放逻辑。
- `docs-site/docs/{zh,en}/control-plane/interfaces.md`: USB JSONL、HTTP、`isolarail`、`isolarail-devd`、CLI 选择器、诊断导出、Web app 仲裁和安全边界。
- `docs-site/docs/{zh,en}/dashboard/front-panel.md`: 160x50 dashboard 布局、状态优先级、格式化规则、刷新来源、颜色约束和预览资产。
- `docs-site/docs/{zh,en}/reference/specs.md`: specs/current-truth 索引、阅读顺序和事实优先级。
- `scripts/assemble-site-artifact.mjs`: 组合根站点与 `/docs/` 子站点的发布目录。
- `PRODUCT.md` / `DESIGN.md`: 站点战略与视觉合同。
- `.github/workflows/docs-pages.yml`: `web/` + `docs-site/` 组合发布链路。
- `README.md`: 文档站入口。
- `docs/specs/r6wfr-docs-site/assets/`: 根语言入口、首页桌面/移动端和内容页视觉证据。

## Remaining Gaps

- 首页现在复用已批准的项目配图与 dashboard 证据；后续若补入真实 bench 照片，可在不改变 switchboard / manual 结构的前提下替换或补强 hero 视觉。
- 视觉证据已补入 `SPEC.md`；后续 PR 阶段如有 UI 改动需重新绑定最新截图。
- 文档站全局样式已清理掉不再被任何页面引用的旧首页 hero/map 规则，减少后续回归风险。

## Related Changes

- The root site now builds `web/` at `/` and `docs-site/` at `/docs/`, then assembles both into one publish directory.
- `docs-site/docs/public/CNAME` is copied to the artifact root so GitHub Pages can keep the custom-domain hot backup mapping.
- `DOCS_BASE` and `DOCS_PORT` still provide docs deploy and preview overrides; production publish now fixes `DOCS_BASE=/docs/`.
- The publish workflow resolves the site root base to `/` when `docs-site/docs/public/CNAME` exists, otherwise `/${repo}/`, and derives the docs base from that root as `/docs/` or `/${repo}/docs/`.
- EdgeOne direct-upload deploy reuses the same combined artifact through `edgeone makers deploy`.
- The EdgeOne deploy step must stage `.site-publish` into a temporary directory outside the repository before invoking the CLI; deploying the same files from an in-repo path caused EdgeOne preview URLs to return `504 CLOUD_FUNCTION_INVOCATION_TIMEOUT` even though the deployment logs reported a pure static project.
- Local preview verified on leased port `57850`.
- Rspress locale routing uses explicit `zh` and `en` theme configs with `localeRedirect: 'never'`;
  the internal `x-default` locale is hidden from the language menu so topic navigation cannot drift
  into an unprefixed route.
- 首页导航已改为 `docs-site/theme/DocsNavTitle.tsx` 内联向量 lockup；主题切换改走 CSS 色彩角色，不再为了 light / dark 同时请求两张 Logo 图片。
- `docs-site/theme/appearanceTransition.ts` 负责主题动画生命周期，`docs-site/theme/DocsSwitchAppearance.tsx` 负责将实际点击坐标传入动画；`rspress.config.ts` 的主题 alias 与 `theme-original` 导入共同保证主题入口可控且无递归。
- 首页 root / `zh` / `en` 自定义 MDX 已修正为有效段落结构，避免 generated HTML 出现嵌套 `<p>`。
- 首页 proof 图改为 `loading="lazy"` + `decoding="async"`，减少首屏非关键图像竞争。
