# 品牌视觉资产（#b8r3n）

## 状态

- Lifecycle: active
- Implementation: 已完成，随 PR #32 落地

## 背景 / 问题陈述

IsolaRail 需要一组可直接用于 README、GitHub repository social card、后续 HTML 海报和项目宣传材料的品牌资产。资产必须让主人能快速判断“哪个是最终成品、哪个是变体、哪个只是后续布局素材”，并且在 spec 内可直接预览。

## 目标 / 非目标

### Goals

- 固定 IsolaRail 当前选定的 Logo、App Icon、海报和 GitHub Social preview。
- GitHub Social preview 必须有一个主图和一个变体，并都保存在项目内。
- 保留后续 HTML 海报可复用的产品场景素材。
- 在本 spec 内完整列出所有品牌资产、用途、尺寸、文件大小和预览图。

### Non-goals

- 不重新打开 Logo 方向选择。
- 不在本次实现 HTML 海报页面或模板。
- 不把产品场景素材误标为最终海报。
- 不用当前生成图替代硬件事实或硬件规格文档。

## 范围

### In scope

- `docs/assets/brand/**` 品牌图片资产。
- README 中品牌资产入口。
- 本 spec 的资产清单和视觉预览。

### Out of scope

- Firmware source behavior changes.
- Hardware wiring / BOM / netlist changes.
- GitHub repository settings 中实际上传 social preview 的 owner 侧操作。

## 需求

### MUST

- `isolarail-logo-lockup.png` 是当前主 Logo。
- `isolarail-app-icon.png` 是当前 App Icon。
- `isolarail-poster.png` 是当前单张最终海报。
- `isolarail-github-social-preview.png` 是 GitHub Social preview 主图。
- `isolarail-github-social-preview-variant.png` 是 GitHub Social preview 变体。
- 两张 GitHub Social preview 必须保持 2:1 附近画幅，并控制在 GitHub 上传限制内。
- 产品图形不得出现明显违背常识的形体错误，例如楔形尾部、三角侧板、端口数量异常或不可能透视。
- 产品场景素材必须保留为后续 HTML 海报布局输入，但不得冒充最终海报。

### SHOULD

- 资产文件名应体现用途，避免 `source` / `layout` 等含糊名称进入最终成品清单。
- SPEC 和 `docs/assets/brand/README.md` 应保持用途说明一致。
- README 只暴露最常用的成品入口，详细预览以本 spec 为准。

## 资产清单

| Asset | Role | Dimensions | Size | Usage |
| --- | --- | --- | --- | --- |
| `docs/assets/brand/isolarail-logo-lockup.png` | Logo 主锁定图 | 410 x 226 | 49,084 bytes | README、文档站、品牌露出 |
| `docs/assets/brand/isolarail-app-icon.png` | App Icon | 1024 x 1024 | 45,171 bytes | App 图标、头像、方形入口 |
| `docs/assets/brand/isolarail-poster.png` | 最终海报 | 1024 x 1536 | 2,143,451 bytes | 单张品牌海报 |
| `docs/assets/brand/isolarail-github-social-preview.png` | GitHub Social preview 主图 | 1774 x 887 | 147,506 bytes | Repository social card 主图 |
| `docs/assets/brand/isolarail-github-social-preview-variant.png` | GitHub Social preview 变体 | 1774 x 887 | 379,129 bytes | 备用 social card / campaign variant |
| `docs/assets/brand/isolarail-product-scene-portrait.png` | HTML 海报产品场景素材 | 1024 x 1536 | 2,118,932 bytes | 后续可复制 HTML 海报布局输入 |
| `docs/assets/brand/isolarail-product-scene-wide.png` | HTML 海报宽幅产品场景素材 | 1774 x 887 | 1,847,567 bytes | 后续可复制 HTML social/hero 布局输入 |
| `docs/assets/brand/isolarail-logo-proposal-sheet.png` | Logo 方向来源图 | 1536 x 1024 | 1,050,040 bytes | 记录已选 Logo 来源方向 |

## 功能与行为规格

- GitHub Social preview 主图采用更扁平、正面/轻微俯视的产品表达，优先满足 GitHub 上传体积和远距离可读性。
- GitHub Social preview 变体采用更写实、斜向产品表达，优先满足 campaign preview 的质感和空间感。
- 海报是带 Logo、标题层级、产品主视觉和底部信息的最终海报，不是单纯产品场景图。
- 产品场景素材只作为后续 HTML 海报或 social layout 的输入图，不作为最终成品露出。

## 验收标准

- Given 品牌资产被合入仓库，When 阅读 `docs/assets/brand/README.md`，Then 每个文件用途清晰且没有含糊的 social layout source 命名。
- Given 阅读本 spec，When 查看 `## Visual Evidence`，Then 能直接预览 Logo、App Icon、海报、GitHub Social preview 主图、变体和产品场景素材。
- Given 上传 GitHub Social preview，When 使用 `isolarail-github-social-preview.png`，Then 文件大小低于 GitHub social preview 上传限制。
- Given 需要替换 social preview，When 使用 `isolarail-github-social-preview-variant.png`，Then 该变体同样低于 GitHub social preview 上传限制。
- Given 主人检查产品形体，When 查看两张 social preview，Then 产品外壳为常识正确的矩形盒体，不出现明显楔形尾部或不可能透视。

## 非功能性验收 / 质量门槛

### Testing

- `bunx markdownlint-cli2 README.md docs/assets/brand/README.md docs/specs/README.md docs/specs/b8r3n-brand-visual-assets/SPEC.md docs/specs/b8r3n-brand-visual-assets/IMPLEMENTATION.md docs/specs/b8r3n-brand-visual-assets/HISTORY.md`
- `file docs/assets/brand/*.png`
- `cargo +esp check`
- `cargo +esp build --release`

### Visual Evidence

Logo 主锁定图：

![IsolaRail logo lockup](../../assets/brand/isolarail-logo-lockup.png)

App Icon：

![IsolaRail app icon](../../assets/brand/isolarail-app-icon.png)

最终海报：

![IsolaRail final poster](../../assets/brand/isolarail-poster.png)

GitHub Social preview 主图：

![IsolaRail GitHub social preview main](../../assets/brand/isolarail-github-social-preview.png)

GitHub Social preview 变体：

![IsolaRail GitHub social preview variant](../../assets/brand/isolarail-github-social-preview-variant.png)

HTML 海报产品场景素材：

![IsolaRail product scene portrait](../../assets/brand/isolarail-product-scene-portrait.png)

HTML social/hero 宽幅产品场景素材：

![IsolaRail product scene wide](../../assets/brand/isolarail-product-scene-wide.png)

Logo 方向来源图：

![IsolaRail logo proposal sheet](../../assets/brand/isolarail-logo-proposal-sheet.png)

## 文档更新

- `README.md`
- `docs/assets/brand/README.md`
- `docs/specs/README.md`
- `docs/specs/b8r3n-brand-visual-assets/SPEC.md`

## 风险 / 开放问题 / 假设

- GitHub repository social preview 的实际上传需要 owner 在 GitHub repository settings 中执行。
- 后续 HTML 海报应复用产品场景素材，但应由 HTML/CSS 实现文字和布局，避免继续依赖不可编辑的 raster 海报文本。
