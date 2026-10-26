"""Combine a paper's per-filter characteristic curves into one engine data file.

Takes curves extracted with tools/datasheet_curves.py (log_h,density per filter grade),
checks each against the datasheet's ISO 6846 range, and writes them, resampled every
0.05 log H and made non-decreasing, as one CSV the print engine compiles in.

    python3 -I tools/paper_family.py --out data/papers/kodak-polymax-ii-rc.csv \\
        -1=data/sensitometry/kodak-g26/...filter-minus1.csv:140 0=...filter-0.csv:140 ...

Each argument is FILTER=CSV[:R], with R the datasheet's ISO range for that filter (if
given, the measured range is printed beside it). ISO 6846 range: 100 × the log-exposure
difference between density Dmin + 0.04 and 0.9 × Dmax.
"""

import argparse
import sys

STEP = 0.05


def load(path):
    rows = [line.strip().split(",") for line in open(path) if line.strip() and not line.startswith("#")]
    return [(float(a), float(b)) for a, b in rows[1:]]


def interp(x, xs, ys):
    if x <= xs[0]:
        return ys[0]
    for i in range(1, len(xs)):
        if x <= xs[i]:
            t = (x - xs[i - 1]) / (xs[i] - xs[i - 1]) if xs[i] > xs[i - 1] else 0.0
            return ys[i - 1] + t * (ys[i] - ys[i - 1])
    return ys[-1]


def log_h_at(d, hs, ds):
    """First log H where the (non-decreasing) curve reaches density d."""
    for i in range(1, len(ds)):
        if ds[i] >= d:
            t = (d - ds[i - 1]) / (ds[i] - ds[i - 1]) if ds[i] > ds[i - 1] else 0.0
            return hs[i - 1] + t * (hs[i] - hs[i - 1])
    return hs[-1]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", required=True)
    ap.add_argument("--comment", action="append", default=[], help="header comment line (repeatable)")
    ap.add_argument("curves", nargs="+", help="FILTER=CSV[:R]")
    args = ap.parse_args()

    rows = []
    print(f"{'filter':>7} {'Dmin':>6} {'Dmax':>6} {'R':>5} {'table':>6} {'speed':>7}")
    for spec in args.curves:
        filt, rest = spec.split("=", 1)
        path, _, table = rest.partition(":")
        points = sorted(load(path))
        hs = [p[0] for p in points]
        ds = []
        for _, d in points:
            ds.append(max(d, ds[-1]) if ds else d)  # non-decreasing
        grid = [hs[0] + i * STEP for i in range(int((hs[-1] - hs[0]) / STEP) + 1)]
        dense = [interp(h, hs, ds) for h in grid]
        d_min, d_max = dense[0], dense[-1]
        r = 100 * (log_h_at(0.9 * d_max, grid, dense) - log_h_at(d_min + 0.04, grid, dense))
        speed = -log_h_at(d_min + 0.6, grid, dense)
        print(f"{filt:>7} {d_min:6.3f} {d_max:6.3f} {r:5.0f} {table or '-':>6} {speed:+7.2f}")
        rows += [(filt, h, d) for h, d in zip(grid, dense)]

    with open(args.out, "w") as f:
        for line in args.comment:
            f.write(f"# {line}\n")
        f.write("filter,log_h,density\n")
        f.writelines(f"{filt},{h:.3f},{d:.4f}\n" for filt, h, d in rows)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    sys.exit(main())
