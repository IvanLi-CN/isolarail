#!/usr/bin/env python3
"""Generate IsolaRail brand SVG and PNG assets."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BRAND_DIR = ROOT / "docs" / "assets" / "brand"
PUBLIC_DIR = ROOT / "docs-site" / "docs" / "public" / "brand"


def write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content.strip() + "\n", encoding="utf-8")


def convert_svg(svg_path: Path, png_path: Path, width: int, height: int) -> None:
    png_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "rsvg-convert",
            "--width",
            str(width),
            "--height",
            str(height),
            "--format",
            "png",
            "--output",
            str(png_path),
            str(svg_path),
        ],
        check=True,
    )


def copy_public(name: str) -> None:
    PUBLIC_DIR.mkdir(parents=True, exist_ok=True)
    shutil.copy2(BRAND_DIR / name, PUBLIC_DIR / name)


def mark_svg(size: int = 1024, app: bool = False) -> str:
    bg = """
  <rect width="1024" height="1024" rx="232" fill="url(#appBg)"/>
  <path d="M138 820C268 900 448 924 624 872C779 826 899 730 946 606C998 467 945 318 816 222C686 124 500 90 332 138C176 182 82 292 76 430C70 586 150 728 138 820Z" fill="#f7faf8" opacity=".08"/>
""" if app else """
  <rect width="1024" height="1024" rx="190" fill="none"/>
"""
    return f"""
<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 1024 1024" role="img" aria-labelledby="title desc">
  <title id="title">IsolaRail brand mark</title>
  <desc id="desc">Four isolated USB rails crossing a protected control core.</desc>
  <defs>
    <linearGradient id="appBg" x1="112" y1="86" x2="918" y2="936" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#17231f"/>
      <stop offset=".46" stop-color="#23423d"/>
      <stop offset="1" stop-color="#0d1110"/>
    </linearGradient>
    <linearGradient id="rail" x1="184" y1="252" x2="840" y2="772" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#50e0c2"/>
      <stop offset=".52" stop-color="#78f7ff"/>
      <stop offset="1" stop-color="#f6b94f"/>
    </linearGradient>
    <linearGradient id="shield" x1="310" y1="246" x2="732" y2="764" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#f9fff7"/>
      <stop offset=".55" stop-color="#e5fff8"/>
      <stop offset="1" stop-color="#b5dcd1"/>
    </linearGradient>
    <filter id="softShadow" x="-30%" y="-30%" width="160%" height="160%">
      <feDropShadow dx="0" dy="28" stdDeviation="32" flood-color="#07100e" flood-opacity=".38"/>
    </filter>
  </defs>
{bg}
  <g filter="url(#softShadow)">
    <path d="M244 270H596C707 270 797 360 797 471V754" fill="none" stroke="url(#rail)" stroke-width="72" stroke-linecap="round"/>
    <path d="M780 754H428C317 754 227 664 227 553V270" fill="none" stroke="url(#rail)" stroke-width="72" stroke-linecap="round"/>
    <path d="M321 345H555C632 345 694 407 694 484V679" fill="none" stroke="#f7faf8" stroke-width="20" stroke-linecap="round" opacity=".84"/>
    <path d="M704 679H469C392 679 330 617 330 540V345" fill="none" stroke="#17231f" stroke-width="16" stroke-linecap="round" opacity=".5"/>
    <path d="M512 232L690 334V544C690 665 617 740 512 792C407 740 334 665 334 544V334L512 232Z" fill="url(#shield)" stroke="#17231f" stroke-width="28" stroke-linejoin="round"/>
    <path d="M512 302L628 370V535C628 614 580 664 512 700C444 664 396 614 396 535V370L512 302Z" fill="#17231f"/>
    <path d="M466 382H558L510 492H604L454 652L504 532H420L466 382Z" fill="#f6b94f"/>
    <circle cx="244" cy="270" r="58" fill="#17231f" stroke="#50e0c2" stroke-width="32"/>
    <circle cx="780" cy="754" r="58" fill="#17231f" stroke="#f6b94f" stroke-width="32"/>
    <circle cx="227" cy="270" r="28" fill="#f7faf8"/>
    <circle cx="797" cy="754" r="28" fill="#f7faf8"/>
  </g>
