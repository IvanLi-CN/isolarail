# 文档 Web 站点历史

## 关键演进

- 首版创建独立 `docs-site/`，避免把普通 `docs/` Markdown 目录误当成可发布 Web 站点。
- 站点信息架构从按读者/用户类型分流改为按主题组织：快速开始、硬件拓扑、固件运行、控制面、Dashboard 和规格索引。
- 双语策略锁定为精选内容全双语，而不是仅双语导航或全量镜像仓库文档。
- 主题页从薄摘要扩展为可操作文档，补入 `just` 入口规则、HIL 序列、V3 拓扑图、sideband 表、boot/runtime 门控表、控制面分层、selector 决策表、dashboard 格式规则和 specs 阅读顺序。
- 首页主视觉从“空照片位”调整为复用已批准的项目配图和 dashboard 证据；仍然禁止伪造不存在的硬件实拍照片。
- 根语言入口进一步收敛为 compact switchboard；`/zh/` 与 `/en/` 首页回到 manual front door，而不是继续承载宣言式标题。
- 首页系统图只展示当前 V3 主路径；`PCA9545A@0x70` 保留在硬件文档的兼容命名语境中，不作为首页主视觉事实。
- Rspress 语言配置从全局共享导航改为 locale-specific 导航与侧栏，避免中文页点击主题导航时跳到英文或无语言前缀路径。
- 原 `tools/buzzer_audio_preview/` 静态试听页迁入 docs-site，保持固件音效候选数据，同时改用站点主题和双语路由。
- 首页导航从 light / dark 双图切换收敛为单一内联向量 lockup，既保留 Logo 字形和几何关系，也去掉了首屏重复 Logo 图片请求。
- 根语言页与双语首页的自定义 MDX 段落结构做了语义清理，generated HTML 不再出现嵌套 `<p>`。
- 首页 dashboard proof 图从 eager 改为 lazy/async，避免首屏非关键 SVG 抢占请求。
- 蜂鸣器音效预览页的按钮、筛选控件和标签从软胶囊语法收口回 44px 硬边模块；同时删除了未再使用的旧首页 CSS 层，避免过时风格重新覆盖当前 manual front door。
- Pages 发布基址从“默认固定 project-pages 子路径”收敛为“显式 `DOCS_BASE` 优先，其次随
  `CNAME` 自动切到根路径”，避免自定义域名首页继续引用 `/isolarail/...` 资源。
- 发布拓扑从“文档站单独占发布入口”修正为“`web/` 主站占根路径，`docs-site/` 挂在 `/docs/` 子目录”，并让 GitHub Pages 与 EdgeOne 共同复用同一份组合静态产物。
- 中文首页 hero 的移动端换行从统一 `ch` 限宽改回 locale-aware 处理，`IsolaRail 文档入口` 不再在首屏被切成生硬的断词。
- 首页 route/meta 微标签统一降低了 uppercase/letter-spacing 强度，让 family cue 回到 rail/marker 结构，而不是靠一排排小号眉标题硬撑。
- 移动端 `/zh/` / `/en/` 首页进一步把产品图提回 first fold，在首屏先交代硬件实体，再进入标题、CTA 和 route band，避免 manual front door 退化成纯文本前言。

## References

- `PRODUCT.md`
- `DESIGN.md`
- `docs-site/`
- `docs/specs/README.md`
