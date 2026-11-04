//! Separable Gaussian filtering.
//!
//! Used for emulsion light spread (irradiation), halation, bloom, developer adjacency
//! and edge effects, and
//! grain clump shaping. Each output pixel is computed independently in a fixed order,
//! so results do not depend on the number of threads.
//!
//! Borders replicate the edge pixel. A stage's [`halo_radius`] is the number of
//! pixels beyond a tile it reads, which tiled execution must provide (design 0001 §4).

use crate::plane::Plane;
use rayon::prelude::*;

/// Kernel support in standard deviations. Truncation at 4σ discards < 1e-4 of the mass.
const TRUNCATE_SIGMAS: f64 = 4.0;

/// Below this σ (in pixels) the kernel is treated as the identity.
const MIN_SIGMA_PX: f64 = 1e-3;

/// Pixels read on each side of an output pixel for a given σ.
pub fn halo_radius(sigma_px: f64) -> usize {
    if sigma_px < MIN_SIGMA_PX {
        0
    } else {
        (TRUNCATE_SIGMAS * sigma_px).ceil() as usize
    }
}

/// Normalised, truncated 1-D Gaussian taps (sum = 1), length `2r + 1`.
pub fn gaussian_kernel(sigma_px: f64) -> Vec<f32> {
    let r = halo_radius(sigma_px);
    if r == 0 {
        return vec![1.0];
    }
    let taps: Vec<f64> = (-(r as i64)..=r as i64)
        .map(|i| (-(i * i) as f64 / (2.0 * sigma_px * sigma_px)).exp())
        .collect();
    let sum: f64 = taps.iter().sum();
    taps.into_iter().map(|t| (t / sum) as f32).collect()
}

/// Sum of squared taps of the full 2-D kernel.
///
/// Filtering unit-variance white noise with a normalised kernel scales the per-pixel
/// variance by this factor while leaving large-aperture variance unchanged. The grain
/// model (design 0001 §5.3) relies on this property.
pub fn variance_factor_2d(sigma_px: f64) -> f64 {
    let k1: f64 = gaussian_kernel(sigma_px)
        .iter()
        .map(|&t| f64::from(t) * f64::from(t))
        .sum();
    k1 * k1
}

/// Gaussian blur with standard deviation `sigma_px` (in pixels).
pub fn gaussian_blur(input: &Plane, sigma_px: f64) -> Plane {
    let kernel = gaussian_kernel(sigma_px);
    if kernel.len() == 1 {
        return input.clone();
    }
    let horizontal = convolve_rows(input, &kernel);
    convolve_columns(&horizontal, &kernel)
}

/// Above this σ (pixels), [`gaussian_blur_wide`] filters a reduced copy.
const WIDE_SIGMA_PX: f64 = 8.0;

/// Gaussian blur for wide kernels (halation, bloom), where an exact separable blur
/// would cost hundreds of taps per pixel. The plane is box-reduced by a factor `k`, blurred
/// with the remaining σ, and enlarged again bilinearly. The reduction and enlargement add
/// roughly `(k² − 1)/12 + k²/6` px² of variance, which is subtracted from the blur, so
/// the overall spread matches `sigma_px` closely. Small σ uses the exact blur.
pub fn gaussian_blur_wide(input: &Plane, sigma_px: f64) -> Plane {
    if sigma_px <= WIDE_SIGMA_PX {
        return gaussian_blur(input, sigma_px);
    }
    let (w, h) = (input.width(), input.height());
    let k = ((sigma_px / 4.0).floor() as usize).clamp(2, w.min(h).max(2));
    let (sw, sh) = (w.div_ceil(k), h.div_ceil(k));
    // Box reduction; partial blocks at the right and bottom edges average what they cover.
    let mut small = Plane::filled(sw, sh, 0.0).expect("non-zero size");
    small
        .data_mut()
        .par_chunks_mut(sw)
        .enumerate()
        .for_each(|(sy, row)| {
            let y0 = sy * k;
            let y1 = (y0 + k).min(h);
            for (sx, o) in row.iter_mut().enumerate() {
                let x0 = sx * k;
                let x1 = (x0 + k).min(w);
                let mut s = 0.0f32;
                for y in y0..y1 {
                    s += input.row(y)[x0..x1].iter().sum::<f32>();
                }
                *o = s / ((y1 - y0) * (x1 - x0)) as f32;
            }
        });
    let kf = k as f64;
    let extra = (kf * kf - 1.0) / 12.0 + kf * kf / 6.0;
    let rest = (sigma_px * sigma_px - extra).max(0.25).sqrt() / kf;
    let small = gaussian_blur(&small, rest);
    // Bilinear enlargement, sampling block centres.
    let mut out = input.zeros_like();
    out.data_mut()
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, row)| {
            let fy = ((y as f64 + 0.5) / kf - 0.5).clamp(0.0, (sh - 1) as f64);
            let y0 = fy.floor() as usize;
            let y1 = (y0 + 1).min(sh - 1);
            let ty = (fy - y0 as f64) as f32;
            let (r0, r1) = (small.row(y0), small.row(y1));
            for (x, o) in row.iter_mut().enumerate() {
                let fx = ((x as f64 + 0.5) / kf - 0.5).clamp(0.0, (sw - 1) as f64);
                let x0 = fx.floor() as usize;
                let x1 = (x0 + 1).min(sw - 1);
                let tx = (fx - x0 as f64) as f32;
                let top = r0[x0] + (r0[x1] - r0[x0]) * tx;
                let bottom = r1[x0] + (r1[x1] - r1[x0]) * tx;
                *o = top + (bottom - top) * ty;
            }
        });
    out
}

