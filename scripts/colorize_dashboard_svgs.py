#!/usr/bin/env python3
"""
Colorize existing per-pixel SVG wireframes for the 160x50 dashboard.

Inputs (must exist):
  - docs/assets/dashboard_wireframe_160x50.svg
  - docs/assets/dashboard_wireframe_160x50_disconnected.svg

Outputs (generated):
  - docs/assets/dashboard_wireframe_160x50_color.svg
  - docs/assets/dashboard_wireframe_160x50_disconnected_color.svg

Color policy (RGB565-friendly palette):
  - Background: #F7F7F7 (≈ 0xEF7D)
  - Border + separators + header labels: #000000 (0x0000)
  - Voltage row (y ∈ [11,19]): Yellow #FFCC00 (≈ 0xFF20)
  - Current row (y ∈ [22,30]): Red    #D32F2F (≈ 0xB0E9)
  - Power row   (y ∈ [33,41]): Green  #2E7D32 (≈ 0x23E6)
  - Power bars  (y ∈ [44,47]): Green  #2E7D32 (≈ 0x23E6)
  - Keep outer border (x=0,159 or y=0,49) and column separators (x in {40,80,120}) black for legibility.

All pixels remain 1×1 rectangles; only the fill color changes per semantic row.
"""

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "docs" / "assets"

INPUTS = [
    (ASSETS / "dashboard_wireframe_160x50.svg", ASSETS / "dashboard_wireframe_160x50_color.svg"),
    (ASSETS / "dashboard_wireframe_160x50_disconnected.svg", ASSETS / "dashboard_wireframe_160x50_disconnected_color.svg"),
]

# Palette
BG = "#F7F7F7"
BLACK = "#000000"
YELLOW = "#FFCC00"  # voltage
RED = "#D32F2F"     # current
GREEN = "#2E7D32"   # power text + bars


def classify_color(x: int, y: int) -> str:
    # Borders
    if x in (0, 159) or y in (0, 49):
        return BLACK
    # Column separators
    if x in (40, 80, 120):
        return BLACK
    # Power bars region (bottom stripes)
    if 44 <= y <= 47:
        return GREEN
    # Text rows (top to bottom)
    if 11 <= y <= 19:
        return YELLOW
    if 22 <= y <= 30:
        return RED
    if 33 <= y <= 41:
        return GREEN
    # Header & other misc remain black
    return BLACK


def colorize(src: Path, dst: Path) -> None:
    txt = src.read_text(encoding="utf-8")

    # Update the background rect fill to slightly gray white.
    # Replace background fill color; ensure quotes are not escaped.
    bg_re = r'(<rect\s+width="160"\s+height="50"\s+fill=)"#[0-9a-fA-F]{3,6}"(/?>)'
    txt = re.sub(bg_re, r'\1"%s"\2' % BG, txt, count=1)

    # Replace each pixel rect's fill according to y/x classification.
    # Example: <rect x="12" y="34" width="1" height="1" fill="#000"/>
    rect_re = re.compile(r"(<rect\s+x=\"(\d+)\"\s+y=\"(\d+)\"\s+width=\"1\"\s+height=\"1\"\s+fill=)\"#[0-9a-fA-F]{3,6}\"(/?>)")

    def repl(m: re.Match) -> str:
        x = int(m.group(2)); y = int(m.group(3))
        color = classify_color(x, y)
        return f"{m.group(1)}\"{color}\"{m.group(4)}"

    txt = rect_re.sub(repl, txt)
    dst.write_text(txt, encoding="utf-8")
    print(f"Wrote {dst}")


def main():
    for src, dst in INPUTS:
        if not src.exists():
            raise FileNotFoundError(f"Missing input asset: {src}")
        colorize(src, dst)


if __name__ == "__main__":
    main()
