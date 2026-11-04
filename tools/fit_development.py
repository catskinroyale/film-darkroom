"""Fit a development calibration to a published characteristic-curve family.

Design 0001 §3.3, case 1 ("curve family available"), with the speed term of design 0002:
fit the asymptotic curve D∞(h), the time constant τc0, the speed gain and fog growth
jointly to every curve of a manufacturer's time family, using the engine's own model and
spline, so the fit is exactly what the app renders:

    D(h, t) = D_base + f1·t + D∞(h + s·ln(t / t_ref)) · (1 − exp(−t / τc0))

D∞ is a monotone Steffen spline (fd-core `MonotoneCurve`) on fixed knots, pinned to 0 at
the lowest plotted exposure. `s` moves the curve to lower exposure as development
lengthens (the speed gain every published family shows in its toe); `t_ref` is the
manufacturer's normal time, where the shift is zero. Fog before development is taken as
zero (f0 = 0): the curves cannot separate base density from initial fog, so their sum is
reported as the film's base density.

    python3 -I tools/fit_development.py \\
        --curve 6=data/sensitometry/kodak-f4017/tri-x-400_35mm_d76-stock_large-tank_06min.csv \\
        --curve 8=... --curve 10=... --curve 12=... --normal-time 7.75 --holdout 8 \\
        --ci data/sensitometry/kodak-f4017/tri-x-400_contrast-index_d76-stock_small-tank.csv \\
        --ci-time-factor 1.15 --out data/calibrations/tri-x_d76-stock.csv

`--holdout T` first fits without the T-minute curve and reports how well the model predicts
it. `--rate NAME=CI.csv` (repeatable; design 0001 §3.3 case 2) finds, for another developer
with only a published CI-vs-time line, the rate k against the fitted developer that best
reproduces that line through the fitted curve: CI_X(t) = CI(F·k·t), F the `--ci-time-factor`. `--ci` compares the model's contrast index with a published CI-vs-time line; with
`--ci-time-factor F` those times are first converted to the curves' agitation (t·F).
`--borrow CAL.csv` (design 0002 §6, Level 1) is for a film published as a single curve:
its growth with development is taken from a fitted calibration of a comparable film (τc0 in
proportion to the normal time, and the speed gain), and only the curve shape is fitted.
`--anchor-iso N` places a curve published on a relative exposure axis: the model is moved
along log H until its ISO speed, at the development giving ISO 6 contrast (ΔD 0.80 over
1.30 log H), equals N.
Requires numpy and scipy.
"""

import argparse
import math
import sys

try:
    import numpy as np
    from scipy.optimize import brentq, least_squares
except ImportError:
    sys.exit("numpy and scipy required: pip install numpy scipy")

KNOT_STEP = 0.2
# Knots start this far below the data, so a long development's shifted curve stays on them.
KNOT_MARGIN = 0.4


# ----- the engine's spline (fd-core/src/curve.rs), ported exactly -------------------


def steffen_end_slope(d0, d1, h0, h1):
    p = d0 * (1 + h0 / (h0 + h1)) - d1 * h0 / (h0 + h1)
    if p * d0 <= 0:
        return 0.0
    if abs(p) > 2 * abs(d0):
        return 2 * d0
    return p


def steffen_slopes(xs, ys):
    n = len(xs)
    h = np.diff(xs)
    d = np.diff(ys) / h
    ms = np.zeros(n)
    for i in range(1, n - 1):
        p = (d[i - 1] * h[i] + d[i] * h[i - 1]) / (h[i - 1] + h[i])
        ms[i] = (np.sign(d[i - 1]) + np.sign(d[i])) * min(abs(d[i - 1]), abs(d[i]), 0.5 * abs(p))
    ms[0] = steffen_end_slope(d[0], d[1], h[0], h[1])
    ms[-1] = steffen_end_slope(d[-1], d[-2], h[-1], h[-2])
    return ms


