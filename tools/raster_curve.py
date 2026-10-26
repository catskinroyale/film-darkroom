"""Trace one characteristic curve from a raster plot embedded in a datasheet PDF.

Ilford's datasheets embed their curves as images. For a plot holding one thick curve on a
regular grid (HP5 Plus, FP4 Plus, Delta 400 …) the curve can still be traced reliably:
grid lines are found as near-full-length dark rows and columns, the axes are calibrated
from two grid lines per axis, and in each column the curve is the centre of the thickest
dark run away from the grid lines.

    python3 -I tools/raster_curve.py SHEET.pdf --page 5 --grid        # list grid lines
    python3 -I tools/raster_curve.py SHEET.pdf --page 5 \\
        --x-lines 0:0,8:4 --y-lines 6:0,0:3 --out curve.csv --png check.png

`--x-lines`/`--y-lines` map grid-line indices (as listed by `--grid`, counted from the
left and from the top) to axis values. `--png` writes the image with the traced points
drawn in red, to check by eye. Requires PyMuPDF and numpy.
"""

import argparse
import sys

try:
    import numpy as np
    import pymupdf
except ImportError:
    sys.exit("PyMuPDF and numpy required: pip install pymupdf numpy")

DARK = 110  # grey level below which a pixel is ink


def load_image(path, page_no, index, clip=None, dpi=400):
    doc = pymupdf.open(path)
    if clip:
        # Render a region of the page: for plots split into many image strips.
        rect = pymupdf.Rect(*map(float, clip.split(",")))
        pix = doc[page_no - 1].get_pixmap(dpi=dpi, clip=rect, colorspace=pymupdf.csGRAY)
    else:
        pix = pymupdf.Pixmap(doc, doc[page_no - 1].get_images()[index][0])
    if pix.n - pix.alpha != 1:
        pix = pymupdf.Pixmap(pymupdf.csGRAY, pix)
    a = np.frombuffer(pix.samples, dtype=np.uint8).reshape(pix.height, pix.width, pix.n)[:, :, 0]
    return a.astype(int)


def grid_lines(a, axis):
    """Centres of rows (axis=1) or columns (axis=0) that are dark over most of the image,
    merged where adjacent."""
    return [sum(g) / len(g) for g in grid_groups(a, axis)]


def grid_thickness(a):
    """Thickest grid line, in pixels."""
    return max(len(g) for axis in (0, 1) for g in grid_groups(a, axis))


def grid_groups(a, axis):
    frac = (a < 200).mean(axis=axis)
    hits = [i for i, v in enumerate(frac) if v > 0.5]
    lines = []
    for i in hits:
        if lines and i - lines[-1][-1] <= 2:
            lines[-1].append(i)
        else:
            lines.append([i])
    return lines


def calibrate(spec, lines):
    (i0, v0), (i1, v1) = [(int(a), float(b)) for a, b in (p.split(":") for p in spec.split(","))]
    p0, p1 = lines[i0], lines[i1]
    return lambda px: v0 + (px - p0) * (v1 - v0) / (p1 - p0)


def trace(a, cols, rows, thick):
    near = lambda x, lines, r=2.5: any(abs(x - l) <= r for l in lines)
    points = []
    # Only inside the plot frame: the outer grid lines. Axis labels lie outside it.
    top, bottom = int(rows[0]) + 2, int(rows[-1]) + 3
    for x in range(int(cols[0]) + 3, int(cols[-1]) - 2):
        if near(x, cols, thick + 1.0):
            continue  # a vertical grid line hides the curve here
        ink = np.zeros(a.shape[0], dtype=bool)
        ink[top:bottom] = a[top:bottom, x] < DARK
        runs, start = [], None
        for y, on in enumerate(ink):
            if on and start is None:
                start = y
            if (not on or y == len(ink) - 1) and start is not None:
                end = y if not on else y + 1
                mid = (start + end - 1) / 2
                # a run no thicker than a grid line, lying on one, is the grid
                if end - start > thick + 1 or not near(mid, rows, thick / 2 + 1):
                    runs.append((end - start, mid))
                start = None
        if runs:
            width, mid = max(runs)
            if width >= 2:
                points.append((x, mid))
    return points


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("pdf")
    ap.add_argument("--page", type=int, required=True)
    ap.add_argument("--image", type=int, default=0, help="which image on the page")
    ap.add_argument("--clip", help="render this page region (x0,y0,x1,y1 points) instead of reading an image")
    ap.add_argument("--grid", action="store_true", help="list the grid lines and stop")
    ap.add_argument("--x-lines")
    ap.add_argument("--y-lines")
    ap.add_argument("--out")
    ap.add_argument("--png")
    ap.add_argument("--columns", default="log_h,density")
    args = ap.parse_args()

    a = load_image(args.pdf, args.page, args.image, args.clip)
    cols, rows = grid_lines(a, 0), grid_lines(a, 1)
    if args.grid or not (args.x_lines and args.y_lines and args.out):
        print(f"image {a.shape[1]}×{a.shape[0]} px")
        print("vertical lines (x):", ", ".join(f"{i}:{c:.1f}" for i, c in enumerate(cols)))
        print("horizontal lines (y):", ", ".join(f"{i}:{r:.1f}" for i, r in enumerate(rows)))
        return
    fx, fy = calibrate(args.x_lines, cols), calibrate(args.y_lines, rows)
    points = trace(a, cols, rows, grid_thickness(a))
    data = sorted((round(fx(x), 4), round(fy(y), 4)) for x, y in points)
    with open(args.out, "w") as f:
        f.write(f"{args.columns}\n")
        f.writelines(f"{x:.4f},{y:.4f}\n" for x, y in data)
    print(f"{len(data)} points, x {data[0][0]:.2f}..{data[-1][0]:.2f}, y {data[0][1]:.3f}..{data[-1][1]:.3f}")
    if args.png:
        rgb = np.stack([a, a, a], axis=-1).astype(np.uint8)
        for x, y in points:
            yi = int(round(y))
            rgb[max(yi - 1, 0) : yi + 2, x] = (230, 30, 30)
        pymupdf.Pixmap(pymupdf.csRGB, a.shape[1], a.shape[0], rgb.tobytes(), False).save(args.png)


if __name__ == "__main__":
    main()
