//! Colour film simulation (design 0001).
//!
//! ```text
//! SceneImage ─► layer exposures (spectral sensitivity) ─► vignette, bloom, halation (light)
//!            ─► log ─► per-layer characteristic curves ─► interlayer + edge effects (density)
//!            ─► grain ─► negative ─► printer lights (timed to grey) ─► print curves ─► dyes
//!            ─► display-linear Rec.709
//! ```
//!
//! Reversal (slide) film skips the print: its own dyes are viewed directly.
//!
//! Exposure is relative: 1.0 is the metered mid-grey (scene value 0.18) and log
//! exposure `x = log10(H)` is 0 there. Densities are net of base + fog unless stated.

pub mod stock;

pub use stock::{
    FilmKind, FilmStock, Grain, Halation, Layer, PrintKind, PrintStock, film_stock, film_stocks,
    print_stock, print_stocks,
};

use cd_core::{FilmGeometry, Lut1D, Plane, SceneImage, blur, noise};
use rayon::prelude::*;

/// Scene value metered as mid-grey.
pub const GREY: f64 = 0.18;
/// Display density of mid-grey above white: 18 % reflectance.
pub const GREY_DENSITY: f64 = 0.744_727_5;
/// One printer point, in log10 exposure (the usual lab step, about 1/12 stop).
pub const POINT_LOG_E: f64 = 0.025;
/// Granularity floor: unexposed film still has fog grain (net density equivalent).
const FOG_GRAIN_DENSITY: f32 = 0.05;
/// Log exposure range of the film lookup tables (−16 to +10 stops round grey).
const LOG_H_RANGE: (f64, f64) = (-5.0, 3.0);
/// Log exposure range of the print lookup tables.
const LOG_E_RANGE: (f64, f64) = (-4.0, 4.0);
const LUT_SIZE: usize = 4096;
const LOG10_2: f64 = std::f64::consts::LOG10_2;

/// Creative adjustments to the film's grain; the default is the film's own grain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainAdjust {
    /// Multiplier on grain strength.
    pub amount: f64,
    /// Multiplier on dye-cloud size (coarseness, at the same per-pixel strength).
    pub size: f64,
    /// −1 to 1: softer (blends a coarser field) to grittier (heavier tails).
    pub roughness: f64,
}

impl Default for GrainAdjust {
    fn default() -> Self {
        Self {
            amount: 1.0,
            size: 1.0,
            roughness: 0.0,
        }
    }
}

/// Light effects before the emulsion records the image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightEffects {
    /// Multiplier on the film's own halation strength (0 removes it).
    pub halation: f64,
    /// Multiplier on the halation spread.
    pub halation_size: f64,
    /// Bloom (diffusion filter / lens glow): fraction of light spread, 0–1.
    pub bloom: f64,
    /// Bloom spread as a fraction of the frame width.
    pub bloom_size: f64,
    /// Lens vignette: exposure loss in the frame corners, stops.
    pub vignette: f64,
}

impl Default for LightEffects {
    fn default() -> Self {
        Self {
            halation: 1.0,
            halation_size: 1.0,
            bloom: 0.0,
            bloom_size: 0.02,
            vignette: 0.0,
        }
    }
}

/// How the negative is printed (or the slide balanced).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Printing {
    /// Print exposure relative to the grey-timed exposure, stops: more is darker.
    pub exposure_stops: f64,
    /// Colour in printer points (red, green, blue): positive adds that colour to the
    /// print by taking away printer light of that colour. For slides these act as
    /// colour-compensating filters on the camera.
    pub colour_points: [f64; 3],
    /// Apply the stock's typical lab rendering (`FilmStock::look_points`).
    pub stock_look: bool,
}

impl Default for Printing {
    fn default() -> Self {
        Self {
            exposure_stops: 0.0,
            colour_points: [0.0; 3],
            stock_look: true,
        }
    }
}

/// Where the rendered pixels sit in the whole frame (for grain and vignette).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    /// Top-left pixel of this image within the full frame.
    pub origin: [u32; 2],
    /// Full frame size in pixels at this image's scale.
    pub frame: [usize; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub seed: u64,
    pub geometry: FilmGeometry,
    /// Camera exposure compensation, stops.
    pub ev_compensation: f64,
    /// Push (+) or pull (−) processing, stops: development only.
    pub push: f64,
    /// Multiplier on the film's interlayer (colour saturation) effect.
    pub interlayer: f64,
    pub grain: GrainAdjust,
    pub light: LightEffects,
    pub printing: Printing,
    /// `None`: the image is the whole frame.
    pub region: Option<Region>,
}