def spline_eval(xs, ys, ms, x):
    x = np.atleast_1d(np.asarray(x, dtype=float))
    out = np.empty_like(x)
    lo, hi = x <= xs[0], x >= xs[-1]
    out[lo] = ys[0] + ms[0] * (x[lo] - xs[0])
    out[hi] = ys[-1] + ms[-1] * (x[hi] - xs[-1])
    mid = ~(lo | hi)
    i = np.searchsorted(xs, x[mid], side="right") - 1
    h = xs[i + 1] - xs[i]
    t = (x[mid] - xs[i]) / h
    t2, t3 = t * t, t * t * t
    out[mid] = (
        (2 * t3 - 3 * t2 + 1) * ys[i]
        + (t3 - 2 * t2 + t) * h * ms[i]
        + (-2 * t3 + 3 * t2) * ys[i + 1]
        + (t3 - t2) * h * ms[i + 1]
    )
    return out


def softplus(v):
    return np.log1p(np.exp(-np.abs(v))) + np.maximum(v, 0)


# ----- model -----------------------------------------------------------------------


class Model:
    """Parameters: base, f1, log τc0, s, then the knot-to-knot increments of D∞ (through
    softplus, so D∞ is non-decreasing from 0 at the first knot). `fixed` pins τc0 and s
    (borrowed growth); then only base, f1 and the curve are free."""

    def __init__(self, knots, t_ref, fixed=None):
        self.knots, self.t_ref, self.fixed = knots, t_ref, fixed

    def unpack(self, p):
        base, f1, log_tau, shift = p[:4]
        if self.fixed:
            log_tau, shift = math.log(self.fixed[0]), self.fixed[1]
        ys = np.concatenate([[0.0], np.cumsum(softplus(p[4:]))])
        return base, f1, math.exp(log_tau), shift, ys

    def fog(self, p, t):
        base, f1, *_ = self.unpack(p)
        return base + f1 * t

    def density(self, p, h, t):
        base, f1, tau, shift, ys = self.unpack(p)
        d_inf = spline_eval(self.knots, ys, steffen_slopes(self.knots, ys), np.asarray(h) + shift * math.log(t / self.t_ref))
        return base + f1 * t + d_inf * (1 - math.exp(-t / tau))

    def at(self, p, h, t):
        return float(self.density(p, h, t)[0])


def load_curve(path):
    rows = [line.strip().split(",") for line in open(path) if line.strip() and not line.startswith("#")]
    data = np.array([[float(a), float(b)] for a, b in rows[1:]])
    return data[:, 0], data[:, 1]


def resample(curves, step=0.05):
    """Each curve on a common log H grid, so every time in the family weighs the same
    however densely its plot path happened to be drawn."""
    out = {}
    for t, (h, d) in curves.items():
        grid = np.arange(h.min(), h.max() + 1e-9, step)
        out[t] = (grid, np.interp(grid, h, d))
    return out


def fit(model, curves, smooth=0.02, base=None):
    curves = resample(curves)

    def residuals(p):
        r = [model.density(p, h, t) - d for t, (h, d) in curves.items()]
        r.append(smooth * np.diff(model.unpack(p)[4], 2))  # against knot wiggle
        return np.concatenate(r)

    # Start from the data: the longest development's net curve, scaled up a little.
    h, d = curves[max(curves)]
    net = np.maximum.accumulate(np.interp(model.knots, h, d) - d.min())
    steps = np.maximum(np.diff(net / 0.8), 1e-4)
    p0 = np.concatenate([[d.min(), 0.002, math.log(8.0), 0.3], np.log(np.expm1(steps))])
    lower = np.concatenate([[0.0, 0.0, 0.0, -1.0], np.full(len(steps), -30.0)])
    upper = np.concatenate([[1.0, 0.02, math.log(100.0), 2.0], np.full(len(steps), 30.0)])
    if base is not None:
        p0[0], lower[0], upper[0] = base, base - 1e-9, base + 1e-9
    if model.fixed or len(curves) == 1:
        # One curve cannot separate fog growth from the curve: no fog growth.
        upper[1] = 1e-9
        p0[1] = 0.0
    return least_squares(residuals, p0, bounds=(lower, upper), method="trf").x


def rms(model, p, h, d, t):
    return float(np.sqrt(np.mean((model.density(p, h, t) - d) ** 2)))


# ----- sensitometric figures (design 0001 §8.2) ------------------------------------