</svg>
"""


def logo_svg() -> str:
    return f"""
<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="420" viewBox="0 0 1600 420" role="img" aria-labelledby="title desc">
  <title id="title">IsolaRail logo</title>
  <desc id="desc">IsolaRail wordmark with isolated USB rail brand mark.</desc>
  <defs>
    <linearGradient id="word" x1="454" y1="78" x2="1374" y2="336" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#17231f"/>
      <stop offset=".48" stop-color="#2a5e55"/>
      <stop offset="1" stop-color="#c88928"/>
    </linearGradient>
  </defs>
  <g transform="translate(48 38) scale(.335)">
    {mark_svg().split('<defs>')[1].split('</defs>')[0].join(['<defs>', '</defs>'])}
    <g transform="translate(0 0)">
      <path d="M244 270H596C707 270 797 360 797 471V754" fill="none" stroke="url(#rail)" stroke-width="72" stroke-linecap="round"/>
      <path d="M780 754H428C317 754 227 664 227 553V270" fill="none" stroke="url(#rail)" stroke-width="72" stroke-linecap="round"/>
      <path d="M512 232L690 334V544C690 665 617 740 512 792C407 740 334 665 334 544V334L512 232Z" fill="url(#shield)" stroke="#17231f" stroke-width="28" stroke-linejoin="round"/>
      <path d="M512 302L628 370V535C628 614 580 664 512 700C444 664 396 614 396 535V370L512 302Z" fill="#17231f"/>
      <path d="M466 382H558L510 492H604L454 652L504 532H420L466 382Z" fill="#f6b94f"/>
      <circle cx="244" cy="270" r="58" fill="#17231f" stroke="#50e0c2" stroke-width="32"/>
      <circle cx="780" cy="754" r="58" fill="#17231f" stroke="#f6b94f" stroke-width="32"/>
    </g>
  </g>
  <text x="440" y="230" font-family="Inter, IBM Plex Sans, Avenir Next, Helvetica, Arial, sans-serif" font-size="154" font-weight="800" fill="url(#word)">IsolaRail</text>
  <text x="448" y="302" font-family="Inter, IBM Plex Sans, Avenir Next, Helvetica, Arial, sans-serif" font-size="38" font-weight="650" fill="#43645e">isolated USB power control - four monitored rails</text>
</svg>
"""


def poster_svg() -> str:
    return """
