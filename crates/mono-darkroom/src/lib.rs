//! Printing (design 0002): the negative projected onto a measured paper.
//!
//! The paper is a family of characteristic curves, one per variable-contrast filter
//! grade, taken from the manufacturer's datasheet (`data/papers/`). Between published
//! grades the curves are blended horizontally: at each fraction of the paper's density
//! range, the log exposure is interpolated between the neighbouring grades, as mixed
//! filtration on variable-contrast paper behaves to a first approximation.

use std::sync::OnceLock;

use fd_core::{CalibrationLevel, Lut1D, MonotoneCurve, Plane, Provenance};
use mono_negative::Negative;
use rayon::prelude::*;

const POLYMAX_II_RC: &str = include_str!("../../../data/papers/kodak-polymax-ii-rc.csv");
const POLYCONTRAST_IV_RC_N: &str =
    include_str!("../../../data/papers/kodak-polycontrast-iv-rc-n.csv");

/// A printing paper available to the engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Paper {
    /// Kodak Polymax II RC, F surface (glossy), in Dektol 1:2 (Kodak G-26).
    #[default]
    PolymaxIIRc,
    /// Kodak Polycontrast IV RC, N surface (matte), in Dektol 1:2 (Kodak G-4037).
    PolycontrastIVRcN,
}

impl Paper {
    pub const ALL: [Paper; 2] = [Paper::PolymaxIIRc, Paper::PolycontrastIVRcN];

    /// Stable id, as saved in styles.
    pub fn id(self) -> &'static str {
        match self {
            Paper::PolymaxIIRc => "polymax-ii-rc",
            Paper::PolycontrastIVRcN => "polycontrast-iv-rc-n",
        }
    }

    /// The paper with this id; the default paper for unknown or empty ids.
    pub fn from_id(id: &str) -> Paper {
        Paper::ALL
            .into_iter()
            .find(|p| p.id() == id)
            .unwrap_or_default()
    }
}

/// One filter grade's characteristic curve: reflection density against log exposure.
struct GradeCurve {
    filter: f64,
    log_h: Vec<f64>,
    density: Vec<f64>,
}

impl GradeCurve {
    fn d_min(&self) -> f64 {
        self.density[0]
    }

    fn d_max(&self) -> f64 {
        self.density[self.density.len() - 1]
    }

    /// Log exposure where the curve first reaches `fraction` of its density range.
    fn log_h_at_fraction(&self, fraction: f64) -> f64 {
        let target = self.d_min() + fraction * (self.d_max() - self.d_min());
        let i = self
            .density
            .partition_point(|&d| d < target)
            .clamp(1, self.density.len() - 1);
        let (d0, d1) = (self.density[i - 1], self.density[i]);
        let t = if d1 > d0 {
            (target - d0) / (d1 - d0)
        } else {
            0.0
        };
        self.log_h[i - 1] + t * (self.log_h[i] - self.log_h[i - 1])
    }
}

/// A paper's grade family, as published.
pub struct PaperFamily {
    pub name: &'static str,
    grades: Vec<GradeCurve>,
    pub provenance: Provenance,
}