def iso_contrast_time(model, p):
    """Development time at which ΔD over 1.30 log H from the speed point is 0.80."""
    ts = np.linspace(0.3 * model.t_ref, 4.0 * model.t_ref, 200)
    gaps = [iso_speed(model, p, t)[1] - 0.80 for t in ts]
    crossing = [k for k in range(1, len(ts)) if gaps[k - 1] < 0 <= gaps[k]]
    if not crossing:
        sys.exit(f"ΔD over 1.30 log H runs {min(gaps) + 0.8:.2f}–{max(gaps) + 0.8:.2f} between "
                 f"{ts[0]:.1f} and {ts[-1]:.1f} min: never ISO 6 contrast (0.80)")
    i = crossing[0]
    return brentq(lambda t: iso_speed(model, p, t)[1] - 0.80, ts[i - 1], ts[i])


def read_calibration(path):
    values = {}
    for line in open(path):
        if line.startswith("#") or line.startswith("log_h"):
            continue
        key, _, value = line.strip().partition(",")
        if key and not key[0].isdigit() and key[0] != "-":
            values[key] = float(value)
    return values


def contrast_index(model, p, t):
    """Kodak contrast index (H-740, "Contrast Index"): a straight edge marked at 0, 0.2 and
    2.2 log units is laid so that its 0 mark touches the base+fog line and both other marks
    touch the curve; the contrast index is the edge's slope. Distances are measured along
    the edge, with density and log exposure on equal scales."""
    fog = model.fog(p, t)

    def lower_start(theta):
        # Where the edge must start for its 0.2 mark to touch the curve.
        c, s = math.cos(theta), math.sin(theta)
        return brentq(lambda x0: model.at(p, x0 + 0.2 * c, t) - fog - 0.2 * s, -10.0, 3.0)

    def upper_miss(theta):
        c, s = math.cos(theta), math.sin(theta)
        x0 = lower_start(theta)
        return model.at(p, x0 + 2.2 * c, t) - fog - 2.2 * s

    theta = brentq(upper_miss, math.atan(0.05), math.atan(3.0))
    return math.tan(theta)