<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="2400" viewBox="0 0 1600 2400" role="img" aria-labelledby="title desc">
  <title id="title">IsolaRail poster</title>
  <desc id="desc">Poster art for the IsolaRail isolated USB hub project.</desc>
  <defs>
    <linearGradient id="posterBg" x1="0" y1="0" x2="1600" y2="2400" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#f7faf8"/>
      <stop offset=".52" stop-color="#dff2eb"/>
      <stop offset="1" stop-color="#f3e4c7"/>
    </linearGradient>
    <linearGradient id="darkPanel" x1="204" y1="650" x2="1396" y2="1840" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#17231f"/>
      <stop offset=".56" stop-color="#24443f"/>
      <stop offset="1" stop-color="#0f1413"/>
    </linearGradient>
    <linearGradient id="railGrad" x1="245" y1="830" x2="1380" y2="1590" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#50e0c2"/>
      <stop offset=".44" stop-color="#78f7ff"/>
      <stop offset="1" stop-color="#f6b94f"/>
    </linearGradient>
  </defs>
  <rect width="1600" height="2400" fill="url(#posterBg)"/>
  <rect x="152" y="164" width="1296" height="2072" rx="58" fill="#fbfffc" stroke="#17231f" stroke-width="10"/>
  <text x="214" y="282" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="58" font-weight="800" fill="#17231f">ISOLATED USB HUB</text>
  <text x="214" y="452" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="176" font-weight="850" fill="#17231f">IsolaRail</text>
  <text x="220" y="532" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="40" font-weight="650" fill="#47645f">four controlled ports / measured power rails</text>
  <text x="220" y="588" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="40" font-weight="650" fill="#47645f">visible fault states</text>
  <rect x="214" y="642" width="1172" height="1136" rx="44" fill="url(#darkPanel)"/>
  <g opacity=".22" stroke="#f7faf8" stroke-width="2">
    <path d="M290 760H1320M290 910H1320M290 1060H1320M290 1210H1320M290 1360H1320M290 1510H1320M290 1660H1320"/>
    <path d="M370 720V1700M550 720V1700M730 720V1700M910 720V1700M1090 720V1700M1270 720V1700"/>
  </g>
  <path d="M326 856H982C1144 856 1274 986 1274 1148V1608" fill="none" stroke="url(#railGrad)" stroke-width="88" stroke-linecap="round"/>
  <path d="M1274 1608H618C456 1608 326 1478 326 1316V856" fill="none" stroke="url(#railGrad)" stroke-width="88" stroke-linecap="round"/>
  <path d="M804 750L1064 900V1210C1064 1390 956 1500 804 1578C652 1500 544 1390 544 1210V900L804 750Z" fill="#f7faf8" stroke="#17231f" stroke-width="32" stroke-linejoin="round"/>
  <path d="M804 858L968 954V1198C968 1312 898 1384 804 1434C710 1384 640 1312 640 1198V954L804 858Z" fill="#17231f"/>
  <path d="M742 982H870L804 1138H940L724 1372L792 1202H676L742 982Z" fill="#f6b94f"/>
  <g font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-weight="750">
    <rect x="292" y="1858" width="282" height="164" rx="26" fill="#17231f"/>
    <text x="332" y="1924" font-size="28" fill="#78f7ff">CONTROL</text>
    <text x="332" y="1984" font-size="42" fill="#f7faf8">ESP32-S3</text>
    <rect x="626" y="1858" width="282" height="164" rx="26" fill="#d4f4e9"/>
    <text x="666" y="1924" font-size="28" fill="#20463f">USB HUB</text>
    <text x="666" y="1984" font-size="42" fill="#17231f">CH335F</text>
    <rect x="960" y="1858" width="282" height="164" rx="26" fill="#f6d99b"/>
    <text x="1000" y="1924" font-size="28" fill="#5a4218">DISPLAY</text>
    <text x="1000" y="1984" font-size="42" fill="#17231f">160x50</text>
  </g>
  <text x="214" y="2128" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="40" font-weight="750" fill="#17231f">Hardware evidence and firmware contracts</text>
  <text x="214" y="2186" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="40" font-weight="750" fill="#17231f">stay connected.</text>
</svg>
"""


def social_svg() -> str:
    return """
