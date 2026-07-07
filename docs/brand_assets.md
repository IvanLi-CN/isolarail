# IsolaRail Brand Assets

This page records the canonical brand assets used by the repository and the
published documentation site.

## Source and Regeneration

The assets are generated from `scripts/generate_brand_assets.py`:

```bash
python3 scripts/generate_brand_assets.py
```

The script writes canonical source assets to `docs/assets/brand/` and copies
site-ready assets to `docs-site/docs/public/brand/`.

## Delivered Assets

- Logo: `docs/assets/brand/isolarail-logo.svg` and `.png`
- Brand mark: `docs/assets/brand/isolarail-mark.svg` and `.png`
- App icon: `docs/assets/brand/isolarail-app-icon.svg`, `.png`, `isolarail-app-icon-512.png`, and `apple-touch-icon.png`
- Poster: `docs/assets/brand/isolarail-poster.svg` and `.png`
- GitHub social preview: `docs/assets/brand/isolarail-social-preview.svg` and `.png`

## Site Integration

`docs-site/rspress.config.ts` uses the generated public assets for the nav logo,
favicon, Apple touch icon, web manifest, Open Graph image, and Twitter preview.
The GitHub social preview image is `1280x640` and is intended for both repository
social preview upload and docs-site metadata.

Set `DOCS_SITE_URL` during docs builds if the deployed site is not hosted at
`https://ivanli-cn.github.io`:

```bash
DOCS_SITE_URL=https://docs.example.com DOCS_BASE=/ bun run docs:build
```