/// One rendered image.
pub struct Rendered {
    /// Display-linear Rec.709, 1.0 = print white.
    pub image: [Plane; 3],
    /// The developed film's net densities per layer (with grain).
    pub density: [Plane; 3],
}

// ---------------------------------------------------------------------------------------
// Characteristic curves
// ---------------------------------------------------------------------------------------

/// A stock's layers after push/pull: what development produced.
#[derive(Clone, Debug, PartialEq)]
pub struct Developed {
    pub kind: FilmKind,
    pub gamma: [f64; 3],
    pub speed: [f64; 3],
    pub toe_log_h: f64,
    pub toe_width: f64,
    pub density_range: f64,
    pub grain_rms: f64,
    /// Reversal: log exposure of each layer's mid-point, chosen so grey is neutral.
    mid: [f64; 3],
}

/// 3×3 inverse (the dye matrices are well conditioned).
fn inverse(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let c =
        |r0: usize, c0: usize, r1: usize, c1: usize| m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0];
    [
        [
            c(1, 1, 2, 2) / det,
            -c(0, 1, 2, 2) / det,
            c(0, 1, 1, 2) / det,
        ],
        [
            -c(1, 0, 2, 2) / det,
            c(0, 0, 2, 2) / det,
            -c(0, 0, 1, 2) / det,
        ],
        [
            c(1, 0, 2, 1) / det,
            -c(0, 0, 2, 1) / det,
            c(0, 0, 1, 1) / det,
        ],
    ]
}

/// Net dye densities that display as a neutral at `display_density` through `dyes`.
pub fn neutral_dye_densities(dyes: &[[f64; 3]; 3], display_density: f64) -> [f64; 3] {
    let inv = inverse(dyes);
    std::array::from_fn(|i| display_density * inv[i].iter().sum::<f64>())
}

impl Developed {
    /// The stock developed with a push (+) or pull (−) of `push` stops. The effects are
    /// illustrative: about +12 % contrast, a quarter stop of shadow speed and +20 %
    /// granularity per stop of push for negatives; slides gain about a stop per stop.
    pub fn new(stock: &FilmStock, push: f64) -> Self {
        let push = push.clamp(-2.0, 3.0);
        let (gain, speed, range) = match stock.kind {
            FilmKind::Negative => (1.0 + 0.12 * push, 0.075 * push, 1.0),
            FilmKind::Reversal => (1.0 + 0.08 * push, 0.28 * push, 1.0 - 0.04 * push.max(0.0)),
        };
        let gamma = std::array::from_fn(|i| stock.layers[i].gamma * gain);
        let speeds = std::array::from_fn(|i| stock.layers[i].speed + speed);
        let density_range = stock.density_range * range;
        let mut mid = [0.0; 3];
        if stock.kind == FilmKind::Reversal {
            let target = neutral_dye_densities(&stock.dyes, GREY_DENSITY);
            for i in 0..3 {
                // Grey (x = 0) at the neutral target before any push: D(0) = target.
                let f = |u| toe_line_shoulder(u, gamma[i], stock.toe_width, density_range, 4.0);
                mid[i] = solve_increasing(f, target[i]) + stock.layers[i].speed;
            }
        }
        Self {
            kind: stock.kind,
            gamma,
            speed: speeds,
            toe_log_h: stock.toe_log_h,
            toe_width: stock.toe_width,
            density_range,
            grain_rms: stock.grain.rms * (1.0 + 0.2 * push).max(0.5),
            mid,
        }
    }

    /// Net density of layer `i` at log10 exposure `x` (0 = metered grey).
    pub fn net_density(&self, i: usize, x: f64) -> f64 {
        let x = x + self.speed[i];
        let r = self.density_range;
        match self.kind {
            FilmKind::Negative => {
                toe_line_shoulder(x - self.toe_log_h, self.gamma[i], self.toe_width, r, 3.0)
            }
            // A falling curve: highlights in the toe (clear film), shadows in the shoulder.
            FilmKind::Reversal => {
                toe_line_shoulder(self.mid[i] - x, self.gamma[i], self.toe_width, r, 4.0)
            }
        }
    }

    /// Log exposure below which no layer develops measurably (net density < 0.002 for
    /// negatives; for slides, the shadows have reached Dmax).
    pub fn development_floor(&self) -> f64 {
        match self.kind {
            FilmKind::Negative => {
                self.toe_log_h - 1.0 - self.speed.iter().cloned().fold(f64::MIN, f64::max)
            }
            FilmKind::Reversal => {
                let slowest = (0..3)
                    .map(|i| self.mid[i] - self.speed[i] - self.density_range / self.gamma[i])
                    .fold(f64::MAX, f64::min);
                slowest - 1.0
            }
        }
    }

