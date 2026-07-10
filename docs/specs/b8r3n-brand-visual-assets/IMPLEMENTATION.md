# 品牌视觉资产实现状态

## 当前状态

品牌视觉资产已形成项目内成品集：Logo、App Icon、最终海报、GitHub Social preview 主图、GitHub Social preview 变体和产品场景素材。

## Coverage

- `docs/assets/brand/isolarail-logo-lockup.png`: 当前主 Logo。
- `docs/assets/brand/isolarail-logo-lockup.svg`: 当前主 Logo 的 true SVG 几何源。
- `docs/assets/brand/isolarail-logo-lockup-light.svg`: Logo 亮色主题 SVG。
- `docs/assets/brand/isolarail-logo-lockup-dark.svg`: Logo 暗色主题 SVG。
- `docs/assets/brand/isolarail-logo-lockup-light.png`: Logo 亮色主题运行时版本。
- `docs/assets/brand/isolarail-logo-lockup-dark.png`: Logo 暗色主题运行时版本。
- `docs/assets/brand/isolarail-app-icon.png`: 当前已批准 App Icon 设计源图。
- `docs/assets/brand/isolarail-app-icon-light.svg`: App Icon 亮色主题 SVG 源。
- `docs/assets/brand/isolarail-app-icon-dark.svg`: App Icon 暗色主题 SVG 源。
- `docs/assets/brand/isolarail-app-icon-light.png`: App Icon 亮色主题运行时版本。
- `docs/assets/brand/isolarail-app-icon-dark.png`: App Icon 暗色主题运行时版本。
- `docs/assets/brand/isolarail-poster.png`: 当前最终海报。
- `docs/assets/brand/isolarail-github-social-preview.png`: GitHub Social preview 主图，低体积、正面产品表达。
- `docs/assets/brand/isolarail-github-social-preview-variant.png`: GitHub Social preview 变体，写实斜向产品表达。
- `docs/assets/brand/isolarail-product-scene-portrait.png`: 后续 HTML 海报产品场景素材。
- `docs/assets/brand/isolarail-product-scene-wide.png`: 后续 HTML social/hero 宽幅产品场景素材。
- `docs/assets/brand/README.md`: 品牌资产目录说明。
- `README.md`: 品牌资产入口链接。
- `docs/specs/b8r3n-brand-visual-assets/SPEC.md`: 完整资产清单与视觉预览。
- `docs-site/docs/public/`: 文档站运行时 favicon、manifest icon、logo 和 social preview 导出。
- `web/public/`: Web 控制台运行时 favicon、manifest icon、logo 和 social preview 导出。
- `docs-site/rspress.config.ts`: 文档站 favicon、logo、manifest、Open Graph 和 Twitter metadata 接入。
- `web/index.html`: Web 控制台 favicon、manifest、Open Graph 和 Twitter metadata 接入。
- `docs-site/theme/DocsNavTitle.tsx`: 文档站导航改为内联 lockup 几何，避免 light / dark 双图请求，同时继续复用已批准 Logo 的轨道、隔离符和字形轮廓。

## Remaining Gaps

- GitHub repository settings 中的 social preview 上传需要 owner 侧操作。
- HTML 海报模板尚未实现。

## Related Changes

- 去掉含糊的 `isolarail-social-preview-layout-source.png` 资产命名。
- 新增 `isolarail-github-social-preview-variant.png`，作为明确的 social preview 变体。
- 从 App Icon 和主 GitHub Social preview 派生 docs-site / web 运行时静态资产。
- Runtime icon 改为透明画布上的圆角 tile，避免 favicon、touch icon 和 manifest icon 出现整块白色画布背景。
- Logo 与 App Icon 均补齐 light / dark theme runtime exports。
- App Icon runtime exports 改为从 1024 x 1024 SVG 源导出，SVG 几何按已批准 App Icon 的像素坐标还原。
- Logo 补齐 410 x 226 true SVG 几何版本，使用矩形轨道、矩形隔离符号和文字路径轮廓，不使用 raster 嵌入。
- 文档站导航不再依赖同时渲染两张主题 Logo 图片，而是以内联向量 lockup 复用同一套几何，在主题切换时只改变颜色角色。
