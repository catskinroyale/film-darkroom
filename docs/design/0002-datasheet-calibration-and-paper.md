# Design 0002 — Datasheet calibration and the paper model

Status: **Implemented (unreleased)** · Date: 2026-10-24 (§4–§8 added the same day) · Builds on [design 0001](0001-negative-simulation-model.md)

This note records how manufacturer curves are now read from datasheets, the first Level-2
calibration (Tri-X 400 in D-76), a change to the development model that the data forced, and
the replacement of the placeholder paper with a measured one. The approach was prompted by
NegPy (GPL-3.0); no NegPy code or fitted constants are used, only the idea and the primary
sources it points to.

## 1. Reading curves from datasheet PDFs

Kodak's datasheets draw their characteristic curves as **PDF vector paths**, not images.
`tools/datasheet_curves.py` reads those paths with PyMuPDF and maps them through the plot's
axis frame to data units, so the extracted points are the plotted curves exactly: there is
no hand-digitising error. Curves are told apart by dash pattern, or by drawing order where
they are drawn alike; paths holding several curves are split into their subpaths.
`overlay` draws the points back onto the page, and every extraction is checked that way.

| Datasheet | Plots | Vector? |
|---|---|---|
| Kodak F-4017 (Tri-X, Feb 2016) | characteristic curves, CI vs time, spectral, MTF | yes |
| Kodak G-26 (Polymax II RC, June 2005) | grade curves −1…5+, spectral | yes |
| Kodak G-4037 (Polycontrast IV RC, June 2005) | grade curves 1, 3, 5+ per surface | yes |
| Kodak F-4043 (T-Max 400, Feb 2016) | D-76 family 6/8/11 min; CI vs time | yes |
| Kodak F-4016 (T-Max 100, Feb 2016) | D-76 family; legend times lost to a font fault | yes (times from the Mar 2002 edition, whose curves are identical within 0.003 D) |
| Kodak H-1-5222 (Double-X, Mar 2022) | no characteristic curve | — |
| Ilford Multigrade RC (Oct 2020) | grade curves | **no**: raster images, curves overlapping; only the ISO R table is usable as text |
| Ilford HP5 Plus, FP4 Plus (Nov 2018), Delta 400 (Nov 2018) | one curve each, relative axis | **no**: raster; traced (§6) |
| Ilford 100 Delta (2010) | one curve, relative axis | **no**: image strips; traced from the rendered page |
| Ilford Delta 3200 (Apr 1999) | DD-X and Microphen families, relative axis | yes |
| Kentmere Pan 100/200/400 | no characteristic curve | — |

Raster curves are traced with `tools/raster_curve.py`: grid lines are found as near-full
dark rows and columns, the axes are calibrated from two grid lines per axis, and in each
column the curve is the thickest dark run off the grid. At these resolutions (about 90–270
px per log unit) the uncertainty is roughly ±0.02 D; each trace is checked on an overlay.

Extracted data lives in `data/sensitometry/<source>/` with a README per source.

## 2. Tri-X 400 + D-76 at Level 2

Source: F-4017 p. 8, 35 mm, D-76 stock, large tank, agitation every minute, 20 °C, 6, 8, 10
and 12 min, diffuse visual density. `tools/fit_development.py` fits the engine's own model
(ported spline included) to all four curves jointly and writes
`data/calibrations/tri-x_d76-stock.csv`, which `bw-negative::fitted` compiles in.

### 2.1 The data contradict design 0001's family form

Design 0001 §3.3 grows one asymptotic curve by a development factor:
`D(h, t) = D_f(t) + D∞(h)·(1 − e^(−t/τc))`. At any exposure, doubling development can then
at most double net density. Kodak's family does not obey this:

| log H | net D 6 min | net D 12 min | ratio 12/6 |
|---|---|---|---|
| −3.0 | 0.013 | 0.054 | 4.3 |
| −2.5 | 0.095 | 0.315 | 3.3 |
| −1.0 | 0.787 | 1.566 | 2.0 |
| 0.0 | 1.310 | 2.252 | 1.7 |

