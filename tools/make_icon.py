#!/usr/bin/env python3
"""Draw the Film Darkroom app icon (1024 x 1024 PNG) with no dependencies.

Three overlapping dye discs (cyan, magenta, yellow), like the dye layers of colour film,
over white inside a ringed circle on a dark rounded square. The right half of the circle
shows the same discs in black and white: one picture, either process.
Usage: python3 -I tools/make_icon.py packaging/macos/icon_1024.png
"""
import math
import struct
import sys
import zlib

N = 1024


def png(path, rows):
    raw = b"".join(b"\x00" + bytes(r) for r in rows)
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    open(path, "wb").write(data)


def coverage(d):
    """Antialiased edge: 1 inside (d < -0.5), 0 outside."""
    return max(0.0, min(1.0, 0.5 - d))


# Disc radius, and each disc's centre offset and absorption of R, G, B (cyan, magenta, yellow).
DISC = 175
DYES = [
    (0, -105, (0.92, 0.18, 0.05)),
    (-92, 55, (0.10, 0.90, 0.25)),
    (92, 55, (0.02, 0.10, 0.92)),
]


def main(path):
    margin, radius_corner = 100, 185
    cx = cy = N / 2
    r_outer, ring = 330, 14
    rows = []
    for y in range(N):
        row = []
        for x in range(N):
            # rounded-square background (macOS icon grid leaves a margin)
            qx = max(abs(x + 0.5 - cx) - (N / 2 - margin - radius_corner), 0)
            qy = max(abs(y + 0.5 - cy) - (N / 2 - margin - radius_corner), 0)
            bg_a = coverage(math.hypot(qx, qy) - radius_corner)
            r = math.hypot(x + 0.5 - cx, y + 0.5 - cy)
            # background: near-black with a faint vertical gradient
            v = 22 + 18 * (1 - y / N)
            # subtractive dye discs over white, clipped to the circle
            inside = coverage(r - (r_outer - ring))
            t = [1.0, 1.0, 1.0]
            for k, (dx, dy, absorb) in enumerate(DYES):
                d = coverage(math.hypot(x + 0.5 - cx - dx, y + 0.5 - cy - dy) - DISC)
                for c in range(3):
                    t[c] *= 1 - d * absorb[c]
            # right half: the same discs as greys (Rec. 709 luminance)
            mono = coverage(cx - (x + 0.5))
            grey = 0.2126 * t[0] + 0.7152 * t[1] + 0.0722 * t[2]
            t = [t[c] * (1 - mono) + grey * mono for c in range(3)]
            rgb = [v * (1 - inside) + 255 * t[c] * inside for c in range(3)]
            # thin light ring around the discs
            ring_a = coverage(abs(r - (r_outer - ring / 2)) - ring / 2)
            rgb = [c * (1 - ring_a) + 235 * ring_a for c in rgb]
            px = [int(max(0, min(255, c))) for c in rgb]
            row += px + [int(255 * bg_a)]
        rows.append(row)
    png(path, rows)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "packaging/macos/icon_1024.png")
