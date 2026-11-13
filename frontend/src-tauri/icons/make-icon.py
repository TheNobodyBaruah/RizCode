#!/usr/bin/env python3
"""Generates the application source icon (1024x1024 PNG) without any image library.

The icon is a dark rounded square with a green terminal prompt — the visual language of the
app itself. It is used as the input of `tauri icon`, which produces every platform specific
size that the bundler needs.
"""

import struct
import zlib
from pathlib import Path

SIZE = 1024
BG = (18, 21, 28, 255)
FG = (74, 222, 128, 255)
DIM = (63, 71, 86, 255)
ACCENT = (56, 189, 248, 255)


def blank():
    return [[(0, 0, 0, 0) for _ in range(SIZE)] for _ in range(SIZE)]


def rounded_rect(pixels, x0, y0, x1, y1, radius, color):
    for y in range(max(0, y0), min(SIZE, y1)):
        for x in range(max(0, x0), min(SIZE, x1)):
            dx = 0
            dy = 0
            if x < x0 + radius:
                dx = x0 + radius - x
            elif x >= x1 - radius:
                dx = x - (x1 - radius - 1)
            if y < y0 + radius:
                dy = y0 + radius - y
            elif y >= y1 - radius:
                dy = y - (y1 - radius - 1)
            if dx * dx + dy * dy <= radius * radius:
                pixels[y][x] = color


def line(pixels, x0, y0, x1, y1, thickness, color):
    """Draws a straight line using a distance-to-segment test."""
    vx, vy = x1 - x0, y1 - y0
    length_sq = vx * vx + vy * vy
    half = thickness / 2.0
    for y in range(max(0, int(min(y0, y1) - thickness)), min(SIZE, int(max(y0, y1) + thickness) + 1)):
        for x in range(max(0, int(min(x0, x1) - thickness)), min(SIZE, int(max(x0, x1) + thickness) + 1)):
            if length_sq == 0:
                distance_sq = (x - x0) ** 2 + (y - y0) ** 2
            else:
                t = ((x - x0) * vx + (y - y0) * vy) / length_sq
                t = max(0.0, min(1.0, t))
                px, py = x0 + t * vx, y0 + t * vy
                distance_sq = (x - px) ** 2 + (y - py) ** 2
            if distance_sq <= half * half:
                pixels[y][x] = color


def write_png(pixels, path):
    raw = bytearray()
    for row in pixels:
        raw.append(0)  # filter type: none
        for pixel in row:
            raw.extend(pixel)

    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )
    Path(path).write_bytes(png)


def main():
    pixels = blank()

    # Background plate.
    rounded_rect(pixels, 64, 64, 960, 960, 180, BG)

    # Terminal chevron ">".
    line(pixels, 300, 360, 470, 512, 62, FG)
    line(pixels, 470, 512, 300, 664, 62, FG)

    # Cursor underscore "_".
    rounded_rect(pixels, 530, 630, 740, 690, 28, FG)

    # Decorative code lines on the right.
    rounded_rect(pixels, 790, 330, 880, 372, 21, ACCENT)
    rounded_rect(pixels, 790, 450, 880, 492, 21, DIM)
    rounded_rect(pixels, 790, 570, 880, 612, 21, DIM)

    target = Path(__file__).resolve().parent / "app-icon.png"
    write_png(pixels, target)
    print(f"wrote {target}")


if __name__ == "__main__":
    main()
