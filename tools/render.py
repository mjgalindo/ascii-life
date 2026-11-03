#!/usr/bin/env python3
"""Render a cell dump (from `OUT=file ascii-zoo --dump ...`) to a PNG.

usage: render.py cells.txt out.png [cell_w cell_h]
"""
import sys
from PIL import Image, ImageDraw, ImageFont

FONTS = [
    "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
]


def load_font(size):
    for f in FONTS:
        try:
            return ImageFont.truetype(f, size)
        except OSError:
            continue
    return ImageFont.load_default()


def rgb(s):
    return tuple(int(v) for v in s.split(","))


def main():
    src, dst = sys.argv[1], sys.argv[2]
    cw = int(sys.argv[3]) if len(sys.argv) > 3 else 9
    ch = int(sys.argv[4]) if len(sys.argv) > 4 else 18
    font = load_font(ch - 3)
    with open(src, encoding="utf-8") as fh:
        cols, rows = map(int, fh.readline().split())
        lines = fh.read().split("\n")
    img = Image.new("RGB", (cols * cw, rows * ch))
    d = ImageDraw.Draw(img)
    for i in range(cols * rows):
        parts = lines[i].split("\t")
        if len(parts) != 3:
            continue
        sym, fg, bg = parts[0], rgb(parts[1]), rgb(parts[2])
        x, y = (i % cols) * cw, (i // cols) * ch
        if sym == "▀":
            d.rectangle([x, y, x + cw - 1, y + ch // 2 - 1], fill=fg)
            d.rectangle([x, y + ch // 2, x + cw - 1, y + ch - 1], fill=bg)
        else:
            d.rectangle([x, y, x + cw - 1, y + ch - 1], fill=bg)
            if sym.strip():
                d.text((x, y), sym, font=font, fill=fg)
    img.save(dst)


main()
