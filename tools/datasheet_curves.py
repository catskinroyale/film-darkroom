"""Extract characteristic curves drawn as vector paths in a datasheet PDF.

Kodak's (and many Ilford) datasheets draw their curves as PDF vector paths, not images,
so the exact plotted coordinates can be read back: no hand digitising and no
digitising error beyond the plot's own line placement. This tool reads one plot.

Requires PyMuPDF (`pip install pymupdf`). Run it with `python3 -I`, and keep downloaded
PDFs in their own directory: they are untrusted input.

    python3 -I tools/datasheet_curves.py extract SHEET.pdf --page 8 \\
        --frame 81.4,63.5,265.8,248.0 --legend 92.0,119.8,175.1,156.5 \\
        --x=-4,1 --y=0,4 --out DIR  t12=16.2 t10=6.5 t08=3.2 t06=solid

    python3 -I tools/datasheet_curves.py inspect SHEET.pdf --page 8 --clip 0,30,310,310
    python3 -I tools/datasheet_curves.py overlay SHEET.pdf --page 8 --frame ... --x ... \\
        --y ... --png check.png DIR/t12.csv DIR/t10.csv ...

`--frame` is the plot's axis rectangle in PDF points (from `inspect`: the plot frame is
the thin rectangle whose corners sit at the first and last tick labels); `--x`/`--y` give
the data values at its left/right and bottom/top edges. Curves are told apart by their
dash pattern: `solid`, or the first dash length rounded to 0.1 pt; curves drawn alike
are picked by position instead, `@0`, `@1`, … in the PDF's drawing order (`inspect`
lists them). Points inside
`--legend` (the legend's line samples) are dropped. `overlay` draws the extracted points
back onto the page, to check the extraction by eye.
"""

import argparse
import csv
import sys

try:
    import pymupdf
except ImportError:
    sys.exit("PyMuPDF required: pip install pymupdf")


def rect(text):
    return pymupdf.Rect(*map(float, text.split(",")))


def pair(text):
    a, b = map(float, text.split(","))
    return a, b


def dash_key(drawing):
    dashes = (drawing.get("dashes") or "").strip()
    inner = dashes.split("]")[0].strip("[ ").split()
    return "solid" if not inner else f"{float(inner[0]):.1f}"


def bezier(p0, p1, p2, p3, n=12):
    for i in range(1, n + 1):
        t = i / n
        u = 1 - t
        yield pymupdf.Point(
            u**3 * p0.x + 3 * u * u * t * p1.x + 3 * u * t * t * p2.x + t**3 * p3.x,
            u**3 * p0.y + 3 * u * u * t * p1.y + 3 * u * t * t * p2.y + t**3 * p3.y,
        )


def inspect(args):
    page = pymupdf.open(args.pdf)[args.page - 1]
    clip = rect(args.clip)
    for d in page.get_drawings():
        if not clip.intersects(d["rect"]):
            continue
        r = d["rect"]
        kinds = "".join(item[0] for item in d["items"])
        print(
            f"path ({r.x0:.1f},{r.y0:.1f},{r.x1:.1f},{r.y1:.1f}) items={len(d['items'])} "
            f"kinds={kinds[:12]} width={d.get('width')} dash={dash_key(d)}"
        )
    for x0, y0, x1, y1, word, *_ in page.get_text("words"):
        if clip.intersects(pymupdf.Rect(x0, y0, x1, y1)):
            print(f"text {word!r} centre ({(x0 + x1) / 2:.1f},{(y0 + y1) / 2:.1f})")


def extract(args):
    page = pymupdf.open(args.pdf)[args.page - 1]
    frame, legend = rect(args.frame), rect(args.legend) if args.legend else None
    (xl, xr), (yb, yt) = pair(args.x), pair(args.y)

    def to_data(p):
        return (
            xl + (p.x - frame.x0) / frame.width * (xr - xl),
            yb + (frame.y1 - p.y) / frame.height * (yt - yb),
        )

    curves, ordered = {}, []
    for d in page.get_drawings():
        if d.get("width") is None or not frame.intersects(d["rect"]) or d["items"][0][0] == "re":
            continue
        # One path can hold several curves: split it where a segment does not start at
        # the previous one's end.
        subpaths, end = [], None
        for item in d["items"]:
            if item[0] not in ("l", "c"):
                continue
            start = item[1]
            if end is None or abs(start.x - end.x) + abs(start.y - end.y) > 0.01:
                subpaths.append([start])
            if item[0] == "l":
                subpaths[-1].append(item[2])
                end = item[2]
            else:
                subpaths[-1].extend(bezier(*item[1:5]))
                end = item[4]
        for sub in subpaths:
            points = [p for p in sub if frame.contains(p) and not (legend and legend.contains(p))]
            if len(points) > 5:
                curves.setdefault(dash_key(d), []).extend(points)
                ordered.append(points)

    for spec in args.curves:
        name, key = spec.split("=")
        if key.startswith("@"):
            if int(key[1:]) >= len(ordered):
                sys.exit(f"only {len(ordered)} curve paths")
            points = ordered[int(key[1:])]
        elif key in curves:
            points = curves[key]
        else:
            sys.exit(f"no curve with dash {key}; found {sorted(curves)}")
        data = sorted({(round(x, 4), round(y, 4)) for x, y in map(to_data, points)})
        with open(f"{args.out}/{name}.csv", "w", newline="") as f:
            f.write(f"{args.columns}\n")
            f.writelines(f"{x:.4f},{y:.4f}\n" for x, y in data)
        print(f"{name}: {len(data)} points, x {data[0][0]:.2f}..{data[-1][0]:.2f}")


def overlay(args):
    doc = pymupdf.open(args.pdf)
    page = doc[args.page - 1]
    frame = rect(args.frame)
    (xl, xr), (yb, yt) = pair(args.x), pair(args.y)
    colours = [(1, 0, 0), (0, 0.6, 0), (0, 0, 1), (1, 0.5, 0), (0.6, 0, 0.6), (0, 0.6, 0.6)]
    for i, path in enumerate(args.csv):
        with open(path) as f:
            rows = list(csv.reader(line for line in f if not line.startswith("#")))[1:]
        for x, y in ((float(r[0]), float(r[1])) for r in rows):
            p = pymupdf.Point(
                frame.x0 + (x - xl) / (xr - xl) * frame.width,
                frame.y1 - (y - yb) / (yt - yb) * frame.height,
            )
            c = colours[i % len(colours)]
            page.draw_circle(p, 0.9, color=c, fill=c)
    page.get_pixmap(dpi=300, clip=frame + (-30, -10, 10, 25)).save(args.png)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("inspect", "extract", "overlay"):
        p = sub.add_parser(name)
        p.add_argument("pdf")
        p.add_argument("--page", type=int, required=True)
    sub.choices["inspect"].add_argument("--clip", required=True)
    for name in ("extract", "overlay"):
        p = sub.choices[name]
        p.add_argument("--frame", required=True)
        p.add_argument("--x", required=True)
        p.add_argument("--y", required=True)
    e = sub.choices["extract"]
    e.add_argument("--legend")
    e.add_argument("--out", required=True)
    e.add_argument("--columns", default="log_h,density")
    e.add_argument("curves", nargs="+", help="NAME=DASH, e.g. t12=16.2 or t06=solid")
    o = sub.choices["overlay"]
    o.add_argument("--png", required=True)
    o.add_argument("csv", nargs="+")
    args = ap.parse_args()
    {"inspect": inspect, "extract": extract, "overlay": overlay}[args.cmd](args)


if __name__ == "__main__":
    main()
