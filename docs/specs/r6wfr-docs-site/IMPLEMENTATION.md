# 文档 Web 站点实现状态

## 当前状态

首版已实现到本地验证状态：新增 Rspress/Bun 文档站、主题式双语精选内容、组合静态发布 workflow、产品/设计上下文与视觉证据。主题页已从薄导览扩展为基于 `README.md`、`docs/software_design.md`、`docs/hardware_connection_overview.md`、`docs/dashboard_spec.md` 与相关 specs 的可操作文档。

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
- `docs/specs/r6wfr-docs-site/assets/`: 桌面、移动和内容页视觉证据。

## Remaining Gaps

- 真机或实物照片尚未提供，首页保留真实照片位。
- 视觉证据已补入 `SPEC.md`；后续 PR 阶段如有 UI 改动需重新绑定最新截图。

## Related Changes

- The root site now builds `web/` at `/` and `docs-site/` at `/docs/`, then assembles both into one publish directory.
- `docs-site/docs/public/CNAME` is copied to the artifact root so GitHub Pages can keep the custom-domain hot backup mapping.
- `DOCS_BASE` and `DOCS_PORT` still provide docs deploy and preview overrides; production publish now fixes `DOCS_BASE=/docs/`.
- The publish workflow resolves the site root base to `/` when `docs-site/docs/public/CNAME` exists, otherwise `/${repo}/`, and derives the docs base from that root as `/docs/` or `/${repo}/docs/`.
- EdgeOne direct-upload deploy reuses the same combined artifact through `edgeone makers deploy`.
- Local preview verified on leased port `57850`.
- Rspress locale routing uses explicit `zh` and `en` theme configs with `localeRedirect: 'never'`;
  the internal `x-default` locale is hidden from the language menu so topic navigation cannot drift
  into an unprefixed route.
