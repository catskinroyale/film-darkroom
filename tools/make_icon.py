#!/usr/bin/env python3
"""Draw the Film Darkroom app icon (1024 x 1024 PNG) with no dependencies.

A strip of 35 mm film lying across a lightbox, its sprocket holes lit from below. The
strip holds two frames of the same scene, a sunset over hills: one in colour, one in
black and white, the two processes the app prints.
Usage: python3 -I tools/make_icon.py packaging/macos/icon_1024.png
"""
import math
import struct
import sys
import zlib

N = 1024
ANGLE = math.radians(-10)  # the strip's tilt


def png(path, rows):
    raw = b"".join(b"\x00" + bytes(r) for r in rows)

    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    data = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
    open(path, "wb").write(data)


def coverage(d):
    """Antialiased edge from a signed distance in pixels: 1 inside, 0 outside."""
    return max(0.0, min(1.0, 0.5 - d))


def rounded_box(x, y, hw, hh, r):
    """Signed distance to a rounded rectangle centred on the origin."""
    qx, qy = abs(x) - hw + r, abs(y) - hh + r
    return math.hypot(max(qx, 0.0), max(qy, 0.0)) + min(max(qx, qy), 0.0) - r


def mix(a, b, t):
    return [a[i] + (b[i] - a[i]) * t for i in range(3)]


# The strip, in its own coordinates (u along, v across, origin at the icon's centre).
STRIP_HH = 228  # half height
HOLE_V = STRIP_HH - 38  # sprocket rows
HOLE_HW, HOLE_HH, HOLE_PITCH = 15, 19, 60
FRAME_HW, FRAME_HH, FRAME_U = 168, 140, 182  # two frames at u = ±FRAME_U


def scene(fx, fy, px):
    """The frame's picture at (fx, fy) in 0..1 (y down); `px` is one pixel in frame units."""
    sky = mix([46, 74, 150], [252, 150, 82], max(0.0, min(1.0, fy / 0.68)) ** 1.4)
    c = sky
    # the sun, half set behind the far hills
    sun = coverage((math.hypot(fx - 0.64, fy - 0.56) - 0.12) / px)
    c = mix(c, [255, 226, 150], sun)
    far = 0.60 + 0.05 * math.sin(fx * 7.0 + 0.6) + 0.025 * math.sin(fx * 17.0)
    c = mix(c, [120, 70, 112], coverage((far - fy) / px))
    near = 0.74 + 0.07 * math.sin(fx * 4.2 + 2.2)
    c = mix(c, [32, 58, 60], coverage((near - fy) / px))
    return c


def grey(c):
    y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    return [y, y, y]


def pixel(x, y):
    cx = cy = N / 2
    # lightbox: a warm white rounded square with a soft glow towards the centre
    margin, corner = 100, 185
    half = N / 2 - margin
    box = coverage(rounded_box(x - cx, y - cy, half, half, corner))
    glow = 1 - 0.10 * (math.hypot(x - cx, y - cy) / half) ** 2
    light = [248 * glow, 244 * glow, 234 * glow]
    # into the strip's coordinates
    dx, dy = x - cx, y - cy
    u = dx * math.cos(ANGLE) + dy * math.sin(ANGLE)
    v = -dx * math.sin(ANGLE) + dy * math.cos(ANGLE)
    c = light
    # film base, with sprocket holes punched through to the light
    strip = coverage(abs(v) - STRIP_HH)
    if strip > 0:
        base = [24, 21, 19]
        k = round(u / HOLE_PITCH)
        hole = coverage(rounded_box(u - k * HOLE_PITCH, abs(v) - HOLE_V, HOLE_HW, HOLE_HH, 6))
        film = mix(base, light, hole)
        # the two frames
        for side in (-1, 1):
            fu, fv = u - side * FRAME_U, v
            inside = coverage(rounded_box(fu, fv, FRAME_HW, FRAME_HH, 10))
            if inside > 0:
                fx = (fu + FRAME_HW) / (2 * FRAME_HW)
                fy = (fv + FRAME_HH) / (2 * FRAME_HH)
                pic = scene(fx, fy, 1 / (2 * FRAME_HH))
                if side > 0:
                    pic = grey(pic)
                film = mix(film, pic, inside)
        c = mix(c, film, strip)
    # a faint shadow along the strip's edges keeps it off the light
    shade = 0.10 * coverage((abs(v) - STRIP_HH - 6) / 6) * (1 - strip)
    c = [ch * (1 - shade) for ch in c]
    return [int(max(0, min(255, ch))) for ch in c] + [int(255 * box)]


def main(path):
    rows = []
    for y in range(N):
        row = []
        for x in range(N):
            row += pixel(x + 0.5, y + 0.5)
        rows.append(row)
    png(path, rows)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "packaging/macos/icon_1024.png")