impl PaperFamily {
    fn parse(name: &'static str, text: &str, provenance: Provenance) -> Result<Self, String> {
        let mut grades: Vec<GradeCurve> = Vec::new();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') || line.starts_with("filter,") {
                continue;
            }
            let v: Vec<f64> = line
                .split(',')
                .map(|x| {
                    x.trim()
                        .parse::<f64>()
                        .map_err(|_| format!("bad line {line:?}"))
                })
                .collect::<Result<_, _>>()?;
            let [filter, log_h, density] = v[..] else {
                return Err(format!("bad line {line:?}"));
            };
            match grades.last_mut() {
                Some(g) if g.filter == filter => {
                    g.log_h.push(log_h);
                    g.density.push(density);
                }
                _ => grades.push(GradeCurve {
                    filter,
                    log_h: vec![log_h],
                    density: vec![density],
                }),
            }
        }
        grades.sort_by(|a, b| a.filter.total_cmp(&b.filter));
        if grades.len() < 2 {
            return Err("a paper needs at least two grades".into());
        }
        Ok(Self {
            name,
            grades,
            provenance,
        })
    }

    pub fn get(paper: Paper) -> &'static PaperFamily {
        static POLYMAX: OnceLock<PaperFamily> = OnceLock::new();
        static POLYCONTRAST_N: OnceLock<PaperFamily> = OnceLock::new();
        match paper {
            Paper::PolycontrastIVRcN => POLYCONTRAST_N.get_or_init(|| {
                PaperFamily::parse(
                    "Polycontrast IV-like, matte",
                    POLYCONTRAST_IV_RC_N,
                    Provenance {
                        level: CalibrationLevel::Sensitometric,
                        version: "1.0.0".into(),
                        sources: vec![
                            "Kodak G-4037 (June 2005) p. 8: Characteristic Curves, KODAK \
                             PROFESSIONAL POLYCONTRAST IV RC Paper, N surface, POLYMAX Filters \
                             1, 3, 5+, DEKTOL 1:2, 60 s, 20 °C; extracted from the PDF's vector \
                             paths"
                                .into(),
                        ],
                        assumptions: vec![
                            "Grades between the published filters 1, 3 and 5+ blend the \
                             neighbouring curves; below filter 1 the filter-1 curve is used"
                                .into(),
                        ],
                        limitations: vec![
                            "Kodak discontinued its black-and-white papers".into(),
                            "Only three filters are published, so in-between grades are \
                             coarser than on Polymax II"
                                .into(),
                        ],
                    },
                )
                .expect("bundled paper parses")
            }),
            Paper::PolymaxIIRc => POLYMAX.get_or_init(|| {
                PaperFamily::parse(
                    "Polymax II-like, glossy",
                    POLYMAX_II_RC,
                    Provenance {
                        level: CalibrationLevel::Sensitometric,
                        version: "1.0.0".into(),
                        sources: vec![
                            "Kodak G-26 (June 2005) p. 7: Characteristic Curves, KODAK \
                             PROFESSIONAL POLYMAX II RC Paper, F surface, POLYMAX Filters -1 to \
                             5+, DEKTOL 1:2, 60 s, 20 °C; extracted from the PDF's vector paths"
                                .into(),
                        ],
                        assumptions: vec![
                            "Half grades and in-between settings blend the neighbouring \
                             published curves horizontally at equal density fraction"
                                .into(),
                        ],
                        limitations: vec![
                            "Kodak discontinued its black-and-white papers; this is a \
                             representative modern RC variable-contrast paper"
                                .into(),
                            "Filter 5+ measures R56 on the plotted curve against R40 in \
                             Kodak's own table"
                                .into(),
                        ],
                    },
                )
                .expect("bundled paper parses")
            }),
        }
    }

    /// Lowest and highest published filter grade.
    pub fn grade_range(&self) -> (f64, f64) {
        (
            self.grades[0].filter,
            self.grades[self.grades.len() - 1].filter,
        )
    }

    /// The characteristic curve at `grade`, blended between the neighbouring published
    /// grades.
    pub fn curve(&self, grade: f64) -> PaperCurve {
        let (lo, hi) = self.grade_range();
        let g = grade.clamp(lo, hi);
        let i = self
            .grades
            .partition_point(|c| c.filter <= g)
            .clamp(1, self.grades.len() - 1);
        let (a, b) = (&self.grades[i - 1], &self.grades[i]);
        let w = ((g - a.filter) / (b.filter - a.filter)).clamp(0.0, 1.0);
        let d_min = a.d_min() + w * (b.d_min() - a.d_min());
        let d_max = a.d_max() + w * (b.d_max() - a.d_max());
        // Strictly inside the density range, where both curves are rising.
        const N: usize = 400;
        let points: Vec<(f64, f64)> = (0..=N)
            .map(|k| {
                let f = 0.002 + 0.996 * k as f64 / N as f64;
                let h = (1.0 - w) * a.log_h_at_fraction(f) + w * b.log_h_at_fraction(f);
                (h, d_min + f * (d_max - d_min))
            })
            .collect();
        let mut monotone: Vec<(f64, f64)> = Vec::with_capacity(points.len());
        for p in points {
            if monotone.last().is_none_or(|q| p.0 > q.0 + 1e-9) {
                monotone.push(p);
            }
        }
        let inverse: Vec<(f64, f64)> = monotone.iter().map(|&(h, d)| (d, h)).collect();
        PaperCurve {
            d_min,
            d_max,
            forward: MonotoneCurve::new(&monotone).expect("blended curve is monotone"),
            inverse: MonotoneCurve::new(&inverse).expect("blended curve is monotone"),
        }
    }
}