fn convolve_rows(input: &Plane, kernel: &[f32]) -> Plane {
    let (w, r) = (input.width(), kernel.len() / 2);
    let mut out = input.zeros_like();
    out.data_mut()
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, out_row)| {
            let row = input.row(y);
            for (x, o) in out_row.iter_mut().enumerate() {
                let mut acc = 0.0f32;
                for (k, &tap) in kernel.iter().enumerate() {
                    let sx = (x + k).saturating_sub(r).min(w - 1);
                    acc += tap * row[sx];
                }
                *o = acc;
            }
        });
    out
}

/// Vertical pass written row-wise for cache locality: each output row accumulates
/// whole neighbouring input rows.
fn convolve_columns(input: &Plane, kernel: &[f32]) -> Plane {
    let (w, h, r) = (input.width(), input.height(), kernel.len() / 2);
    let mut out = input.zeros_like();
    out.data_mut()
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, out_row)| {
            for (k, &tap) in kernel.iter().enumerate() {
                let sy = (y + k).saturating_sub(r).min(h - 1);
                for (o, &v) in out_row.iter_mut().zip(input.row(sy)) {
                    *o += tap * v;
                }
            }
        });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::gaussian;

    #[test]
    fn kernel_is_normalised_and_symmetric() {
        for sigma in [0.3, 1.0, 2.5, 7.0] {
            let k = gaussian_kernel(sigma);
            let sum: f32 = k.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6, "σ={sigma} sum={sum}");
            assert!(k.iter().eq(k.iter().rev()));
        }
        assert_eq!(gaussian_kernel(0.0), vec![1.0]);
    }

    #[test]
    fn preserves_uniform_fields() {
        let p = Plane::filled(37, 23, 1.25).unwrap();
        let b = gaussian_blur(&p, 3.0);
        assert!(b.data().iter().all(|&v| (v - 1.25).abs() < 1e-6));
    }

    #[test]
    fn impulse_response_has_unit_mass_and_expected_width() {
        let n = 65;
        let mut p = Plane::filled(n, n, 0.0).unwrap();
        p.set(32, 32, 1.0);
        let sigma = 2.0;
        let b = gaussian_blur(&p, sigma);
        let mass: f64 = b.data().iter().map(|&v| f64::from(v)).sum();
        assert!((mass - 1.0).abs() < 1e-5);
        let var_x: f64 = (0..n)
            .flat_map(|y| (0..n).map(move |x| (x, y)))
            .map(|(x, y)| f64::from(b.get(x, y)) * ((x as f64 - 32.0).powi(2)))
            .sum();
        // Truncating at 4σ removes ~0.1% of the variance (2·4·φ(4) ≈ 1.1e-3).
        assert!((var_x / (sigma * sigma) - 1.0).abs() < 2e-3, "var {var_x}");
    }

    #[test]
    fn white_noise_variance_scales_by_kernel_factor() {
        let n = 512;
        let sigma = 1.5;
        let noise = Plane::from_fn(n, n, |x, y| gaussian(3, 0, x as u32, y as u32)).unwrap();
        let b = gaussian_blur(&noise, sigma);
        let r = halo_radius(sigma);
        let interior: Vec<f64> = (r..n - r)
            .flat_map(|y| (r..n - r).map(move |x| (x, y)))
            .map(|(x, y)| f64::from(b.get(x, y)))
            .collect();
        let var = interior.iter().map(|v| v * v).sum::<f64>() / interior.len() as f64;
        let expected = variance_factor_2d(sigma);
        assert!(
            (var / expected - 1.0).abs() < 0.03,
            "var {var}, expected {expected}"
        );
    }

    #[test]
    fn wide_blur_keeps_mass_and_spread() {
        let n = 401;
        let mut p = Plane::filled(n, n, 0.0).unwrap();
        // A small square source in the middle (an impulse would alias in the reduction).
        for y in 196..205 {
            for x in 196..205 {
                p.set(x, y, 1.0 / 81.0);
            }
        }
        let sigma = 30.0;
        let b = gaussian_blur_wide(&p, sigma);
        let mass: f64 = b.data().iter().map(|&v| f64::from(v)).sum();
        assert!((mass - 1.0).abs() < 0.02, "mass {mass}");
        let var_x: f64 = (0..n)
            .flat_map(|y| (0..n).map(move |x| (x, y)))
            .map(|(x, y)| f64::from(b.get(x, y)) * (x as f64 - 200.0).powi(2))
            .sum::<f64>()
            / mass;
        // Source variance (9² − 1)/12 ≈ 6.7 px² adds to σ².
        let expected = sigma * sigma + 6.7;
        assert!(
            (var_x / expected - 1.0).abs() < 0.08,
            "var {var_x} vs {expected}"
        );
        let flat = gaussian_blur_wide(&Plane::filled(90, 50, 0.5).unwrap(), 20.0);
        assert!(flat.data().iter().all(|&v| (v - 0.5).abs() < 1e-5));
    }

    #[test]
    fn identical_across_thread_counts() {
        let p = Plane::from_fn(300, 200, |x, y| gaussian(5, 0, x as u32, y as u32)).unwrap();
        let run = |threads| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| gaussian_blur(&p, 2.2))
        };
        let one = run(1);
        let many = run(8);
        assert!(
            one.data()
                .iter()
                .zip(many.data())
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
}