The toe grows far faster than any multiplicative factor allows: **longer development makes
the film faster**, moving the whole curve to lower exposure (the point 0.1 above fog moves
about 0.35 log H from 6 to 12 min). Fitting the old form, the optimiser hid this in an
implausible fog growth. Design 0001's own remedy, an h-dependent τc, cannot help: it changes
the growth rate but not the 2× ceiling.

### 2.2 Decision D12: development moves the curve

```
D(h, τ) = D_base + D_f(τ) + D∞(h + s·ln(τ/τ_ref)) · (1 − e^(−τ/τc0))
```

`s` (log H per unit ln τ) is fitted with the curve; `τ_ref` is the manufacturer's normal
time, where the shift is zero. Illustrative calibrations keep `s = 0`, so they are unchanged.
In the engine this is `bw_developer::SpeedGain`.

### 2.3 Fit and validation

| Check | Result | Target |
|---|---|---|
| Hold-out: fit 6/10/12 min, predict 8 min | RMS 0.014 D | ≤ 0.03 D (0001 §8.2) |
| All four curves, engine evaluation (test) | RMS ≤ 0.019 D, max 0.041 D | ≤ 0.03 D |
| Contrast index at Kodak's large-tank normal time, 7¾ min | 0.575 | Kodak's aim 0.56 |
| Kodak's separate small-tank D-76 CI line (5.4–10.8 min), times × 1.155 | RMS 0.023 | — |
| ISO speed at ISO 6 contrast (ΔD 0.80 over 1.30 log H, 9.3 min) | 483 (+0.27 stop) | within ⅓ stop of 400 |
| Real speed gain of Kodak's EI 1600 push (9½ vs 6¾ min, small tank) | +0.99 stop | — |

Fitted values: τc0 = 11.3 min, s = 0.649, base + initial fog 0.282 (fog growth fitted at 0).

Contrast index follows Kodak H-740 exactly: a straight edge marked at 0, 0.2 and 2.2 log
units, its 0 mark on the base+fog line and both other marks on the curve.

**Agitation.** Kodak's tables give large-tank (every minute) times 1.155× the small-tank
(every 30 s) times, the mean of the five temperatures. The calibration's reference
agitation is every 60 s and its rate slope reproduces that ratio, so 6¾ min small tank
equals the 7¾ min large-tank curve. Temperature: α = ln(9¼/5½)/6 from the large-tank table.

**Push.** Design 0001 §8.2 expected a push to gain "less than one stop" of real speed. On
Kodak's own curves the two-stop push gains 0.99 stop and the three-stop push 1.4 stops: about
half the nominal push, the darkroom rule of thumb. The test now asks for 0.6–1.4 stops.

### 2.4 Still illustrative in this combination

Adjacency, stand-development compensation, grain growth and spectral weights are not fitted.
The provenance says so, and the app's Calibration readout shows *level 2: sensitometric* for
Tri-X + D-76 only.

## 3. The paper: Kodak Polymax II RC

The placeholder logistic (grade 0–5 mapped to a log-exposure range) is replaced by a measured
grade family: Kodak G-26 p. 7, Polymax II RC, F surface, Polymax filters −1, 0, 1, 1½, 2, 3,
3½, 4 and 5+, Dektol 1:2, 60 s, 20 °C. `tools/paper_family.py` writes
`data/papers/kodak-polymax-ii-rc.csv`.

**Identification.** The curves are drawn alike and labelled by leader lines, so each was
identified by its ISO 6846 range and relative speed against Kodak's own table, then checked
against the leader lines on an overlay:

