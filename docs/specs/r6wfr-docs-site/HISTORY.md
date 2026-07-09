# 文档 Web 站点历史

## 关键演进

- 首版创建独立 `docs-site/`，避免把普通 `docs/` Markdown 目录误当成可发布 Web 站点。
- 站点信息架构从按读者/用户类型分流改为按主题组织：快速开始、硬件拓扑、固件运行、控制面、Dashboard 和规格索引。
- 双语策略锁定为精选内容全双语，而不是仅双语导航或全量镜像仓库文档。
- 主题页从薄摘要扩展为可操作文档，补入 `just` 入口规则、HIL 序列、V3 拓扑图、sideband 表、boot/runtime 门控表、控制面分层、selector 决策表、dashboard 格式规则和 specs 阅读顺序。
- 首页主视觉保留真实照片位，不生成可能误导读者的产品渲染图。
- 首页系统图只展示当前 V3 主路径；`PCA9545A@0x70` 保留在硬件文档的兼容命名语境中，不作为首页主视觉事实。
- Rspress 语言配置从全局共享导航改为 locale-specific 导航与侧栏，避免中文页点击主题导航时跳到英文或无语言前缀路径。
- 原 `tools/buzzer_audio_preview/` 静态试听页迁入 docs-site，保持固件音效候选数据，同时改用站点主题和双语路由。
- Pages 发布基址从“默认固定 project-pages 子路径”收敛为“显式 `DOCS_BASE` 优先，其次随
  `CNAME` 自动切到根路径”，避免自定义域名首页继续引用 `/isolarail/...` 资源。
- 发布拓扑从“文档站单独占发布入口”修正为“`web/` 主站占根路径，`docs-site/` 挂在 `/docs/` 子目录”，并让 GitHub Pages 与 EdgeOne 共同复用同一份组合静态产物。

## References

- `PRODUCT.md`
- `DESIGN.md`
- `docs-site/`
- `docs/specs/README.md`
