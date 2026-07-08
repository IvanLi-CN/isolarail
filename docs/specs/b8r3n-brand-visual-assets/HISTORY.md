# 品牌视觉资产历史

## 关键演进

- Logo 方向锁定为横向铁路/隔离意象的 `IsolaRail` lockup。
- 初版产品场景图保留为 HTML 海报素材，不再命名为最终海报。
- 最终海报改为包含 Logo、标题层级、产品主视觉和 footer 信息的完整 poster。
- GitHub Social preview 从单一图扩展为主图与变体。
- 主图采用更扁平、体积更小、适合 GitHub 上传的正面产品表达。
- 变体采用更写实的斜向产品表达，作为 campaign/social 备用视觉。
- 修正 GitHub Social preview 中产品右侧外壳异常楔形的问题，要求后续 social 资产不得出现明显违背常识的产品几何。
- 品牌资产从 `docs/assets/brand` 设计成品扩展到 `docs-site/docs/public` 和 `web/public` 运行时导出，替换默认站点图标与 social metadata。
- 运行时 icon 从整块白色画布改为透明画布上的圆角 tile。
- Logo 与 App Icon 均补齐 light / dark theme runtime exports，并在 docs-site / web 中按主题接入 favicon。
- App Icon runtime exports 改为从 1024 x 1024 SVG 源导出，SVG 几何按已批准 App Icon 的像素坐标还原。
- Logo 补齐 410 x 226 true SVG 几何版本，使用矩形轨道、矩形隔离符号和文字路径轮廓，并让 docs-site runtime logo 使用 SVG。

## References

- `docs/assets/brand/README.md`
- `docs/assets/brand/isolarail-github-social-preview.png`
- `docs/assets/brand/isolarail-github-social-preview-variant.png`