| Filter | −1 | 0 | 1 | 1½ | 2 | 3 | 3½ | 4 | 5+ | white light |
|---|---|---|---|---|---|---|---|---|---|---|
| Measured ISO R | 144 | 136 | 128 | 125 | 119 | 98 | 87 | 74 | 56 | 107 |
| Kodak's table (F, Dektol 1:2) | 140 | 140 | 130 | 120 | 120 | 100 | 80 | 70 | 40 | 100 |

Filters −1 to 3½ are speed-matched (within 0.1 log H) and 4 and 5+ are one stop slower, as
Kodak's P250/P125 figures say. Two disagreements inside the datasheet are recorded, not
corrected: 5+ plots at R56 against R40 in the table, and white light at R107 against R100.
The plot's absolute exposure axis does not match Kodak's P numbers; it does not matter,
because print exposure is solved from the curve (black point held, §9.1 of 0001).

**In-between grades** blend the neighbouring curves horizontally: at each fraction of the
density range the log exposure is interpolated, a first approximation to mixed filtration.
**Dmax** comes from each curve (2.10–2.15; 1.88 for filter −1).

**Why Polymax.** Of the variable-contrast papers whose curves are published as vectors
(Kodak's Polymax II and Polycontrast IV), it has the fullest grade family, −1 to 5+. Kodak
discontinued its papers; the curves stand for a representative RC VC paper and the app
labels it "Polymax II-like, glossy". Ilford's Multigrade curves are raster images with
the grades overlapping; adding them would need careful raster digitising.

**Second paper: Polycontrast IV RC, N surface (matte)** (G-4037 p. 8, Dektol 1:2): filters
1, 3 and 5+ only, Dmax about 1.69 against glossy's 2.12, the visible difference between a
matte and a glossy print. Measured ranges are R113, R89 and R47; the datasheet's shared range
table (R130, R100, R40) is lower-Dmax-blind, since ISO R reads to 0.9 × Dmax. Grades below 1
use the filter-1 curve. Styles store the paper (`print.paper`; empty means Polymax II).

## 4. T-Max 100 and T-Max 400 + D-76 at Level 2

Same method as §2, from small-tank families (30-second agitation, the app's default, so no
agitation conversion):

| | T-Max 400 (F-4043) | T-Max 100 (F-4016) |
|---|---|---|
| Curves | 6, 8, 11 min | 6, 7½, 10 min |
| Hold-out RMS (middle curve) | 0.030 D | 0.038 D (only two curves left to fit) |
| All curves, engine (test) | ≤ 0.020 D | ≤ 0.024 D |
| Speed shift s | 0.698 | 0.285 |
| ISO speed at ISO 6 contrast | 391 at 8.9 min (−0.03 stop) | 133 at 7 min (**+0.41 stop**) |
| Kodak's D-76 CI line | RMS 0.027 (to 0.02 up to 9 min; Kodak steeper above) | — (no CI chart) |
| α (small-tank table) | 0.068 / °C | 0.095 / °C |

T-Max 400's τc0 sits at the fitter's bound (100 min): over 6–11 min its contrast grows
almost linearly, with no sign of saturation; raising the bound changes nothing measurable.
T-Max 100 reads +0.4 stop on Kodak's own D-76 curves; Kodak's ISO figure is measured in the
ISO standard's developer, so the test holds it to ½ stop.

## 5. Other developers from contrast-index lines (Level 2−)

Design 0001 §3.3 case 2. Where a developer has only a published CI-vs-time line, its
calibration is the film's D-76 curve run at a fitted rate k: CI_dev(t) = CI_D76(k·t)
(`fit_development.py --rate`). In the engine this rescales τc0 and τ_ref by 1/k; no new term.
`data/calibrations/developer-rates.csv` holds the rates; the temperature coefficient comes
from Kodak's small-tank table for that developer.

| Film | Developer | k | Kodak's normal-time ratio (not used to fit) | CI RMS |
|---|---|---|---|---|
| Tri-X | HC-110 B | 1.747 | 6¾ / 3¾ = 1.80 | 0.042 (max 0.097) |
| Tri-X | D-76 1+1 | 0.699 | 6¾ / 9¾ = 0.69 | 0.028 |
| T-Max 400 | HC-110 B | 1.294 | 7½ / 5½ = 1.36 | 0.009 |
| T-Max 400 | D-76 1+1 | 0.745 | 7½ / 10¼ = 0.73 | 0.025 |

The D-76 lines themselves give k = 1.000 (Tri-X, after the 1.155 tank conversion) and 1.03
(T-Max 400). Tri-X in HC-110 B is the weakest: Kodak's line curves more than a time-scaled
D-76 can follow. These calibrations are labelled level 2 with the assumption stated.

## 6. Ilford films at Level 1

Ilford publishes, per film, one curve in an Ilford developer on a **relative** exposure axis
(Delta 3200: a family). That gives the curve's shape but not its growth with development or
its place on the absolute axis. The calibration (`fit_development.py --borrow --pin-base
--anchor-iso`):

- **Shape** from Ilford's curve; base + fog is the lowest plotted density (`--pin-base`:
  these plots start on the flat base, and letting the fit place the base below it drags the
  speed point down the toe).