    fn luts(&self) -> [Lut1D; 3] {
        std::array::from_fn(|i| {
            Lut1D::from_fn(LOG_H_RANGE.0, LOG_H_RANGE.1, LUT_SIZE, |x| {
                self.net_density(i, x)
            })
            .expect("valid range")
        })
    }

    /// Curve samples `(log H, net D)` per layer, for plotting.
    pub fn curves(&self, lo: f64, hi: f64, n: usize) -> [Vec<(f64, f64)>; 3] {
        std::array::from_fn(|i| {
            (0..n)
                .map(|k| {
                    let x = lo + (hi - lo) * k as f64 / (n - 1) as f64;
                    (x, self.net_density(i, x))
                })
                .collect()
        })
    }
}

/// The shape of a characteristic curve: a soft toe of width `toe` (log10 exposure), a
/// straight line of slope `gamma`, and a shoulder approaching `dmax` with `sharpness`
/// (higher is more abrupt). `u` is log exposure past the toe. Increasing in `u`.
pub fn toe_line_shoulder(u: f64, gamma: f64, toe: f64, dmax: f64, sharpness: f64) -> f64 {
    let toe = toe.max(1e-3);
    let z = u / toe;
    let t = if z > 30.0 { u } else { toe * z.exp().ln_1p() };
    let s = gamma * t;
    s / (1.0 + (s / dmax).powf(sharpness)).powf(1.0 / sharpness)
}