def iso_speed(model, p, t):
    """ISO 6: speed point 0.10 above base+fog; speed 0.8/H_m; ΔD over the next 1.30 log H."""
    fog = model.fog(p, t)
    hm = brentq(lambda h: model.at(p, h, t) - fog - 0.10, model.knots[0] - 6.0, model.knots[-1] + 2.0)
    return 0.8 / 10**hm, model.at(p, hm + 1.30, t) - model.at(p, hm, t)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--curve", action="append", required=True, help="TIME_MIN=path.csv")
    ap.add_argument("--normal-time", type=float, required=True, help="manufacturer's normal time for these curves' conditions")
    ap.add_argument("--holdout", type=float, help="first validate by predicting this curve")
    ap.add_argument("--ci", help="published CI-vs-time line (time_min,contrast_index) to compare with")
    ap.add_argument("--ci-time-factor", type=float, default=1.0, help="convert the CI line's times to the curves' agitation")
    ap.add_argument("--rate", action="append", default=[], help="NAME=ci.csv: fit a developer's rate from its CI line")
    ap.add_argument("--borrow", help="fitted calibration whose growth (τc0 per normal time, speed gain) is borrowed")
    ap.add_argument("--anchor-iso", type=float, help="place a relative-axis curve so its ISO speed at ISO contrast is this")
    ap.add_argument("--pin-base", action="store_true", help="base+fog = the lowest plotted density; curve starts at the first plotted exposure")
    ap.add_argument("--out", help="calibration CSV to write")
    args = ap.parse_args()

    curves = {float(t): load_curve(path) for t, path in (s.split("=", 1) for s in args.curve)}
    lo = min(h.min() for h, _ in curves.values()) - (0.0 if args.pin_base else KNOT_MARGIN)
    hi = max(h.max() for h, _ in curves.values())
    knots = np.arange(lo, hi + 1e-9, KNOT_STEP)
    if hi - knots[-1] > 1e-6:
        knots = np.append(knots, hi)
    fixed = None
    if args.borrow:
        b = read_calibration(args.borrow)
        fixed = (b["tau_c0_min"] * args.normal_time / b["tau_ref_min"], b["speed_shift"])
        print(f"borrowed from {args.borrow}: tau_c0 {fixed[0]:.2f} min, speed shift {fixed[1]:.3f}")
    model = Model(knots, args.normal_time, fixed)

    base = min(d[np.argmin(h)] for h, d in curves.values()) if args.pin_base else None
    if args.holdout is not None:
        p = fit(model, {t: c for t, c in curves.items() if t != args.holdout}, base=base)
        h, d = curves[args.holdout]
        print(f"hold-out {args.holdout:g} min: RMS {rms(model, p, h, d, args.holdout):.4f} D (predicted, not fitted)")

    p = fit(model, curves, base=base)
    offset = 0.0
    if args.anchor_iso:
        t_iso = iso_contrast_time(model, p)
        speed, _ = iso_speed(model, p, t_iso)
        offset = math.log10(speed / args.anchor_iso)
        model.knots = model.knots + offset  # more exposure needed: the curve moves right
        knots = model.knots
        print(f"anchored: ISO contrast at {t_iso:.2f} min; relative axis + {offset:.3f} log H "
              f"puts the ISO speed at {iso_speed(model, p, t_iso)[0]:.0f}")
    base, f1, tau, shift, ys = model.unpack(p)
    print(f"base+initial fog {base:.3f} D, fog growth {f1:.4f} D/min, tau_c0 {tau:.2f} min, "
          f"speed shift {shift:.3f} log H per ln(t/{args.normal_time:g})")
    for t, (h, d) in sorted(curves.items()):
        err = model.density(p, h + offset, t) - d
        print(f"  {t:g} min: RMS {np.sqrt(np.mean(err**2)):.4f} D, max |err| {np.abs(err).max():.4f} D, "
              f"CI {contrast_index(model, p, t):.3f}")
    speed, delta = iso_speed(model, p, args.normal_time)
    print(f"at the normal {args.normal_time:g} min: CI {contrast_index(model, p, args.normal_time):.3f}, "
          f"ISO speed {speed:.0f} (ΔD over 1.30 log H {delta:.2f}; ISO 6 asks 0.80 ± 0.05)")
    if args.ci:
        times, published = load_curve(args.ci)
        rows = [(t, c, contrast_index(model, p, t * args.ci_time_factor)) for t, c in zip(times, published)]
        err = np.array([m - c for _, c, m in rows])
        print(f"CI line ({args.ci_time_factor:g}× time): RMS {np.sqrt(np.mean(err**2)):.3f}, "
              + ", ".join(f"{t:.1f} min {c:.2f}→{m:.2f}" for t, c, m in rows[::3]))

    for spec in args.rate:
        name, path = spec.split("=", 1)
        times, published = load_curve(path)
        factor = args.ci_time_factor

        def miss(log_k):
            k = math.exp(log_k[0])
            return [contrast_index(model, p, t * factor * k) - c for t, c in zip(times, published)]

        k = math.exp(least_squares(miss, [0.0], bounds=([-1.5], [1.5])).x[0])
        err = np.array(miss([math.log(k)]))
        print(f"rate {name}: k {k:.3f} against the fitted developer, CI RMS {np.sqrt(np.mean(err**2)):.3f}, "
              f"max {np.abs(err).max():.3f} over {times.min():.1f}–{times.max():.1f} min")

    if args.out:
        with open(args.out, "w") as f:
            f.write("# Development calibration fitted by tools/fit_development.py (design 0002).\n")
            f.write("# D = base_density + fog_f1_per_min * tau\n")
            f.write("#     + d_inf(log_h + speed_shift * ln(tau / tau_ref_min)) * (1 - exp(-tau / tau_c0_min))\n")
            f.write("# d_inf: Steffen monotone spline through the knots below (net density; log10 lux-seconds).\n")
            f.write("# Fitted to: " + ", ".join(f"{t:g} min" for t in sorted(curves)) + "\n")
            if args.borrow:
                f.write(f"# Growth borrowed from {args.borrow} (Level 1).\n")
            if args.anchor_iso:
                f.write(f"# Relative exposure axis placed so the ISO speed at ISO contrast is {args.anchor_iso:g}.\n")
            f.write(f"tau_c0_min,{tau:.4f}\nspeed_shift,{shift:.4f}\ntau_ref_min,{args.normal_time:g}\n")
            f.write(f"base_density,{base:.4f}\nfog_f1_per_min,{f1:.5f}\n")
            f.write("log_h,d_inf\n")
            f.writelines(f"{x:.4f},{y:.5f}\n" for x, y in zip(knots, ys))
        print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