- **Growth** (τc0 in proportion to the normal time, and the speed gain) borrowed from the
  nearest fitted Kodak film: HP5 Plus and FP4 Plus from Tri-X, 100 Delta from T-Max 100,
  Delta 400 from T-Max 400. Delta 3200 fits its own DD-X family (hold-out RMS 0.030 D).
- **Absolute axis**: the curve is moved along log H until the model's ISO speed at ISO 6
  contrast equals the film's ISO (design 0001 §7's stated anchoring rule).
- **Developers**: normal development in any developer is assumed to give Ilford's published
  curve; each developer's published time sets its timing. Kentmere and Double-X publish no
  curves and stay illustrative.

The time at which ISO contrast falls is not forced by the anchoring, and lands at Ilford's
normal time each time: HP5 7.0 min (Ilford 6½), FP4 7.2 (8), 100 Delta 8.3 (8½), Delta 400
8.0 (8), Delta 3200 8.1 min in DD-X, between Ilford's EI 800 (7 min) and EI 1600 (8 min)
times for an ISO 1000 film.

## 7. Print exposure: printing for the shadows (decision P3)

With the fitted films, holding clear film (the black point) made pushed prints much too
light: Kodak's curves show N+2 development adding a lot of density, real speed included, so
Tri-X pushed two stops printed mid-grey at 0.24 instead of 0.68, and HP5 in HC-110 B at
0.00. The printer now prints for the shadows: the exposure puts the scene's **zone I** (four
stops under mid-grey) at the print density normal development gives it. Normal development
prints exactly as before (mid-grey 0.75). A push now holds the shadows and brightens the
mid-tones and highlights; Tri-X pushed two stops prints mid-grey at 0.30 with zone I
unchanged. A pull darkens and softens.

## 8. Preflash

A uniform exposure added to the image (`PrintSettings::preflash`, a fraction of the paper's
threshold exposure, where density first rises 0.04 above paper white). It is exact in
exposure terms, the paper sees H_image + H_flash, and keeps the main exposure unchanged. On
Polymax II at grade 2, a 100% flash takes print densities 0.08 → 0.28, 0.3 → 0.53, 1.0 →
1.16 and 1.9 → 1.93: highlights gain most, mid-tones a little, shadows nothing; unexposed
paper reaches only the threshold. Split-grade printing is not modelled: it needs each
emulsion component's response to each filter, which the datasheets do not give.

## 9. Follow-ups

- Kodak's other families in F-4017/F-4043 (T-MAX Developer, T-MAX RS) if those developers
  are added; XTOL from the CI lines.
- Ilford's Microphen family for Delta 3200 as a check on borrowing growth across developers.
- Ilford Multigrade papers, if the overlapping raster grade curves can be separated.