/// `u` with `f(u) = target` for an increasing `f`, by bisection over a wide range.
fn solve_increasing(f: impl Fn(f64) -> f64, target: f64) -> f64 {
    let (mut lo, mut hi) = (-10.0, 10.0);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if f(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Net density of print layer `i` at log10 exposure `e` (toe starting near `e = 0`).
pub fn print_density(print: &PrintStock, i: usize, e: f64) -> f64 {
    toe_line_shoulder(e, print.gamma[i], print.toe, print.dmax[i], 4.0)
}

/// Inverse of [`print_density`].
fn print_log_e(print: &PrintStock, i: usize, d: f64) -> f64 {
    solve_increasing(|e| print_density(print, i, e), d)
}

/// Printer lights (log10 exposure through clear film, per print layer) that print the
/// metered grey as a neutral mid-grey, then the user's density and colour on top.
pub fn printer_lights(
    stock: &FilmStock,
    film: &Developed,
    print: &PrintStock,
    printing: &Printing,
) -> [f64; 3] {
    let target = neutral_dye_densities(&print.dyes, GREY_DENSITY);
    let points = colour_points(stock, printing);
    std::array::from_fn(|i| {
        let negative_grey = stock.base[i] + film.net_density(i, 0.0);
        print_log_e(print, i, target[i]) + negative_grey + printing.exposure_stops * LOG10_2
            - points[i] * POINT_LOG_E
    })
}

fn colour_points(stock: &FilmStock, printing: &Printing) -> [f64; 3] {
    std::array::from_fn(|i| {
        printing.colour_points[i]
            + if printing.stock_look {
                stock.look_points[i]
            } else {
                0.0
            }
    })
}

// ---------------------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------------------

/// Layer exposures relative to the metered grey, with vignette, bloom and halation.
pub fn expose(scene: &SceneImage, stock: &FilmStock, opts: &RenderOptions) -> [Plane; 3] {
    let [r, g, b] = scene.to_rgb();
    let s = stock.sensitivity;
    let gain = (2f64.powf(opts.ev_compensation) / GREY) as f32;
    // Slides take colour-compensating filters on the camera instead of printer lights.
    let filter: [f32; 3] = if stock.kind == FilmKind::Reversal {
        let p = colour_points(stock, &opts.printing);
        std::array::from_fn(|i| 10f64.powf(-p[i] * POINT_LOG_E) as f32)
    } else {
        [1.0; 3]
    };
    let m: [[f32; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| s[i][j] as f32));
    let (w, h) = (r.width(), r.height());
    let mut layers = [r.zeros_like(), r.zeros_like(), r.zeros_like()];
    let vignette = Vignette::new(opts, w, h);
    {
        let [lr, lg, lb] = &mut layers;
        lr.data_mut()
            .par_chunks_mut(w)
            .zip(lg.data_mut().par_chunks_mut(w))
            .zip(lb.data_mut().par_chunks_mut(w))
            .enumerate()
            .for_each(|(y, ((or, og), ob))| {
                let (sr, sg, sb) = (r.row(y), g.row(y), b.row(y));
                for x in 0..w {
                    let c = [sr[x], sg[x], sb[x]];
                    let v = gain * vignette.factor(x, y);
                    let e = |i: usize| {
                        (m[i][0] * c[0] + m[i][1] * c[1] + m[i][2] * c[2]).max(0.0) * v * filter[i]
                    };
                    or[x] = e(0);
                    og[x] = e(1);
                    ob[x] = e(2);
                }
            });
    }
    let light = &opts.light;
    if light.bloom > 1e-4 {
        let sigma = light.bloom_size * w.max(h) as f64;
        let b = light.bloom.clamp(0.0, 1.0) as f32;
        for p in &mut layers {
            let wide = blur::gaussian_blur_wide(p, sigma);
            p.data_mut()
                .par_iter_mut()
                .zip(wide.data().par_iter())
                .for_each(|(v, &s)| *v = (1.0 - b) * *v + b * s);
        }
    }
    let strength = stock.halation.strength * light.halation.max(0.0);
    if strength > 1e-5 {
        // Red light reaches the base most; green partly; blue is stopped by the yellow filter.
        let mut source = layers[0].clone();
        source
            .data_mut()
            .par_iter_mut()
            .zip(layers[1].data().par_iter())
            .for_each(|(s, &g)| *s = 0.75 * *s + 0.25 * g);
        let sigma = opts
            .geometry
            .um_to_px(stock.halation.radius_um * light.halation_size.max(0.05));
        let halo = blur::gaussian_blur_wide(&source, sigma);
        for (i, p) in layers.iter_mut().enumerate() {
            let k = (strength * stock.halation.layers[i]) as f32;
            if k > 0.0 {
                p.data_mut()
                    .par_iter_mut()
                    .zip(halo.data().par_iter())
                    .for_each(|(v, &s)| *v += k * s);
            }
        }
    }
    layers
}

struct Vignette {
    stops: f32,
    cx: f32,
    cy: f32,
    inv_r2: f32,
    ox: f32,
    oy: f32,
}

impl Vignette {
    fn new(opts: &RenderOptions, w: usize, h: usize) -> Self {
        let (origin, frame) = opts
            .region
            .map_or(([0, 0], [w, h]), |r| (r.origin, r.frame));
        let (fw, fh) = (frame[0] as f32, frame[1] as f32);
        Self {
            stops: opts.light.vignette.max(0.0) as f32,
            cx: fw / 2.0,
            cy: fh / 2.0,
            inv_r2: 4.0 / (fw * fw + fh * fh),
            ox: origin[0] as f32,
            oy: origin[1] as f32,
        }
    }

    /// Exposure factor: `2^(−stops · r⁴)`-shaped falloff (r = 1 in the corners), close to
    /// the cos⁴ law's gentle centre and faster edges.
    #[inline]
    fn factor(&self, x: usize, y: usize) -> f32 {
        if self.stops <= 0.0 {
            return 1.0;
        }
        let dx = self.ox + x as f32 + 0.5 - self.cx;
        let dy = self.oy + y as f32 + 0.5 - self.cy;
        let r2 = (dx * dx + dy * dy) * self.inv_r2;
        (-self.stops * r2 * r2 * std::f32::consts::LN_2).exp()
    }
}

/// Develop layer exposures into net dye densities with interlayer and edge effects
/// and grain.
pub fn develop(
    exposure: &[Plane; 3],
    stock: &FilmStock,
    film: &Developed,
    opts: &RenderOptions,
) -> [Plane; 3] {
    // Log exposure per layer.
    let mut log_h: [Plane; 3] = std::array::from_fn(|i| {
        let mut p = exposure[i].clone();
        p.data_mut()
            .par_iter_mut()
            .for_each(|v| *v = v.max(1e-7).log10());
        p
    });

    // Interimage effect: DIR couplers in each layer hold back development in proportion to
    // what the other layers develop, so each layer's record moves away from the mean of
    // the other two. Applied to log exposure, where a neutral is exactly equal in all
    // three layers, so colour differences grow and every neutral stays neutral.
    // Inhibition needs development: below the toe nothing develops, so log exposure is
    // floored there first. Without the floor, sensor noise in deep shadows (log exposures
    // far apart, all developing nothing) would be pushed up into one layer's straight line
    // and print as coloured specks.
    let c = (stock.interlayer * opts.interlayer.max(0.0)) as f32;
    if c > 0.0 {
        let floor = film.development_floor() as f32;
        for p in &mut log_h {
            p.data_mut().par_iter_mut().for_each(|v| *v = v.max(floor));
        }
        let [r, g, b] = &mut log_h;
        r.data_mut()
            .par_iter_mut()
            .zip(g.data_mut().par_iter_mut())
            .zip(b.data_mut().par_iter_mut())
            .for_each(|((r, g), b)| {
                let (xr, xg, xb) = (*r, *g, *b);
                *r = xr + c * (xr - 0.5 * (xg + xb));
                *g = xg + c * (xg - 0.5 * (xr + xb));
                *b = xb + c * (xb - 0.5 * (xr + xg));
            });
    }

    let luts = film.luts();
    let mut net: [Plane; 3] = std::array::from_fn(|i| {
        let mut p = std::mem::replace(&mut log_h[i], exposure[i].zeros_like());
        let lut = &luts[i];
        p.data_mut().par_iter_mut().for_each(|v| *v = lut.eval(*v));
        p
    });

    // Edge effects: density pushed away from its neighbourhood.
    let (a, reach_um) = stock.adjacency;
    if a > 0.0 {
        let sigma = opts.geometry.um_to_px(reach_um);
        if sigma > 0.3 {
            for p in &mut net {
                let around = blur::gaussian_blur(p, sigma);
                p.data_mut()
                    .par_iter_mut()
                    .zip(around.data().par_iter())
                    .for_each(|(d, &s)| *d = (*d + a as f32 * (*d - s)).max(0.0));
            }
        }
    }

    for (i, p) in net.iter_mut().enumerate() {
        let rms = film.grain_rms * stock.grain.layers[i];
        apply_grain(p, rms, stock.grain.clump_um, opts, i as u32);
    }
    net
}

/// Effective grain area (µm²) from diffuse RMS granularity (×1000, 48 µm aperture).
pub fn grain_area_um2(rms: f64) -> f64 {
    let sigma = rms / 1000.0;
    sigma * sigma * std::f64::consts::PI * 24.0 * 24.0 / 0.434
}

/// Selwyn grain in density: per-pixel σ² = 0.434 · a · D / pixel area (BW Darkroom's
/// model), one independent dye-cloud field per layer.
fn apply_grain(net: &mut Plane, rms: f64, clump_um: f64, opts: &RenderOptions, layer: u32) {
    let adjust = &opts.grain;
    if adjust.amount <= 0.0 || rms <= 0.0 {
        return;
    }
    let (w, h) = (net.width(), net.height());
    let origin = opts.region.map_or([0, 0], |r| r.origin);
    let film_sigma = opts.geometry.um_to_px(clump_um);
    let size = adjust.size.max(0.05);
    let stream = layer * 4;
    let mut field = grain_field(
        w,
        h,
        origin,
        opts.seed,
        stream,
        film_sigma * size,
        film_sigma,
    );
    let r = adjust.roughness.clamp(-1.0, 1.0);
    if r < -1e-6 {
        let other = grain_field(
            w,
            h,
            origin,
            opts.seed,
            stream + 1,
            film_sigma * size * 2.0,
            film_sigma,
        );
        let (a, b) = (1.0 + r, -r);
        let norm = 1.0 / (a * a + b * b).sqrt();
        let (a, b) = ((a * norm) as f32, (b * norm) as f32);
        field
            .data_mut()
            .par_iter_mut()
            .zip(other.data().par_iter())
            .for_each(|(f, &o)| *f = a * *f + b * o);
    } else if r > 1e-6 {
        let p = (1.0 + 0.6 * r) as f32;
        let before = std_dev(&field);
        field
            .data_mut()
            .par_iter_mut()
            .for_each(|f| *f = f.signum() * f.abs().powf(p));
        let after = std_dev(&field);
        if after > 0.0 {
            let gain = (before / after) as f32;
            field.data_mut().par_iter_mut().for_each(|f| *f *= gain);
        }
    }
    let k = (0.434 * grain_area_um2(rms) * adjust.amount.powi(2) / opts.geometry.pixel_area_um2())
        as f32;
    net.data_mut()
        .par_iter_mut()
        .zip(field.data().par_iter())
        .for_each(|(d, &n)| *d = (*d + (k * (d.max(0.0) + FOG_GRAIN_DENSITY)).sqrt() * n).max(0.0));
}

fn std_dev(p: &Plane) -> f64 {
    let n = p.data().len().max(1) as f64;
    let mean = p.data().iter().map(|&v| f64::from(v)).sum::<f64>() / n;
    (p.data()
        .iter()
        .map(|&v| (f64::from(v) - mean).powi(2))
        .sum::<f64>()
        / n)
        .sqrt()
}

/// White noise blurred to `sigma_px`, scaled to the per-pixel strength it would have at
/// `film_sigma_px`, keyed on full-frame coordinates so a crop has its region's grain.
fn grain_field(
    w: usize,
    h: usize,
    origin: [u32; 2],
    seed: u64,
    stream: u32,
    sigma_px: f64,
    film_sigma_px: f64,
) -> Plane {
    let margin = blur::gaussian_kernel(sigma_px).len() / 2;
    let (mw, mh) = (w + 2 * margin, h + 2 * margin);
    let (ox, oy) = (
        origin[0].wrapping_sub(margin as u32),
        origin[1].wrapping_sub(margin as u32),
    );
    let white = Plane::from_fn(mw, mh, |x, y| {
        noise::gaussian(
            seed,
            stream,
            ox.wrapping_add(x as u32),
            oy.wrapping_add(y as u32),
        )
    })
    .expect("dimensions come from an existing plane");
    let wide = blur::gaussian_blur(&white, sigma_px);
    let mut field = Plane::from_fn(w, h, |x, y| wide.get(x + margin, y + margin))
        .expect("dimensions come from an existing plane");
    if sigma_px != film_sigma_px {
        let gain = (blur::variance_factor_2d(film_sigma_px) / blur::variance_factor_2d(sigma_px))
            .sqrt() as f32;
        field.data_mut().par_iter_mut().for_each(|v| *v *= gain);
    }
    field
}

/// Display light through dyes of net densities `d`: `10^(−dyes · d)` per channel.
fn view_through_dyes(d: &[Plane; 3], dyes: &[[f64; 3]; 3]) -> [Plane; 3] {
    let a: [[f32; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| dyes[i][j] as f32));
    let mut out = [d[0].zeros_like(), d[0].zeros_like(), d[0].zeros_like()];
    let [or, og, ob] = &mut out;
    or.data_mut()
        .par_iter_mut()
        .zip(og.data_mut().par_iter_mut())
        .zip(ob.data_mut().par_iter_mut())
        .enumerate()
        .for_each(|(k, ((r, g), b))| {
            let c = [d[0].data()[k], d[1].data()[k], d[2].data()[k]];
            let t = |i: usize| 10f32.powf(-(a[i][0] * c[0] + a[i][1] * c[1] + a[i][2] * c[2]));
            *r = t(0);
            *g = t(1);
            *b = t(2);
        });
    out
}

/// Print the negative (net densities) onto `print` with `lights`, returning the print's
/// net dye densities.
pub fn print_negative(
    negative: &[Plane; 3],
    stock: &FilmStock,
    print: &PrintStock,
    lights: [f64; 3],
) -> [Plane; 3] {
    std::array::from_fn(|i| {
        // Exposure through the negative: lights minus the negative's total density.
        let offset = lights[i] - stock.base[i];
        let lut = Lut1D::from_fn(LOG_E_RANGE.0, LOG_E_RANGE.1, LUT_SIZE, |e| {
            print_density(print, i, e)
        })
        .expect("valid range");
        let mut p = negative[i].clone();
        p.data_mut()
            .par_iter_mut()
            .for_each(|d| *d = lut.eval(offset as f32 - *d));
        p
    })
}

/// The whole chain: expose, develop, then print (negative) or view (reversal).
pub fn render(
    scene: &SceneImage,
    stock: &FilmStock,
    print: &PrintStock,
    opts: &RenderOptions,
) -> Rendered {
    let film = Developed::new(stock, opts.push);
    let exposure = expose(scene, stock, opts);
    let density = develop(&exposure, stock, &film, opts);
    drop(exposure);
    let image = match stock.kind {
        FilmKind::Negative => {
            let lights = printer_lights(stock, &film, print, &opts.printing);
            let printed = print_negative(&density, stock, print, lights);
            view_through_dyes(&printed, &print.dyes)
        }
        FilmKind::Reversal => {
            let mut image = view_through_dyes(&density, &stock.dyes);
            // Projection brightness stands in for print exposure.
            let gain = 2f32.powf(-opts.printing.exposure_stops as f32);
            if gain != 1.0 {
                for p in &mut image {
                    p.data_mut()
                        .par_iter_mut()
                        .for_each(|v| *v = (*v * gain).min(1.0));
                }
            }
            image
        }
    };
    Rendered { image, density }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(w: usize, h: usize) -> RenderOptions {
        RenderOptions {
            seed: 1,
            // 20 µm pixels: a 6000-pixel-wide 35 mm frame's scale is 6 µm.
            geometry: FilmGeometry::from_frame_width(w as f64 * 0.02, w, h).unwrap(),
            ev_compensation: 0.0,
            push: 0.0,
            interlayer: 1.0,
            grain: GrainAdjust {
                amount: 0.0,
                ..Default::default()
            },
            light: LightEffects {
                halation: 0.0,
                ..Default::default()
            },
            printing: Printing {
                stock_look: false,
                ..Default::default()
            },
            region: None,
        }
    }

    fn patch(rgb: [f32; 3]) -> SceneImage {
        let p = |v| Plane::filled(8, 8, v).unwrap();
        SceneImage::Rgb {
            r: p(rgb[0]),
            g: p(rgb[1]),
            b: p(rgb[2]),
        }
    }

    fn centre(r: &Rendered) -> [f32; 3] {
        std::array::from_fn(|i| r.image[i].get(4, 4))
    }

    #[test]
    fn inverse_is_an_inverse() {
        let m = stock::print_stock("2383").unwrap().dyes;
        let inv = inverse(&m);
        for (i, row) in m.iter().enumerate() {
            for (j, expected) in [i == 0, i == 1, i == 2].into_iter().enumerate() {
                let v: f64 = row.iter().zip(&inv).map(|(a, inv_k)| a * inv_k[j]).sum();
                assert!((v - f64::from(u8::from(expected))).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn metered_grey_prints_as_neutral_mid_grey_on_every_stock_pair() {
        for stock in film_stocks() {
            for print in print_stocks() {
                let out = centre(&render(&patch([0.18; 3]), &stock, &print, &options(8, 8)));
                for c in out {
                    assert!(
                        (c - 0.18).abs() < 0.004,
                        "{} on {}: {out:?}",
                        stock.id,
                        print.id
                    );
                }
            }
        }
    }

    #[test]
    fn tones_are_monotonic_and_white_stays_below_paper_white() {
        let print = print_stock("endura").unwrap();
        for stock in film_stocks() {
            let mut last = 0.0;
            for k in -12..=10 {
                let v = 0.18 * 2f32.powf(k as f32 / 2.0);
                let out = centre(&render(&patch([v; 3]), &stock, &print, &options(8, 8)));
                let y = 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2];
                assert!(y >= last, "{} at {k}: {y} < {last}", stock.id);
                assert!(y <= 1.0);
                last = y;
            }
        }
    }

    #[test]
    fn interlayer_effect_saturates_colour_and_leaves_grey() {
        let stock = film_stock("ektar-100").unwrap();
        let print = print_stock("scan").unwrap();
        let red = patch([0.30, 0.12, 0.09]);
        let mut o = options(8, 8);
        o.interlayer = 0.0;
        let flat = centre(&render(&red, &stock, &print, &o));
        o.interlayer = 1.0;
        let rich = centre(&render(&red, &stock, &print, &o));
        assert!(
            rich[0] / rich[1] > flat[0] / flat[1] * 1.05,
            "{flat:?} → {rich:?}"
        );
        let grey = centre(&render(&patch([0.18; 3]), &stock, &print, &o));
        assert!((grey[0] - grey[2]).abs() < 0.003);
    }

    #[test]
    fn noisy_black_shadows_do_not_turn_into_coloured_specks() {
        // Sensor noise in a near-black sky: one channel at zero, the others tiny.
        let print = print_stock("scan").unwrap();
        for stock in film_stocks() {
            let out = centre(&render(
                &patch([0.0, 0.002, 0.0005]),
                &stock,
                &print,
                &options(8, 8),
            ));
            let black = centre(&render(&patch([0.0; 3]), &stock, &print, &options(8, 8)));
            for c in 0..3 {
                assert!(
                    (out[c] - black[c]).abs() < 0.01,
                    "{}: {out:?} vs black {black:?}",
                    stock.id
                );
            }
        }
    }

    #[test]
    fn print_film_is_contrastier_than_a_scan() {
        let stock = film_stock("vision3-250d").unwrap();
        let o = options(8, 8);
        let span = |print: &str| {
            let p = print_stock(print).unwrap();
            let lo = centre(&render(&patch([0.09; 3]), &stock, &p, &o))[1];
            let hi = centre(&render(&patch([0.36; 3]), &stock, &p, &o))[1];
            (hi / lo).log2()
        };
        // Two stops of scene stay about two stops on a scan; print film is contrastier.
        let (scan, cine) = (span("scan"), span("2383"));
        assert!(scan > 1.9 && cine > scan * 1.15, "scan {scan}, 2383 {cine}");
    }

    #[test]
    fn print_colour_points_add_the_named_colour() {
        let stock = film_stock("portra-400").unwrap();
        let print = print_stock("endura").unwrap();
        let mut o = options(8, 8);
        o.printing.colour_points = [6.0, 0.0, 0.0];
        let out = centre(&render(&patch([0.18; 3]), &stock, &print, &o));
        assert!(out[0] > out[1] * 1.1 && out[0] > out[2] * 1.1, "{out:?}");
        // Print exposure darkens.
        o.printing.colour_points = [0.0; 3];
        o.printing.exposure_stops = 1.0;
        let darker = centre(&render(&patch([0.18; 3]), &stock, &print, &o));
        assert!(darker[1] < 0.12, "{darker:?}");
    }

    #[test]
    fn halation_glows_red_round_a_bright_light_only() {
        let stock = film_stock("cinestill-800t").unwrap();
        let print = print_stock("scan").unwrap();
        let (w, h) = (200, 120);
        let lamp = |x: usize, y: usize| {
            if (x as i32 - 100).pow(2) + (y as i32 - 60).pow(2) < 16 {
                40.0
            } else {
                0.03
            }
        };
        let p = || Plane::from_fn(w, h, lamp).unwrap();
        let scene = SceneImage::Rgb {
            r: p(),
            g: p(),
            b: p(),
        };
        let mut o = options(w, h);
        o.light.halation = 1.0;
        let r = render(&scene, &stock, &print, &o);
        let near = |c: usize| r.image[c].get(112, 60);
        assert!(
            near(0) > near(2) * 1.5,
            "red halo: {} vs {}",
            near(0),
            near(2)
        );
        // Far away the halo has faded to (almost) the plain dark background.
        o.light.halation = 0.0;
        let plain = render(&scene, &stock, &print, &o);
        assert!((r.image[0].get(5, 5) - plain.image[0].get(5, 5)).abs() < 0.01);
    }

    #[test]
    fn grain_scales_with_granularity_and_crops_match_the_frame() {
        let stock = film_stock("portra-800").unwrap();
        let print = print_stock("scan").unwrap();
        let (w, h) = (160, 120);
        let scene = cd_core::SceneImage::Rgb {
            r: Plane::filled(w, h, 0.18).unwrap(),
            g: Plane::filled(w, h, 0.18).unwrap(),
            b: Plane::filled(w, h, 0.18).unwrap(),
        };
        let mut o = options(w, h);
        o.grain = GrainAdjust::default();
        let full = render(&scene, &stock, &print, &o);
        let std = std_dev(&full.density[1]);
        assert!(std > 0.005, "visible grain {std}");
        // The crop at (40, 30) has exactly that region's grain.
        let crop = cd_core::SceneImage::Rgb {
            r: Plane::filled(50, 40, 0.18).unwrap(),
            g: Plane::filled(50, 40, 0.18).unwrap(),
            b: Plane::filled(50, 40, 0.18).unwrap(),
        };
        o.region = Some(Region {
            origin: [40, 30],
            frame: [w, h],
        });
        let part = render(&crop, &stock, &print, &o);
        for (x, y) in [(0, 0), (10, 20), (49, 39)] {
            let a = part.density[1].get(x, y);
            let b = full.density[1].get(x + 40, y + 30);
            assert!((a - b).abs() < 1e-5, "({x},{y}) {a} vs {b}");
        }
    }

    #[test]
    fn push_raises_negative_contrast_and_slide_brightness() {
        let stock = film_stock("portra-400").unwrap();
        let normal = Developed::new(&stock, 0.0);
        let pushed = Developed::new(&stock, 2.0);
        let slope = |f: &Developed| f.net_density(1, 0.5) - f.net_density(1, -0.5);
        assert!(slope(&pushed) > slope(&normal) * 1.1);
        let slide = film_stock("velvia-50").unwrap();
        let n = Developed::new(&slide, 0.0).net_density(1, 0.0);
        let p = Developed::new(&slide, 1.0).net_density(1, 0.0);
        assert!(p < n - 0.2, "pushed slide is lighter: {n} → {p}");
    }

    #[test]
    fn vignette_darkens_corners_of_the_whole_frame() {
        let stock = film_stock("portra-400").unwrap();
        let print = print_stock("scan").unwrap();
        let scene = SceneImage::Mono {
            y: Plane::filled(90, 60, 0.18).unwrap(),
        };
        let mut o = options(90, 60);
        o.light.vignette = 1.0;
        let r = render(&scene, &stock, &print, &o);
        assert!(r.image[1].get(0, 0) < r.image[1].get(45, 30) * 0.8);
    }
}