/// One paper at one grade.
pub struct PaperCurve {
    pub d_min: f64,
    pub d_max: f64,
    forward: MonotoneCurve,
    inverse: MonotoneCurve,
}

impl PaperCurve {
    /// Reflection density at paper log exposure `log_h`.
    pub fn density_at(&self, log_h: f64) -> f64 {
        self.forward.eval(log_h).clamp(self.d_min, self.d_max)
    }

    /// Paper log exposure that prints at `density`.
    pub fn log_exposure_for(&self, density: f64) -> f64 {
        let span = self.d_max - self.d_min;
        self.inverse
            .eval(density.clamp(self.d_min + 0.002 * span, self.d_max - 0.002 * span))
    }

    /// Exposure (lux·s on the curve's axis) at which density first rises 0.04 above paper
    /// white: the reference for preflash.
    pub fn threshold_exposure(&self) -> f64 {
        10f64.powf(self.log_exposure_for(self.d_min + 0.04))
    }

    /// ISO 6846 paper range: 100 × the log exposure from density Dmin + 0.04 to 0.9 × Dmax.
    pub fn iso_range(&self) -> f64 {
        100.0 * (self.log_exposure_for(0.9 * self.d_max) - self.log_exposure_for(self.d_min + 0.04))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrintExposure {
    /// Expose so the negative's metered mid-grey prints at this reflection density.
    MidGreyAt { print_density: f64 },
    /// Expose so clear film (base + fog) prints at this reflection density: the
    /// printer's black point ("minimum time for maximum black") held from negative to
    /// negative. Development changes then print as they do under a real enlarger:
    /// longer development adds density above the base, so blacks hold while shadows
    /// open slightly and highlights brighten.
    BaseAt { print_density: f64 },
    /// Explicit log exposure offset.
    LogOffset(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrintSettings {
    pub paper: Paper,
    /// Variable-contrast filter grade (Kodak Polymax numbering, −1 to 5).
    pub grade: f64,
    pub exposure: PrintExposure,
    /// Callier factor (1.0 for a diffusion enlarger).
    pub callier_q: f64,
    /// Additional enlarger exposure in stops (positive = longer exposure, darker print).
    pub exposure_adjust_stops: f64,
    /// Preflash: a uniform exposure added to the image, as a fraction of the paper's
    /// threshold exposure (where density first rises 0.04 above paper white). Up to 1 it
    /// brings bright highlights into printable grey while paper white stays white.
    pub preflash: f64,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            paper: Paper::default(),
            grade: 2.0,
            exposure: PrintExposure::MidGreyAt {
                print_density: 0.75,
            },
            callier_q: 1.0,
            exposure_adjust_stops: 0.0,
            preflash: 0.0,
        }
    }
}

impl PrintSettings {
    /// The paper's curve at this grade.
    pub fn curve(&self) -> PaperCurve {
        PaperFamily::get(self.paper).curve(self.grade)
    }

    /// Paper log exposure that prints at `density`.
    pub fn log_exposure_for(&self, density: f64) -> f64 {
        self.curve().log_exposure_for(density)
    }

    /// Reflection density at paper log exposure `log_h`.
    pub fn density_at(&self, log_h: f64) -> f64 {
        self.curve().density_at(log_h)
    }
}

/// Print a negative. Returns reflectance relative to paper white (1.0 = paper base).
pub fn print(negative: &Negative, settings: &PrintSettings) -> Plane {
    let curve = settings.curve();
    let d_min = curve.d_min;
    let q = settings.callier_q;
    let offset = std::f64::consts::LOG10_2 * settings.exposure_adjust_stops
        + match settings.exposure {
            PrintExposure::LogOffset(o) => o,
            PrintExposure::MidGreyAt { print_density } => {
                curve.log_exposure_for(print_density) + q * negative.grey_density
            }
            PrintExposure::BaseAt { print_density } => {
                curve.log_exposure_for(print_density) + q * negative.base_plus_fog
            }
        };
    // Preflash adds a uniform exposure in linear terms: the paper sees H_image + H_flash.
    let flash = settings.preflash.max(0.0) * curve.threshold_exposure();
    // Per pixel through a lookup table spanning everything the negative can reach.
    let (lo, hi) = (offset - q * 6.0, offset + q * 0.5);
    let lut = Lut1D::from_fn(lo, hi, 8192, |h| {
        curve.density_at((10f64.powf(h) + flash).log10())
    })
    .expect("valid range");
    let mut out = negative.density.zeros_like();
    out.data_mut()
        .par_iter_mut()
        .zip(negative.density.data().par_iter())
        .for_each(|(r, &d)| {
            let dp = f64::from(lut.eval((offset - q * f64::from(d)) as f32));
            *r = 10f64.powf(-(dp - d_min)) as f32;
        });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(grade: f64) -> PaperCurve {
        PaperFamily::get(Paper::PolymaxIIRc).curve(grade)
    }

    #[test]
    fn higher_grade_has_shorter_range() {
        let ranges: Vec<f64> = [-1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0]
            .into_iter()
            .map(|g| at(g).iso_range())
            .collect();
        assert!(ranges.windows(2).all(|w| w[1] < w[0]), "{ranges:?}");
    }

    /// The published grades reproduce Kodak's own ISO range table (G-26, F surface,
    /// Dektol 1:2) within 8 points, except 5+, whose plotted curve and table disagree.
    #[test]
    fn published_grades_match_kodaks_iso_ranges() {
        for (grade, table) in [
            (-1.0, 140.0),
            (0.0, 140.0),
            (1.0, 130.0),
            (1.5, 120.0),
            (2.0, 120.0),
            (3.0, 100.0),
            (3.5, 80.0),
            (4.0, 70.0),
        ] {
            let r = at(grade).iso_range();
            assert!(
                (r - table).abs() <= 8.0,
                "grade {grade}: R{r:.0} vs R{table}"
            );
        }
    }

    #[test]
    fn full_preflash_brings_highlights_to_threshold_but_keeps_paper_white() {
        let curve = at(2.0);
        let flash = curve.threshold_exposure();
        let with = |h: f64| curve.density_at((10f64.powf(h) + flash).log10());
        // No image exposure: the flash alone reaches the threshold density.
        assert!((with(-12.0) - (curve.d_min + 0.04)).abs() < 0.01);
        // A highlight just under the threshold now prints visibly grey.
        let h = curve.log_exposure_for(curve.d_min + 0.02);
        assert!(with(h) - curve.density_at(h) > 0.03);
        // The effect falls off with density: highlights gain most, mid-tones a little,
        // the deepest shadows almost nothing.
        let gain = |d: f64| with(curve.log_exposure_for(d)) - d;
        assert!(gain(0.15) > gain(1.0) && gain(1.0) > gain(1.9));
        assert!(gain(1.9) < 0.05);
    }

    #[test]
    fn papers_round_trip_their_ids() {
        for p in Paper::ALL {
            assert_eq!(Paper::from_id(p.id()), p);
            let _ = PaperFamily::get(p).curve(2.0);
        }
        assert_eq!(Paper::from_id(""), Paper::default());
    }

    #[test]
    fn blended_grades_fall_between_their_neighbours() {
        let (g2, g25, g3) = (
            at(2.0).iso_range(),
            at(2.5).iso_range(),
            at(3.0).iso_range(),
        );
        assert!(g3 < g25 && g25 < g2, "{g2:.0} {g25:.0} {g3:.0}");
    }

    #[test]
    fn log_exposure_for_inverts_density_at() {
        let s = PrintSettings::default();
        for d in [0.1, 0.75, 1.5, 2.0] {
            let back = s.density_at(s.log_exposure_for(d));
            assert!((back - d).abs() < 2e-3, "{d} -> {back}");
        }
    }
}
