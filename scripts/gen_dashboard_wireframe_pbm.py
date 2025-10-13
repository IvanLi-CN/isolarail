#!/usr/bin/env python3
W, H = 160, 50

# 0 = white, 1 = black
buf = [[0 for _ in range(W)] for _ in range(H)]

def set_px(x, y, v=1):
    if 0 <= x < W and 0 <= y < H:
        buf[y][x] = 1 if v else 0

def hline(x0, x1, y, v=1):
    if y < 0 or y >= H: return
    if x0 > x1: x0, x1 = x1, x0
    x0 = max(0, x0); x1 = min(W-1, x1)
    for x in range(x0, x1+1):
        set_px(x, y, v)

def vline(x, y0, y1, v=1):
    if x < 0 or x >= W: return
    if y0 > y1: y0, y1 = y1, y0
    y0 = max(0, y0); y1 = min(H-1, y1)
    for y in range(y0, y1+1):
        set_px(x, y, v)

def rect(x, y, w, h, v=1):
    hline(x, x+w-1, y, v)
    hline(x, x+w-1, y+h-1, v)
    vline(x, y, y+h-1, v)
    vline(x+w-1, y, y+h-1, v)

# Minimal 5x7 font, columns are 5, rows are 7; '#' = on, '.' = off
FONT_5x7 = {
    '0': [
        ".###.",
        "#...#",
        "#..##",
        "#.#.#",
        "##..#",
        "#...#",
        ".###.",
    ],
    '1': [
        "..#..",
        ".##..",
        "..#..",
        "..#..",
        "..#..",
        "..#..",
        ".###.",
    ],
    '2': [
        ".###.",
        "#...#",
        "....#",
        "..##.",
        ".#...",
        "#....",
        "#####",
    ],
    '3': [
        "#####",
        "....#",
        "...#.",
        "..##.",
        "....#",
        "#...#",
        ".###.",
    ],
    '4': [
        "...#.",
        "..##.",
        ".#.#.",
        "#..#.",
        "#####",
        "...#.",
        "...#.",
    ],
    '5': [
        "#####",
        "#....",
        "####.",
        "....#",
        "....#",
        "#...#",
        ".###.",
    ],
    '6': [
        ".###.",
        "#...",
        "#....",
        "####.",
        "#...#",
        "#...#",
        ".###.",
    ],
    '7': [
        "#####",
        "....#",
        "...#.",
        "..#..",
        ".#...",
        ".#...",
        ".#...",
    ],
    '8': [
        ".###.",
        "#...#",
        "#...#",
        ".###.",
        "#...#",
        "#...#",
        ".###.",
    ],
    '9': [
        ".###.",
        "#...#",
        "#...#",
        ".####",
        "....#",
        "...#.",
        ".##..",
    ],
    '.': [
        ".....",
        ".....",
        ".....",
        ".....",
        ".....",
        "..#..",
        "..#..",
    ],
    '-': [
        ".....",
        ".....",
        ".###.",
        ".###.",
        ".....",
        ".....",
        ".....",
    ],
    'V': [
        "#...#",
        "#...#",
        "#...#",
        "#...#",
        ".#.#.",
        ".#.#.",
        "..#..",
    ],
    'A': [
        ".###.",
        "#...#",
        "#...#",
        "#####",
        "#...#",
        "#...#",
        "#...#",
    ],
    'W': [
        "#...#",
        "#...#",
        "#...#",
        "#.#.#",
        "#.#.#",
        "##.##",
        "#...#",
    ],
    'C': [
        ".###.",
        "#...#",
        "#....",
        "#....",
        "#....",
        "#...#",
        ".###.",
    ],
    ' ': [
        ".....",
        ".....",
        ".....",
        ".....",
        ".....",
        ".....",
        ".....",
    ],
}

ADV_X = 6  # horizontal advance: keep width budget (<=6 glyphs per 36 px)

# Scale 5x7 glyphs vertically to 5x9 (nearest-neighbor along Y) without changing width.
ORIG_H = 7
TARGET_H = 9

def _scale_rows_5x7_to_5x9(rows):
    out = []
    for oy in range(TARGET_H):
        iy = (oy * ORIG_H) // TARGET_H
        iy = min(iy, ORIG_H - 1)
        out.append(rows[iy])
    return out

def _glyph_5x9(ch):
    base = FONT_5x7.get(ch, FONT_5x7[' '])
    return _scale_rows_5x7_to_5x9(base)

def draw_char(x, y, ch):
    g = _glyph_5x9(ch)
    for dy, row in enumerate(g):
        for dx, c in enumerate(row):
            if c == '#':
                set_px(x+dx, y+dy, 1)

def text_width(s):
    if not s:
        return 0
    # last character without trailing spacing
    return len(s) * ADV_X - 1

def draw_text_center(cx, y, s):
    w = text_width(s)
    x = int(cx - w/2)
    for i, ch in enumerate(s):
        draw_char(x + i*ADV_X, y, ch)

# 1) outer border
rect(0, 0, W, H, 1)

# 2) vertical separators
for x in (40, 80, 120):
    vline(x, 0, H-1, 1)

# 3) header labels C1..C4 (use 5x9; place in 0..9 area)
centers = [20, 60, 100, 140]
for i, cx in enumerate(centers, start=1):
    draw_text_center(cx, 1, f"C{i}")

# 4) sample values per column
# With 5x9 glyphs in 6x10 cells, place three rows at 11,22,33
rows_y = [11, 22, 33]  # V, I, W text top positions
samples = [
    ("5.12V", "0.98A", "4.9W"),
    ("9.00V", "2.50A", "22.5W"),
    ("20.0V", "1.50A", "13.0W"),
    ("--",    "0.00A", "0W"),
]

for col, cx in enumerate(centers):
    v, a, wv = samples[col]
    draw_text_center(cx, rows_y[0], v)
    draw_text_center(cx, rows_y[1], a)
    draw_text_center(cx, rows_y[2], wv)

# 5) power bars (outline + fill example)
bar_y = 44
bar_h = 4
bar_w = 34
bar_xs = [3, 43, 83, 123]
fill_w = [20, 30, 15, 0]

for bx, fw in zip(bar_xs, fill_w):
    rect(bx, bar_y, bar_w, bar_h, 1)
    if fw > 0:
        # fill inside (avoid overwriting border)
        for y in range(bar_y+1, bar_y+bar_h-1):
            for x in range(bx+1, bx+1+max(0, fw-1)):
                set_px(x, y, 1)

# Write PBM (P1, ascii)
with open('docs/ui/dashboard_wireframe_160x50.pbm', 'w') as f:
    f.write('P1\n')
    f.write(f"{W} {H}\n")
    for y in range(H):
        line = ' '.join('1' if buf[y][x] else '0' for x in range(W))
        f.write(line + '\n')

print('Wrote docs/ui/dashboard_wireframe_160x50.pbm')
