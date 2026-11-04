//! Film compression: fit a digital photo's highlights into the range film records, before
//! the film sees it (design 0001 C10).
//!
//! A sensor clips abruptly; a negative keeps absorbing light and rolls highlights off over
//! many stops, so heavily exposed highlights crowd towards the mid-tones and the image
//! flattens. This stage reproduces that on scene luminance, in stops relative to the metered
//! grey (`u = log2(Y / 0.18)`):
//!
//! - **Tonal range** places the knee: 0 puts it above anything a photo holds (no effect),
//!   100 almost down in the deep shadows. Below the knee nothing changes.
//! - **Impact** lowers the slope above the knee, from 1 (none) to 0.25 at 100: highlights
//!   are pulled towards the mid-tones. The knee is soft (about a stop wide).
//! - **White point** stretches (below 100) or squeezes (above 100) the compressed part:
//!   lower gives more highlight contrast and clips sooner; higher gives flatter, greyer
//!   highlights.
//! - **Colour density** is the highlights' saturation: 100 keeps each pixel's colour ratios
//!   (slide-like), 0 lets the compressed highlights fade towards neutral (negative-like).
//!
//! It is a creative control in the spirit of the film's own shoulder, not a separate
//! physical stage: the film's curve still applies afterwards.

use fd_core::Plane;
use rayon::prelude::*;

use crate::GREY;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Compression {
    /// Strength, 0–100. 0 is off.
    pub impact: f64,
    /// How far down the tonal range compression reaches, 0–100.
    pub tonal_range: f64,
    /// 50–120; 100 is neutral.
    pub white_point: f64,
    /// Highlight saturation, 0–100.
    pub colour_density: f64,
}

impl Default for Compression {
    fn default() -> Self {
        Self {
            impact: 0.0,
            tonal_range: 50.0,
            white_point: 100.0,
            colour_density: 100.0,
        }
    }
}

/// Width of the soft knee, stops.
const KNEE_WIDTH: f64 = 0.5;

impl Compression {
    pub fn is_off(&self) -> bool {
        self.impact <= 0.0
    }

    /// Knee position in stops above the metered grey.
    pub fn knee_stops(&self) -> f64 {
        4.0 - 7.0 * self.tonal_range.clamp(0.0, 100.0) / 100.0
    }

    fn slope(&self) -> f64 {
        1.0 - 0.75 * self.impact.clamp(0.0, 100.0) / 100.0
    }

    /// Output stops for input stops `u` (both relative to the metered grey).
    pub fn map_stops(&self, u: f64) -> f64 {
        if self.is_off() {
            return u;
        }
        let d = u - self.knee_stops();
        // Soft knee: `w·ln(1 + e^(d/w))` is 0 well below the knee and `d` well above.
        let z = d / KNEE_WIDTH;
        let above = if z > 30.0 {
            d
        } else {
            KNEE_WIDTH * z.exp().ln_1p()
        };
        let compressed = self.slope() * above * 100.0 / self.white_point.clamp(50.0, 120.0);
        u - above + compressed
    }

    /// How much of the compression applies at `u`, 0–1 (for colour density).
    fn weight(&self, u: f64) -> f64 {
        let z = (u - self.knee_stops()) / KNEE_WIDTH;
        (1.0 - self.slope()) / (1.0 + (-z).exp())
    }

    /// Curve samples `(input stops, output stops)` for plotting.
    pub fn curve(&self, lo: f64, hi: f64, n: usize) -> Vec<(f64, f64)> {
        (0..n)
            .map(|k| {
                let u = lo + (hi - lo) * k as f64 / (n - 1) as f64;
                (u, self.map_stops(u))
            })
            .collect()
    }

    /// Compress scene-linear Rec.709 planes in place.
    pub fn apply(&self, rgb: &mut [Plane; 3]) {
        if self.is_off() {
            return;
        }
        let fade = (1.0 - self.colour_density.clamp(0.0, 100.0) / 100.0) as f32;
        let [r, g, b] = rgb;
        r.data_mut()
            .par_iter_mut()
            .zip(g.data_mut().par_iter_mut())
            .zip(b.data_mut().par_iter_mut())
            .for_each(|((r, g), b)| {
                let y = 0.2126 * *r + 0.7152 * *g + 0.0722 * *b;
                if y <= 1e-6 {
                    return;
                }
                let u = (f64::from(y) / GREY).log2();
                let gain = 2f64.powf(self.map_stops(u) - u) as f32;
                let y2 = y * gain;
                // Colour ratios kept, then faded towards neutral as the highlights compress.
                let keep = 1.0 - fade * self.weight(u) as f32 / 0.75;
                for c in [r, g, b] {
                    *c = y2 + (*c * gain - y2) * keep;
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> Compression {
        Compression {
            impact: 80.0,
            ..Default::default()
        }
    }

    #[test]
    fn off_by_default_and_identity_when_off() {
        let c = Compression::default();
        assert!(c.is_off());
        for u in [-6.0, 0.0, 3.0, 8.0] {
            assert_eq!(c.map_stops(u), u);
        }
    }

    #[test]
    fn highlights_move_towards_the_mid_tones_shadows_stay() {
        let c = on();
        // Default tonal range: knee half a stop above grey.
        assert!((c.map_stops(-3.0) - -3.0).abs() < 0.01, "shadows untouched");
        let six = c.map_stops(6.0);
        assert!(six < 3.0, "+6 stops compressed to {six}");
        // Monotonic everywhere: no tone reversals.
        let mut last = f64::MIN;
        for k in -80..=100 {
            let v = c.map_stops(k as f64 / 10.0);
            assert!(v > last);
            last = v;
        }
    }

    #[test]
    fn tonal_range_and_white_point_do_what_they_say() {
        let reach = |t| {
            Compression {
                tonal_range: t,
                ..on()
            }
            .map_stops(-1.0)
        };
        assert!((reach(0.0) - -1.0).abs() < 1e-3, "0: no compression");
        assert!(reach(100.0) < -1.3, "100 reaches down past mid-grey");
        let top = |w| {
            Compression {
                white_point: w,
                ..on()
            }
            .map_stops(5.0)
        };
        assert!(top(50.0) > top(100.0) && top(100.0) > top(120.0));
    }

    #[test]
    fn colour_density_fades_compressed_highlights_only() {
        let run = |density: f64, rgb: [f32; 3]| {
            let mut p: [Plane; 3] = std::array::from_fn(|i| Plane::filled(1, 1, rgb[i]).unwrap());
            Compression {
                colour_density: density,
                ..on()
            }
            .apply(&mut p);
            [p[0].get(0, 0), p[1].get(0, 0), p[2].get(0, 0)]
        };
        let sat = |c: [f32; 3]| c[0] / c[2];
        let bright = [6.0, 3.0, 1.5];
        assert!(sat(run(0.0, bright)) < sat(run(100.0, bright)) * 0.8);
        assert!(
            (sat(run(100.0, bright)) - 4.0).abs() < 1e-3,
            "100 keeps ratios"
        );
        let dark = [0.04, 0.02, 0.01];
        assert!(
            (sat(run(0.0, dark)) - 4.0).abs() < 0.05,
            "shadows keep colour"
        );
    }
}
