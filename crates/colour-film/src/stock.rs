//! Film and print stock definitions.
//!
//! **Every value here is illustrative.** The parameters were chosen to give each stock the
//! character photographers describe for the product it is named after (contrast, colour
//! saturation, grain, halation, warmth); none is fitted to a manufacturer's curves yet.
//! The product names describe the intended target only. See design 0001 §8 for how
//! datasheet calibration will replace them, as BW Darkroom did for its films.

use fd_core::CalibrationLevel;

/// Layer order everywhere: red-sensitive (cyan dye), green-sensitive (magenta dye),
/// blue-sensitive (yellow dye). Display and scene channels are R, G, B in the same order.
pub const LAYERS: [&str; 3] = ["red", "green", "blue"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilmKind {
    /// Colour negative (C-41 / ECN-2): low contrast, orange mask, printed or scanned.
    Negative,
    /// Colour reversal (E-6 slide): a direct positive viewed on a light box.
    Reversal,
}

/// One dye layer's response.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    /// Straight-line slope: net density per unit log10 exposure. For reversal film this is
    /// the magnitude of the (falling) slope at mid-grey.
    pub gamma: f64,
    /// Sensitivity offset in log10 exposure: positive makes the layer faster.
    pub speed: f64,
}

/// Halation: light that passes through the emulsion, reflects off the film base and
/// re-exposes the layers from behind (red first, as the red layer sits at the bottom).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Halation {
    /// Fraction of the light reaching the base that comes back. Anti-halation backings keep
    /// this to about a percent; film without one (remjet removed) returns far more.
    pub strength: f64,
    /// Spread of the returned light as a Gaussian σ on the film, µm.
    pub radius_um: f64,
    /// Share of the returned light each layer receives (R, G, B).
    pub layers: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grain {
    /// Diffuse RMS granularity (×1000, 48 µm aperture) at net density 1.0: the same
    /// measure as BW Darkroom's films, so the Selwyn model and readout carry over.
    pub rms: f64,
    /// Dye-cloud correlation scale (Gaussian σ), µm. Dye clouds are several times larger
    /// than silver grains, so colour grain is softer per pixel at the same granularity.
    pub clump_um: f64,
    /// Relative granularity of each layer: fast blue-sensitive layers are grainier.
    pub layers: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilmStock {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: FilmKind,
    pub iso: f64,
    /// One line on the stock's character, for the UI.
    pub about: &'static str,
    /// Layer exposures from scene-linear Rec.709 (rows: R, G, B layer; columns: scene
    /// R, G, B). Each row sums to 1, so a neutral exposes all three layers equally (the film
    /// is balanced for the light the photo was white-balanced to). Off-diagonal terms are
    /// the overlap of the layers' spectral sensitivities: positive desaturates, negative
    /// (an inhibiting fourth layer) sharpens a colour.
    pub sensitivity: [[f64; 3]; 3],
    pub layers: [Layer; 3],
    /// Negative: log10 exposure (relative to the metered grey, 0) where the toe turns
    /// into the straight line. Reversal: unused (grey is placed at a neutral density).
    pub toe_log_h: f64,
    /// Width of the toe transition in log10 exposure (softness). For reversal film the toe
    /// is the highlight end.
    pub toe_width: f64,
    /// Negative: net density the shoulder approaches. Reversal: Dmax − Dmin.
    pub density_range: f64,
    /// Base + fog density per layer (the orange mask for negatives; Dmin for reversal).
    pub base: [f64; 3],
    /// Interimage (DIR coupler) strength: how much each layer's development is held back
    /// by its neighbours' density. Raises saturation without changing neutrals.
    pub interlayer: f64,
    /// Edge-effect strength in density and its reach in µm (DIR couplers act locally too).
    pub adjacency: (f64, f64),
    pub grain: Grain,
    pub halation: Halation,
    /// Reversal only: unwanted absorptions of the film's own dyes (rows: display R, G, B;
    /// columns: cyan, magenta, yellow dye at unit density). Negatives use the print's.
    pub dyes: [[f64; 3]; 3],
    /// Printer points a lab typically adds to this stock's prints, as (red, green, blue)
    /// colour in the print: an interpretation of each stock's familiar rendering.
    pub look_points: [f64; 3],
    pub level: CalibrationLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintKind {
    /// Motion-picture print film, projected: high contrast, deep blacks.
    Film,
    /// Colour paper (RA-4), viewed by reflection.
    Paper,
    /// A neutral scan of the negative: ideal dyes, gentle curve.
    Scan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrintStock {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: PrintKind,
    pub about: &'static str,
    /// Slope at mid-scale per layer (C, M, Y dye layers exposed by red, green, blue light).
    pub gamma: [f64; 3],
    /// Maximum net density (above Dmin) per layer.
    pub dmax: [f64; 3],
    /// Width of the toe in log10 exposure: how gently highlights roll off to white.
    pub toe: f64,
    /// Unwanted absorptions of the print dyes (rows: display R, G, B; columns: C, M, Y).
    pub dyes: [[f64; 3]; 3],
    pub level: CalibrationLevel,
}

const fn layer(gamma: f64, speed: f64) -> Layer {
    Layer { gamma, speed }
}

/// Typical subtractive dye crosstalk for colour materials: cyan absorbs some green,
/// magenta some blue and red, yellow a little green.
const TYPICAL_DYES: [[f64; 3]; 3] = [[1.0, 0.10, 0.02], [0.20, 1.0, 0.10], [0.05, 0.28, 1.0]];
const IDEAL_DYES: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

const NEG_BASE: [f64; 3] = [0.22, 0.58, 0.82];
const CINE_BASE: [f64; 3] = [0.20, 0.62, 0.90];
const SLIDE_BASE: [f64; 3] = [0.12, 0.13, 0.15];

/// Anti-halation backing in place: a faint, tight red halo.
const BACKED: Halation = Halation {
    strength: 0.012,
    radius_um: 90.0,
    layers: [1.0, 0.25, 0.0],
};

pub fn film_stocks() -> Vec<FilmStock> {
    use FilmKind::*;
    let il = CalibrationLevel::Illustrative;
    vec![
        FilmStock {
            id: "portra-160",
            name: "Portra 160",
            kind: Negative,
            iso: 160.0,
            about: "Fine-grained portrait negative: soft contrast, gentle saturation, natural skin.",
            sensitivity: [[0.86, 0.12, 0.02], [0.07, 0.85, 0.08], [0.01, 0.09, 0.90]],
            layers: [layer(0.55, 0.0), layer(0.56, 0.0), layer(0.54, 0.0)],
            toe_log_h: -1.55,
            toe_width: 0.28,
            density_range: 2.6,
            base: NEG_BASE,
            interlayer: 0.18,
            adjacency: (0.10, 18.0),
            grain: Grain {
                rms: 8.0,
                clump_um: 5.0,
                layers: [0.9, 1.0, 1.15],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [1.0, 0.0, -1.0],
            level: il,
        },
        FilmStock {
            id: "portra-400",
            name: "Portra 400",
            kind: Negative,
            iso: 400.0,
            about: "The all-round portrait negative: wide latitude, warm skin, restrained colour.",
            sensitivity: [[0.86, 0.12, 0.02], [0.07, 0.85, 0.08], [0.01, 0.09, 0.90]],
            layers: [layer(0.58, 0.0), layer(0.58, 0.0), layer(0.56, 0.0)],
            toe_log_h: -1.6,
            toe_width: 0.3,
            density_range: 2.6,
            base: NEG_BASE,
            interlayer: 0.22,
            adjacency: (0.12, 20.0),
            grain: Grain {
                rms: 11.0,
                clump_um: 5.75,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [1.5, 0.0, -1.5],
            level: il,
        },
        FilmStock {
            id: "portra-800",
            name: "Portra 800",
            kind: Negative,
            iso: 800.0,
            about: "Fast portrait negative: richer colour than 400, visible grain, warm.",
            sensitivity: [[0.87, 0.11, 0.02], [0.07, 0.86, 0.07], [0.01, 0.09, 0.90]],
            layers: [layer(0.62, 0.0), layer(0.61, 0.0), layer(0.59, -0.02)],
            toe_log_h: -1.5,
            toe_width: 0.3,
            density_range: 2.5,
            base: NEG_BASE,
            interlayer: 0.28,
            adjacency: (0.12, 22.0),
            grain: Grain {
                rms: 15.0,
                clump_um: 6.75,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [2.0, 0.0, -2.0],
            level: il,
        },
        FilmStock {
            id: "ektar-100",
            name: "Ektar 100",
            kind: Negative,
            iso: 100.0,
            about: "Ultra-saturated, very fine-grained landscape negative: vivid reds and blues.",
            sensitivity: [[0.93, 0.07, 0.00], [0.04, 0.92, 0.04], [0.00, 0.05, 0.95]],
            layers: [layer(0.68, 0.0), layer(0.68, 0.0), layer(0.66, 0.0)],
            toe_log_h: -1.4,
            toe_width: 0.22,
            density_range: 2.7,
            base: NEG_BASE,
            interlayer: 0.48,
            adjacency: (0.15, 16.0),
            grain: Grain {
                rms: 6.5,
                clump_um: 4.62,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [0.0, 0.0, 0.0],
            level: il,
        },
        FilmStock {
            id: "gold-200",
            name: "Gold 200",
            kind: Negative,
            iso: 200.0,
            about: "Consumer negative: golden warmth, punchy mid-tones, nostalgic snapshot colour.",
            sensitivity: [[0.88, 0.11, 0.01], [0.06, 0.87, 0.07], [0.01, 0.10, 0.89]],
            layers: [layer(0.64, 0.02), layer(0.63, 0.0), layer(0.58, -0.04)],
            toe_log_h: -1.4,
            toe_width: 0.3,
            density_range: 2.5,
            base: NEG_BASE,
            interlayer: 0.32,
            adjacency: (0.14, 20.0),
            grain: Grain {
                rms: 11.0,
                clump_um: 5.75,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [3.0, 1.0, -4.0],
            level: il,
        },
        FilmStock {
            id: "ultramax-400",
            name: "UltraMax 400",
            kind: Negative,
            iso: 400.0,
            about: "Fast consumer negative: saturated, contrasty, warm and gritty.",
            sensitivity: [[0.89, 0.10, 0.01], [0.06, 0.88, 0.06], [0.01, 0.09, 0.90]],
            layers: [layer(0.68, 0.0), layer(0.66, 0.0), layer(0.62, -0.02)],
            toe_log_h: -1.35,
            toe_width: 0.3,
            density_range: 2.5,
            base: NEG_BASE,
            interlayer: 0.36,
            adjacency: (0.16, 22.0),
            grain: Grain {
                rms: 16.0,
                clump_um: 7.0,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [2.0, 0.5, -2.5],
            level: il,
        },
        FilmStock {
            id: "pro-400h",
            name: "Pro 400H",
            kind: Negative,
            iso: 400.0,
            about: "Fuji's four-layer portrait negative: airy pastels, cool cyan-green cast, soft.",
            // A fourth, cyan-sensitive layer inhibits the red layer under blue-green light.
            sensitivity: [[0.98, 0.12, -0.10], [0.06, 0.86, 0.08], [0.00, 0.09, 0.91]],
            layers: [layer(0.54, -0.02), layer(0.57, 0.0), layer(0.56, 0.0)],
            toe_log_h: -1.65,
            toe_width: 0.32,
            density_range: 2.6,
            base: NEG_BASE,
            interlayer: 0.16,
            adjacency: (0.10, 20.0),
            grain: Grain {
                rms: 11.0,
                clump_um: 5.75,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [-1.5, 1.0, 1.0],
            level: il,
        },
        FilmStock {
            id: "superia-400",
            name: "Superia 400",
            kind: Negative,
            iso: 400.0,
            about: "Fuji consumer negative: crisp greens, cool shadows, punchy everyday colour.",
            sensitivity: [[0.97, 0.11, -0.08], [0.05, 0.89, 0.06], [0.00, 0.08, 0.92]],
            layers: [layer(0.66, -0.02), layer(0.67, 0.03), layer(0.64, 0.0)],
            toe_log_h: -1.4,
            toe_width: 0.28,
            density_range: 2.5,
            base: NEG_BASE,
            interlayer: 0.34,
            adjacency: (0.15, 20.0),
            grain: Grain {
                rms: 14.0,
                clump_um: 6.5,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [-0.5, 1.5, 0.5],
            level: il,
        },
        FilmStock {
            id: "vision3-250d",
            name: "Vision3 250D",
            kind: Negative,
            iso: 250.0,
            about: "Daylight cinema negative: enormous latitude, low contrast, made for grading.",
            sensitivity: [[0.88, 0.11, 0.01], [0.06, 0.87, 0.07], [0.01, 0.09, 0.90]],
            layers: [layer(0.56, 0.0), layer(0.57, 0.0), layer(0.56, 0.0)],
            toe_log_h: -1.75,
            toe_width: 0.3,
            density_range: 3.0,
            base: CINE_BASE,
            interlayer: 0.24,
            adjacency: (0.12, 18.0),
            grain: Grain {
                rms: 9.0,
                clump_um: 5.25,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [0.0, 0.0, 0.0],
            level: il,
        },
        FilmStock {
            id: "vision3-500t",
            name: "Vision3 500T",
            kind: Negative,
            iso: 500.0,
            about: "Tungsten cinema negative: night-time colour, wide latitude, visible grain.\nBalanced for 3200 K: set White balance to Tungsten under lamps, or Daylight for its blue cast outdoors.",
            sensitivity: [[0.88, 0.11, 0.01], [0.06, 0.87, 0.07], [0.01, 0.10, 0.89]],
            layers: [layer(0.58, 0.0), layer(0.58, 0.0), layer(0.57, 0.0)],
            toe_log_h: -1.7,
            toe_width: 0.3,
            density_range: 3.0,
            base: CINE_BASE,
            interlayer: 0.24,
            adjacency: (0.12, 20.0),
            grain: Grain {
                rms: 13.0,
                clump_um: 6.25,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: TYPICAL_DYES,
            look_points: [0.0, 0.0, 0.0],
            level: il,
        },
        FilmStock {
            id: "cinestill-800t",
            name: "CineStill 800T",
            kind: Negative,
            iso: 800.0,
            about: "Tungsten cinema film without its remjet backing: glowing red halos round every light.\nSet White balance to Tungsten under lamps, or Daylight for the cold look outdoors.",
            sensitivity: [[0.88, 0.11, 0.01], [0.06, 0.87, 0.07], [0.01, 0.10, 0.89]],
            layers: [layer(0.60, 0.0), layer(0.60, 0.0), layer(0.58, 0.0)],
            toe_log_h: -1.6,
            toe_width: 0.3,
            density_range: 2.9,
            base: [0.16, 0.48, 0.70],
            interlayer: 0.26,
            adjacency: (0.12, 22.0),
            grain: Grain {
                rms: 16.0,
                clump_um: 7.0,
                layers: [0.9, 1.0, 1.2],
            },
            halation: Halation {
                strength: 0.16,
                radius_um: 260.0,
                layers: [1.0, 0.18, 0.0],
            },
            dyes: TYPICAL_DYES,
            look_points: [0.0, 0.0, 0.0],
            level: il,
        },
        FilmStock {
            id: "ektachrome-e100",
            name: "Ektachrome E100",
            kind: Reversal,
            iso: 100.0,
            about: "Slide film: clean, neutral-to-cool colour, deep blacks, very fine grain.",
            sensitivity: [[0.91, 0.08, 0.01], [0.05, 0.90, 0.05], [0.00, 0.06, 0.94]],
            layers: [layer(1.75, 0.0), layer(1.75, 0.0), layer(1.72, 0.0)],
            toe_log_h: 0.0,
            toe_width: 0.2,
            density_range: 3.3,
            base: SLIDE_BASE,
            interlayer: 0.30,
            adjacency: (0.10, 14.0),
            grain: Grain {
                rms: 8.0,
                clump_um: 5.0,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: [[1.0, 0.12, 0.03], [0.18, 1.0, 0.12], [0.04, 0.22, 1.0]],
            look_points: [-0.5, 0.0, 0.5],
            level: il,
        },
        FilmStock {
            id: "velvia-50",
            name: "Velvia 50",
            kind: Reversal,
            iso: 50.0,
            about: "The landscape slide film: extreme saturation, contrasty, magenta-leaning greens and deep blues.",
            sensitivity: [[0.97, 0.06, -0.03], [0.02, 0.96, 0.02], [-0.02, 0.04, 0.98]],
            layers: [layer(2.05, 0.0), layer(2.05, 0.0), layer(2.0, 0.0)],
            toe_log_h: 0.0,
            toe_width: 0.2,
            density_range: 3.5,
            base: SLIDE_BASE,
            interlayer: 0.62,
            adjacency: (0.14, 14.0),
            grain: Grain {
                rms: 7.0,
                clump_um: 4.75,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: [[1.0, 0.14, 0.03], [0.16, 1.0, 0.14], [0.03, 0.20, 1.0]],
            look_points: [0.5, -1.0, 0.0],
            level: il,
        },
        FilmStock {
            id: "provia-100f",
            name: "Provia 100F",
            kind: Reversal,
            iso: 100.0,
            about: "Fuji's standard slide film: accurate colour, moderate saturation, fine grain.",
            sensitivity: [[0.93, 0.07, 0.00], [0.04, 0.92, 0.04], [0.00, 0.05, 0.95]],
            layers: [layer(1.8, 0.0), layer(1.8, 0.0), layer(1.78, 0.0)],
            toe_log_h: 0.0,
            toe_width: 0.2,
            density_range: 3.3,
            base: SLIDE_BASE,
            interlayer: 0.36,
            adjacency: (0.10, 14.0),
            grain: Grain {
                rms: 8.0,
                clump_um: 5.0,
                layers: [0.9, 1.0, 1.2],
            },
            halation: BACKED,
            dyes: [[1.0, 0.12, 0.03], [0.18, 1.0, 0.12], [0.04, 0.22, 1.0]],
            look_points: [0.0, 0.0, 0.0],
            level: il,
        },
    ]
}

pub fn print_stocks() -> Vec<PrintStock> {
    let il = CalibrationLevel::Illustrative;
    vec![
        PrintStock {
            id: "scan",
            name: "Neutral scan",
            kind: PrintKind::Scan,
            about: "The negative scanned and inverted with ideal dyes and a gentle curve: the film's own colour.",
            gamma: [2.3, 2.3, 2.3],
            dmax: [2.6, 2.6, 2.6],
            toe: 0.15,
            dyes: IDEAL_DYES,
            level: il,
        },
        PrintStock {
            id: "endura",
            name: "Endura paper",
            kind: PrintKind::Paper,
            about: "Kodak's RA-4 portrait paper: soft, warm-neutral, paper blacks (about D 2.1).",
            gamma: [2.35, 2.35, 2.3],
            dmax: [2.1, 2.1, 2.05],
            toe: 0.18,
            dyes: [[1.0, 0.09, 0.02], [0.18, 1.0, 0.09], [0.05, 0.25, 1.0]],
            level: il,
        },
        PrintStock {
            id: "crystal-archive",
            name: "Crystal Archive paper",
            kind: PrintKind::Paper,
            about: "Fuji's RA-4 paper: a little more contrast, cooler and cleaner than Endura.",
            gamma: [2.45, 2.5, 2.5],
            dmax: [2.15, 2.2, 2.2],
            toe: 0.17,
            dyes: [[1.0, 0.08, 0.02], [0.16, 1.0, 0.11], [0.04, 0.24, 1.0]],
            level: il,
        },
        PrintStock {
            id: "2383",
            name: "2383 print film",
            kind: PrintKind::Film,
            about: "Kodak's cinema release print film: high contrast, dense blacks, warm-to-teal cinema colour.",
            gamma: [2.75, 2.7, 2.6],
            dmax: [3.6, 3.7, 3.6],
            toe: 0.15,
            dyes: [[1.0, 0.12, 0.03], [0.24, 1.0, 0.12], [0.06, 0.32, 1.0]],
            level: il,
        },
        PrintStock {
            id: "3513",
            name: "3513 print film",
            kind: PrintKind::Film,
            about: "Fuji's cinema print film: a touch softer and cooler than 2383, with cleaner greens.",
            gamma: [2.55, 2.6, 2.55],
            dmax: [3.5, 3.6, 3.5],
            toe: 0.16,
            dyes: [[1.0, 0.10, 0.03], [0.20, 1.0, 0.12], [0.05, 0.28, 1.0]],
            level: il,
        },
    ]
}

pub fn film_stock(id: &str) -> Option<FilmStock> {
    film_stocks().into_iter().find(|s| s.id == id)
}

pub fn print_stock(id: &str) -> Option<PrintStock> {
    print_stocks().into_iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitivity_rows_keep_neutrals_neutral() {
        for s in film_stocks() {
            for (i, row) in s.sensitivity.iter().enumerate() {
                let sum: f64 = row.iter().sum();
                assert!((sum - 1.0).abs() < 1e-9, "{} row {i}: {sum}", s.id);
            }
        }
    }

    #[test]
    fn ids_are_unique() {
        let films = film_stocks();
        for (i, a) in films.iter().enumerate() {
            assert!(films[i + 1..].iter().all(|b| b.id != a.id), "{}", a.id);
        }
        let prints = print_stocks();
        for (i, a) in prints.iter().enumerate() {
            assert!(prints[i + 1..].iter().all(|b| b.id != a.id), "{}", a.id);
        }
    }
}
