# Design 0003: the colour film model

Status: implemented (first version, unreleased). All film and print parameters are **illustrative**.

Written for Colour Darkroom, now Film Darkroom's colour process (`crates/colour-film`,
`fd_profiles::colour`; see [design 0004](0004-one-app-two-processes.md)). It follows the
black-and-white model's approach ([design 0001](0001-negative-simulation-model.md)): the
photographic chain is modelled as separate, physically motivated stages that meet through
a simulated negative, rather than as a colour grade or LUT. Each control maps onto
something that happens to film.

## 1. Pipeline

```text
scene (camera raw → scene-linear Rec.709, white-balanced)
  │  film compression (optional): highlights rolled towards the mid-tones, in stops
  │  ×2^EV / 0.18                       exposure relative to the metered grey
  ▼
layer exposures  H = S · scene          S: 3×3 spectral sensitivity, rows sum to 1
  │  vignette (r⁴ fall-off, full-frame coordinates)
  │  bloom    H ← (1−b)H + b·blur(H)
  │  halation H_i += k·w_i·blur(0.75 H_r + 0.25 H_g)
  ▼
log exposure  x = log10 H              0 = metered mid-grey
  │  floor at the toe, then interimage: x_i += c·(x_i − mean of the other two)
  ▼
net density  D_i = curve_i(x_i)        toe · straight line γ · shoulder
  │  edge effects  D += a·(D − blur(D))
  │  grain         D += √(k·(D + 0.05)) · n_i,  one dye-cloud field per layer
  ▼
negative ── printer lights P_i (timed so grey prints neutral) ──► print exposure e_i = P_i − D_neg,i
  │                                                                 print density D'_i = print_i(e_i)
  ▼
display = 10^(−A · D')                  A: dye unwanted absorptions; 1.0 = print white
```

Reversal (slide) film skips the print: its own falling curve and dyes are viewed directly.

## 2. Decisions

| # | Decision | Rationale |
|---|---|---|
| C1 | **Three dye layers with a spectral-sensitivity matrix**, rows summing to 1. | A neutral exposes all three layers equally, so every stock is balanced for the light the raw was white-balanced to. Off-diagonals are the overlap of the layers' sensitivities (desaturating); a negative term models an inhibiting fourth layer (Fuji). |
| C2 | **Halation and bloom act on light, before the log.** | Both are optical: light scattered back from the base, or spread by a diffusion filter. Added linearly, they only show round sources much brighter than their surroundings, as on film. The halo source is mostly red light (it reaches the base) and lands mostly in the red layer (it sits at the bottom), so halos are red-orange. |
| C3 | **Interimage (DIR) effects act on log exposure, not density.** | Applied to density, layers with slightly different gammas are unequal at grey, so the effect tinted neutrals. In log exposure a neutral is exactly equal in all layers: colour differences grow, neutrals never move. Log exposure is floored at the bottom of the toe first, because nothing develops there, and without the floor sensor noise in black shadows was pushed into one layer and printed as coloured specks. |
| C4 | **One curve shape everywhere: soft toe, straight line, smooth shoulder.** | Negatives, prints and slides all use it (slides as a falling curve). A logistic was tried for prints and slides first, but it put mid-grey on its soft lower half, so every print rendered at roughly the same contrast whatever its gamma. |
| C5 | **The lab times every print to grey.** | Printer lights are solved so the metered grey prints as a neutral 18 % on every film × print pair (tested). So film exposure (EV) changes the negative's character (overexposure: denser, cleaner, more saturated; underexposure: thin and muddy), not the print's brightness, as with real lab prints. Print exposure and printer points sit on top, in the lab's units (1 point = 0.025 log E). |
| C6 | **Stock looks as printer points.** | Each stock carries `look_points`: the lab rendering it is known for (Gold's warmth, 400H's cool cast). It is a separate, switchable layer ("Film's lab look"), so the strictly neutral print is always available. |
| C7 | **Grain is the black-and-white model's Selwyn model, per layer.** | σ²(D) ∝ granularity area × D / pixel area, keyed on full-frame pixel coordinates, so the 1:1 detail view's grain matches the export. The blue-sensitive (fast) layer is grainiest. |
| C8 | **The neutral scan is a print with ideal dyes.** | Scanning is modelled as printing onto a virtual stock with no dye crosstalk and a gentle curve, so it shares timing and printer lights with real prints. |
| C9 | **Push/pull change development only**, as in black and white. | Your photo is already exposed. A push raises contrast (~12 %/stop), shadow speed and grain for negatives; slides gain about a stop per stop. |
| C10 | **Film compression is a creative, scene-referred stage before the film**, after Dehancer's tool of that name. Since design 0004 it lives in `fd-core` and black and white uses it too (luminance only). On luminance stops around grey: a soft knee placed by *Tonal range*, a slope above it lowered by *Impact* (down to 0.25), the compressed part stretched or squeezed by *White point* (50–120), and *Colour density* fading compressed highlights towards neutral. Colour ratios are kept otherwise; off by default. | Digital highlights clip where a negative keeps absorbing light. The film's own shoulder still applies afterwards; compression lets a photo's highlights reach it gently, as heavy overexposure on negative does, and recovers window views and skies. It is labelled a creative control, not a physical layer. |

## 3. What is illustrative

Everything in `crates/colour-film/src/stock.rs`: sensitivities, gammas, toes, base densities,
interlayer strengths, granularity, halation, dye matrices and look points. They were chosen
to give each stock the character photographers describe for it and checked by eye on raw
files; none is fitted to a datasheet. The model is structured so measured data can replace
them stock by stock without changing the pipeline.

## 4. Calibration path (next)

As [design 0002](0002-datasheet-calibration-and-paper.md) did for black and white:

1. **Negative curves**: Kodak's Portra, Ektar, Gold, UltraMax and Vision3 datasheets publish
   per-layer characteristic curves (status M density vs log H) and spectral sensitivity
   curves. Read the vectors, fit `toe_log_h`, `toe_width`, per-layer `gamma`/`speed`.
2. **Spectral sensitivity**: integrate the published layer sensitivities against the camera
   working space (Rec.709 primaries via a spectral basis) to fill `sensitivity`.
3. **Dyes**: the datasheets' spectral dye density curves give the unwanted absorptions `A`.
4. **Print stocks**: Kodak 2383 and Endura datasheets publish curves and dye spectra.
5. **Granularity**: Vision3 datasheets give RMS granularity; Portra and Ektar give print
   grain index instead, which needs a conversion.

Fitted stocks should get a calibration level above Illustrative, shown in the curves panel
and the export recipe.