<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="640" viewBox="0 0 1280 640" role="img" aria-labelledby="title desc">
  <title id="title">IsolaRail GitHub social preview</title>
  <desc id="desc">Social preview for the IsolaRail repository.</desc>
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1280" y2="640" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#f7faf8"/>
      <stop offset=".58" stop-color="#d7f1ea"/>
      <stop offset="1" stop-color="#f2d79b"/>
    </linearGradient>
    <linearGradient id="railGrad" x1="680" y1="170" x2="1160" y2="500" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#50e0c2"/>
      <stop offset=".5" stop-color="#78f7ff"/>
      <stop offset="1" stop-color="#f6b94f"/>
    </linearGradient>
  </defs>
  <rect width="1280" height="640" fill="url(#bg)"/>
  <rect x="58" y="58" width="1164" height="524" rx="34" fill="#fbfffc" stroke="#17231f" stroke-width="8"/>
  <text x="116" y="204" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="126" font-weight="850" fill="#17231f">IsolaRail</text>
  <text x="124" y="282" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="34" font-weight="700" fill="#42645e">isolated USB power control</text>
  <text x="124" y="332" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="34" font-weight="700" fill="#42645e">for bench bring-up</text>
  <text x="124" y="408" font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="28" font-weight="650" fill="#17231f">ESP32-S3 / CH335F / INA226 / 160x50 front panel</text>
  <g transform="translate(820 148) scale(.34)">
    <path d="M244 270H596C707 270 797 360 797 471V754" fill="none" stroke="url(#railGrad)" stroke-width="72" stroke-linecap="round"/>
    <path d="M780 754H428C317 754 227 664 227 553V270" fill="none" stroke="url(#railGrad)" stroke-width="72" stroke-linecap="round"/>
    <path d="M512 232L690 334V544C690 665 617 740 512 792C407 740 334 665 334 544V334L512 232Z" fill="#f7faf8" stroke="#17231f" stroke-width="28" stroke-linejoin="round"/>
    <path d="M512 302L628 370V535C628 614 580 664 512 700C444 664 396 614 396 535V370L512 302Z" fill="#17231f"/>
    <path d="M466 382H558L510 492H604L454 652L504 532H420L466 382Z" fill="#f6b94f"/>
    <circle cx="244" cy="270" r="58" fill="#17231f" stroke="#50e0c2" stroke-width="32"/>
    <circle cx="780" cy="754" r="58" fill="#17231f" stroke="#f6b94f" stroke-width="32"/>
  </g>
  <g font-family="Inter, IBM Plex Sans, Helvetica, Arial, sans-serif" font-size="24" font-weight="750">
    <rect x="124" y="472" width="154" height="58" rx="14" fill="#17231f"/>
    <text x="150" y="510" fill="#78f7ff">4 PORTS</text>
    <rect x="300" y="472" width="190" height="58" rx="14" fill="#d4f4e9"/>
    <text x="326" y="510" fill="#17231f">MEASURED</text>
    <rect x="512" y="472" width="154" height="58" rx="14" fill="#f6d99b"/>
    <text x="540" y="510" fill="#17231f">FAULT UI</text>
  </g>
</svg>
"""


def site_manifest() -> str:
    return """
{
  "name": "IsolaRail",
  "short_name": "IsolaRail",
  "description": "Product and engineering documentation for the IsolaRail isolated USB hub.",
  "icons": [
    {
      "src": "brand/isolarail-app-icon-512.png",
      "sizes": "512x512",
      "type": "image/png",
      "purpose": "any maskable"
    },
    {
      "src": "brand/isolarail-app-icon.png",
      "sizes": "1024x1024",
      "type": "image/png",
      "purpose": "any maskable"
    }
  ],
  "theme_color": "#17231f",
  "background_color": "#f7faf8",
  "display": "standalone"
}
"""


def main() -> None:
    if not shutil.which("rsvg-convert"):
        raise SystemExit("rsvg-convert is required to render PNG assets")

    BRAND_DIR.mkdir(parents=True, exist_ok=True)
    PUBLIC_DIR.mkdir(parents=True, exist_ok=True)

    files = {
        "isolarail-mark.svg": mark_svg(),
        "isolarail-app-icon.svg": mark_svg(app=True),
        "isolarail-logo.svg": logo_svg(),
        "isolarail-poster.svg": poster_svg(),
        "isolarail-social-preview.svg": social_svg(),
    }
    for name, content in files.items():
        write_text(BRAND_DIR / name, content)

    png_targets = [
        ("isolarail-logo.svg", "isolarail-logo.png", 1600, 420),
        ("isolarail-mark.svg", "isolarail-mark.png", 1024, 1024),
        ("isolarail-app-icon.svg", "isolarail-app-icon.png", 1024, 1024),
        ("isolarail-app-icon.svg", "isolarail-app-icon-512.png", 512, 512),
        ("isolarail-app-icon.svg", "apple-touch-icon.png", 180, 180),
        ("isolarail-poster.svg", "isolarail-poster.png", 1600, 2400),
        ("isolarail-social-preview.svg", "isolarail-social-preview.png", 1280, 640),
    ]
    for source, target, width, height in png_targets:
        convert_svg(BRAND_DIR / source, BRAND_DIR / target, width, height)

    for name in [*files.keys(), *(target for _, target, _, _ in png_targets)]:
        copy_public(name)

    write_text(PUBLIC_DIR.parent / "site.webmanifest", site_manifest())

    print(f"Generated brand assets in {BRAND_DIR}")
    print(f"Copied site assets into {PUBLIC_DIR}")


if __name__ == "__main__":
    main()
