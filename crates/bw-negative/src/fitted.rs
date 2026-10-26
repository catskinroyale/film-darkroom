//! Calibrations fitted to manufacturer sensitometry (Level 2; design 0002).
//!
//! Each fitted calibration lives in `data/calibrations/` as written by
//! `tools/fit_development.py` from curves extracted with `tools/datasheet_curves.py`, and
//! is compiled in. Only the characteristic curve, its growth with development and the
//! base density are fitted; adjacency, compensation, grain growth and spectral response
//! still come from the illustrative tables and are labelled as such.

use bw_core::{CalibrationLevel, MonotoneCurve, Provenance};
use bw_developer::{Agitation, DevelopmentCalibration, FogModel, SpeedGain, SupportedRange};
use bw_film::FilmStock;

use crate::illustrative;

/// A fitted calibration file: scalar rows, then the `log_h,d_inf` knots.
#[derive(Debug)]
pub struct FittedCurve {
    pub tau_c0_min: f64,
    pub speed_shift: f64,
    pub tau_ref_min: f64,
    pub base_density: f64,
    pub fog_f1_per_min: f64,
    pub d_inf: MonotoneCurve,
}

impl FittedCurve {
    pub fn parse(text: &str) -> Result<FittedCurve, String> {
        let mut scalars = std::collections::HashMap::new();
        let mut knots = Vec::new();
        let mut in_knots = false;
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "log_h,d_inf" {
                in_knots = true;
                continue;
            }
            let (key, value) = line
                .split_once(',')
                .ok_or_else(|| format!("bad line {line:?}"))?;
            let value: f64 = value
                .trim()
                .parse()
                .map_err(|_| format!("bad number in {line:?}"))?;
            if in_knots {
                let x: f64 = key.parse().map_err(|_| format!("bad knot {line:?}"))?;
                knots.push((x, value));
            } else {
                scalars.insert(key.to_string(), value);
            }
        }
        let get = |k: &str| {
            scalars
                .get(k)
                .copied()
                .ok_or_else(|| format!("missing {k}"))
        };
        Ok(FittedCurve {
            tau_c0_min: get("tau_c0_min")?,
            speed_shift: get("speed_shift")?,
            tau_ref_min: get("tau_ref_min")?,
            base_density: get("base_density")?,
            fog_f1_per_min: get("fog_f1_per_min")?,
            d_inf: MonotoneCurve::new(&knots).map_err(|e| e.to_string())?,
        })
    }
}

/// A film whose D-76 curve family has been fitted.
struct FilmFit {
    film_id: &'static str,
    curve: &'static str,
    /// Agitation interval (s) of the published family.
    reference_interval_s: f64,
    /// How much faster 30-second agitation develops than the family's own agitation, from
    /// the manufacturer's small- and large-tank tables (1 when the family is small tank).
    small_tank_rate: f64,
    /// D-76 temperature coefficient from the manufacturer's table at the family's agitation.
    alpha_per_c: f64,
    /// The published family's development times (min, at its own agitation).
    family_min: (f64, f64),
    holdout_rms: f64,
    sources: &'static [&'static str],
}

const FILMS: [FilmFit; 3] = [
    FilmFit {
        film_id: "tri-x",
        curve: include_str!("../../../data/calibrations/tri-x_d76-stock.csv"),
        reference_interval_s: 60.0,
        // Kodak's tables (F-4017, 400TX, D-76 at 18–24 °C) give large-tank times about
        // 1.155× the small-tank times: mean of 9¼/8, 7¾/6¾, 7/6¼, 6½/5½, 5½/4¾.
        small_tank_rate: 1.155,
        // Large-tank D-76: 9¼ min at 18 °C to 5½ min at 24 °C.
        alpha_per_c: 0.0867,
        family_min: (6.0, 12.0),
        holdout_rms: 0.014,
        sources: &[
            "Kodak Alaris F-4017 (Feb 2016) p. 8: Characteristic Curves, TRI-X 400 / 400TX, 35 mm, \
             D-76, large tank, agitation every minute, 20 °C, 6/8/10/12 min, diffuse visual \
             density; extracted from the PDF's vector paths",
            "Kodak Alaris F-4017 pp. 3–4: time tables, 18–24 °C (temperature and small/large-tank \
             rates)",
        ],
    },
    FilmFit {
        film_id: "tmax-400",
        curve: include_str!("../../../data/calibrations/tmax-400_d76-stock.csv"),
        reference_interval_s: 30.0,
        small_tank_rate: 1.0,
        // Small-tank D-76: 8¼ min at 18 °C to 5½ min at 24 °C.
        alpha_per_c: 0.0676,
        family_min: (6.0, 11.0),
        holdout_rms: 0.030,
        sources: &[
            "Kodak Alaris F-4043 (Feb 2016) p. 8: Characteristic Curves, T-MAX 400, D-76, small \
             tank, 20 °C, 6/8/11 min, diffuse visual density; extracted from the PDF's vector paths",
            "Kodak Alaris F-4043 p. 3: time tables, 18–24 °C",
        ],
    },
    FilmFit {
        film_id: "tmax-100",
        curve: include_str!("../../../data/calibrations/tmax-100_d76-stock.csv"),
        reference_interval_s: 30.0,
        small_tank_rate: 1.0,
        // Small-tank D-76: 7½ min at 18 °C to 4¼ min at 24 °C.
        alpha_per_c: 0.0947,
        family_min: (6.0, 10.0),
        holdout_rms: 0.038,
        sources: &[
            "Kodak F-4016 (March 2002) p. 15: Characteristic Curves, T-MAX 100, D-76, small tank, \
             20 °C, 6/7½/10 min, diffuse visual density; extracted from the PDF's vector paths. \
             The current sheet (Kodak Alaris, Feb 2016) draws the same curves (within 0.003 D) \
             but its legend's times are lost to a font fault",
            "Kodak Alaris F-4016 (Feb 2016) p. 3: time tables, 18–24 °C",
        ],
    },
];

const DEVELOPER_RATES: &str = include_str!("../../../data/calibrations/developer-rates.csv");

/// A developer fitted only through its contrast-index line: its speed against the film's
/// D-76 calibration and its own temperature coefficient.
struct DeveloperRate {
    rate: f64,
    alpha_per_c: f64,
    ci_rms: f64,
    source: String,
}

fn developer_rate(film_id: &str, developer_id: &str) -> Option<DeveloperRate> {
    DEVELOPER_RATES
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("film,"))
        .map(|l| l.split(',').collect::<Vec<_>>())
        .find(|f| f.len() >= 8 && f[0] == film_id && f[1] == developer_id)
        .and_then(|f| {
            Some(DeveloperRate {
                rate: f[2].parse().ok()?,
                alpha_per_c: f[3].parse().ok()?,
                ci_rms: f[4].parse().ok()?,
                source: format!("Kodak {}", f[7..].join(",")),
            })
        })
}

/// The fitted calibration for a film and developer combination, if there is one: the film's
/// D-76 family itself, or D-76's curve run at another developer's fitted rate.
pub fn combination(
    film_id: &str,
    developer_id: &str,
) -> Option<(FilmStock, DevelopmentCalibration)> {
    let developer_id = illustrative::canonical_developer(developer_id);
    let Some(fit) = FILMS.iter().find(|f| f.film_id == film_id) else {
        return approximation(film_id, developer_id);
    };
    let rate = match developer_id {
        "d76-stock" => None,
        other => Some(developer_rate(film_id, other)?),
    };
    let fitted = FittedCurve::parse(fit.curve).expect("bundled calibration parses");
    let (mut film, mut cal) = illustrative::combination(film_id, developer_id)?;

    film.base_density = fitted.base_density;
    film.provenance.assumptions.push(format!(
        "Base density {:.3} is the manufacturer's base plus initial fog in D-76 (the curves \
         cannot separate them)",
        fitted.base_density
    ));
    // Another developer runs the same curve at its own speed k: time scales by k, which is
    // the same as dividing every time constant by k.
    let k = rate.as_ref().map_or(1.0, |r| r.rate);
    cal.d_inf = fitted.d_inf;
    cal.tau_c0_min = fitted.tau_c0_min / k;
    cal.speed = SpeedGain {
        shift_log_h: fitted.speed_shift,
        tau_ref_min: fitted.tau_ref_min / k,
    };
    cal.fog = FogModel {
        f0: 0.0,
        f1_per_min: fitted.fog_f1_per_min * k,
    };
    cal.reference_agitation = Agitation::Intermittent {
        interval_s: fit.reference_interval_s,
    };
    let a = |interval_s: f64| Agitation::Intermittent { interval_s }.index();
    if fit.small_tank_rate != 1.0 {
        // r_A = 1 + slope·(a − a_ref): 30-second agitation at the published rate.
        cal.agitation_rate_slope =
            (fit.small_tank_rate - 1.0) / (a(30.0) - a(fit.reference_interval_s));
    }
    cal.alpha_per_c = rate.as_ref().map_or(fit.alpha_per_c, |r| r.alpha_per_c);
    let (t0, t1) = fit.family_min;
    cal.supported = SupportedRange {
        // The published family, in small-tank minutes for this developer.
        time_min: (
            t0 / (fit.small_tank_rate * k),
            t1 / (fit.small_tank_rate * k),
        ),
        temperature_c: (18.0, 24.0),
    };
    let mut sources: Vec<String> = fit.sources.iter().map(|s| s.to_string()).collect();
    let mut assumptions = vec![format!(
        "Characteristic curve, its growth with development and the speed gain fitted jointly \
         to the published D-76 curves (hold-out RMS {:.3} D; design 0002)",
        fit.holdout_rms
    )];
    if fit.small_tank_rate != 1.0 {
        assumptions.push(format!(
            "Small-tank (30 s) agitation develops {}× as fast as the published curves, from \
             the manufacturer's tables",
            fit.small_tank_rate
        ));
    }
    if let Some(r) = &rate {
        sources.push(r.source.clone());
        assumptions.push(format!(
            "No curves are published in this developer: D-76's curve, run {:.2}× as fast, \
             fitted to the manufacturer's contrast-index line (CI RMS {:.3})",
            r.rate, r.ci_rms
        ));
    }
    cal.provenance = Provenance {
        level: CalibrationLevel::Sensitometric,
        version: "1.1.0".into(),
        sources,
        assumptions,
        limitations: vec![
            "Adjacency, compensation in stand development and grain growth are still \
             illustrative"
                .into(),
            "Outside the published times the curve family is extrapolated".into(),
        ],
    };
    Some((film, cal))
}

/// An Ilford film calibrated from the curve(s) on its datasheet (Level 1; design 0002 §6).
struct IlfordFit {
    film_id: &'static str,
    curve: &'static str,
    source: &'static str,
    /// Where the growth with development came from.
    growth: &'static str,
}

const ILFORD: [IlfordFit; 5] = [
    IlfordFit {
        film_id: "hp5",
        curve: include_str!("../../../data/calibrations/ilford/hp5.csv"),
        source: "HARMAN/Ilford HP5 PLUS Technical Information (Nov 2018) p. 5: characteristic \
                 curve, ILFOTEC HC 1+31, 6½ min, 20 °C; traced from the sheet's raster image",
        growth: "borrowed from Tri-X in D-76 (Kodak F-4017 fit)",
    },
    IlfordFit {
        film_id: "fp4",
        curve: include_str!("../../../data/calibrations/ilford/fp4.csv"),
        source: "HARMAN/Ilford FP4 PLUS Technical Information (Nov 2018) p. 5: characteristic \
                 curve, ILFOTEC HC 1+31, 8 min, 20 °C; traced from the sheet's raster image",
        growth: "borrowed from Tri-X in D-76 (Kodak F-4017 fit)",
    },
    IlfordFit {
        film_id: "delta-100",
        curve: include_str!("../../../data/calibrations/ilford/delta-100.csv"),
        source: "Ilford 100 DELTA Professional fact sheet (2010) p. 4: characteristic curve, ID-11 \
                 stock, 8½ min, 20 °C; traced from the sheet's rendered image",
        growth: "borrowed from T-Max 100 in D-76 (Kodak F-4016 fit)",
    },
    IlfordFit {
        film_id: "delta-400",
        curve: include_str!("../../../data/calibrations/ilford/delta-400.csv"),
        source: "HARMAN/Ilford DELTA 400 Professional Technical Information (Nov 2018) p. 6: \
                 characteristic curve, ID-11 stock, 8 min, 24 °C; traced from the sheet's raster image",
        growth: "borrowed from T-Max 400 in D-76 (Kodak F-4043 fit)",
    },
    IlfordFit {
        film_id: "delta-3200",
        curve: include_str!("../../../data/calibrations/ilford/delta-3200.csv"),
        source: "Ilford DELTA 3200 Professional fact sheet (April 1999) p. 6: characteristic curves, \
                 ILFOTEC DD-X 1+4, 7/9/12/16 min, 20 °C; extracted from the PDF's vector paths",
        growth: "fitted to Ilford's own DD-X family (hold-out RMS 0.030 D)",
    },
];

/// Level-1 calibration for an Ilford film in any developer with a published time: Ilford's
/// curve shape, placed on the absolute exposure axis by its ISO speed, developed in each
/// developer so that its normal time gives the published curve.
fn approximation(film_id: &str, developer_id: &str) -> Option<(FilmStock, DevelopmentCalibration)> {
    let fit = ILFORD.iter().find(|f| f.film_id == film_id)?;
    let fitted = FittedCurve::parse(fit.curve).expect("bundled calibration parses");
    let (mut film, mut cal) = illustrative::combination(film_id, developer_id)?;
    let t_normal = illustrative::normal_time(film_id, developer_id)?;
    film.base_density = fitted.base_density;
    // At this developer's normal time (30 s agitation, 20 °C) τ reaches the published curve's
    // reference: every time constant scales by t_normal / τ_ref.
    let k = fitted.tau_ref_min / t_normal;
    cal.d_inf = fitted.d_inf;
    cal.tau_c0_min = fitted.tau_c0_min / k;
    cal.speed = SpeedGain {
        shift_log_h: fitted.speed_shift,
        tau_ref_min: t_normal,
    };
    cal.fog = FogModel {
        f0: 0.0,
        f1_per_min: fitted.fog_f1_per_min * k,
    };
    cal.reference_agitation = Agitation::Intermittent { interval_s: 30.0 };
    cal.provenance = Provenance {
        level: CalibrationLevel::Approximation,
        version: "1.0.0".into(),
        sources: vec![fit.source.into()],
        assumptions: vec![
            "Curve shape from Ilford's published curve; its exposure axis is relative, so the \
             curve is placed where its ISO speed, at ISO 6 contrast, equals the film's ISO"
                .into(),
            format!("Growth with development {}", fit.growth),
            "Normal development in any developer gives Ilford's published curve; each \
             developer's published time sets its timing"
                .into(),
        ],
        limitations: vec![
            "Level 1: one published curve (Delta 3200: one family) in an Ilford developer; \
             no Ilford sensitometry in D-76, HC-110 or Rodinal"
                .into(),
            "Adjacency, compensation and grain growth are illustrative".into(),
        ],
    };
    Some((film, cal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitometry::iso_speed;
    use bw_developer::ProcessingConditions;

    fn agitated(interval_s: f64, time_min: f64) -> ProcessingConditions {
        ProcessingConditions {
            time_min,
            temperature_c: 20.0,
            agitation: Agitation::Intermittent { interval_s },
        }
    }

    fn published(path: &str) -> Vec<(f64, f64)> {
        let path = format!(
            "{}/../../data/sensitometry/{path}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{path}"))
            .lines()
            .skip(1)
            .map(|l| {
                let (h, d) = l.split_once(',').unwrap();
                (h.parse().unwrap(), d.parse().unwrap())
            })
            .collect()
    }

    /// (film, published curve, minutes, agitation interval of the published family)
    const FAMILIES: [(&str, &str, f64, f64); 10] = [
        (
            "tri-x",
            "kodak-f4017/tri-x-400_35mm_d76-stock_large-tank_06min.csv",
            6.0,
            60.0,
        ),
        (
            "tri-x",
            "kodak-f4017/tri-x-400_35mm_d76-stock_large-tank_08min.csv",
            8.0,
            60.0,
        ),
        (
            "tri-x",
            "kodak-f4017/tri-x-400_35mm_d76-stock_large-tank_10min.csv",
            10.0,
            60.0,
        ),
        (
            "tri-x",
            "kodak-f4017/tri-x-400_35mm_d76-stock_large-tank_12min.csv",
            12.0,
            60.0,
        ),
        (
            "tmax-400",
            "kodak-f4043/tmax-400_d76-stock_small-tank_06min.csv",
            6.0,
            30.0,
        ),
        (
            "tmax-400",
            "kodak-f4043/tmax-400_d76-stock_small-tank_08min.csv",
            8.0,
            30.0,
        ),
        (
            "tmax-400",
            "kodak-f4043/tmax-400_d76-stock_small-tank_11min.csv",
            11.0,
            30.0,
        ),
        (
            "tmax-100",
            "kodak-f4016/tmax-100_d76-stock_small-tank_06min.csv",
            6.0,
            30.0,
        ),
        (
            "tmax-100",
            "kodak-f4016/tmax-100_d76-stock_small-tank_07.5min.csv",
            7.5,
            30.0,
        ),
        (
            "tmax-100",
            "kodak-f4016/tmax-100_d76-stock_small-tank_10min.csv",
            10.0,
            30.0,
        ),
    ];

    /// Design 0001 §8.2 "curve fidelity": the engine reproduces each published curve
    /// within 0.03 D RMS.
    #[test]
    fn engine_reproduces_kodaks_published_d76_curves() {
        for (film_id, path, minutes, interval) in FAMILIES {
            let (film, cal) = combination(film_id, "d76-stock").unwrap();
            let state = cal.resolve(&agitated(interval, minutes));
            let points = published(path);
            let sq: f64 = points
                .iter()
                .map(|&(h, d)| (film.base_density + cal.net_density(h, &state) - d).powi(2))
                .sum();
            let rms = (sq / points.len() as f64).sqrt();
            assert!(rms < 0.03, "{film_id} {minutes} min: RMS {rms:.4} D");
        }
    }

    /// Design 0001 §8.2 "ISO speed": at the development giving ISO 6's contrast (ΔD 0.80
    /// over 1.30 log H), the speed is within ⅓ stop of the box speed. T-Max 100 in D-76
    /// reads +0.4 stop on Kodak's own curves (Kodak's ISO figure is measured in the ISO
    /// standard's developer, not D-76), so it is held to ½ stop.
    #[test]
    fn iso_speed_at_iso_contrast_matches_the_box_speed() {
        for (film_id, tolerance) in [
            ("tri-x", 1.0 / 3.0),
            ("tmax-400", 1.0 / 3.0),
            ("tmax-100", 0.5),
        ] {
            let (film, cal) = combination(film_id, "d76-stock").unwrap();
            let at = |t: f64| iso_speed(&cal, &cal.resolve(&agitated(30.0, t))).unwrap();
            let (mut lo, mut hi) = (3.0, 14.0);
            for _ in 0..60 {
                let mid = 0.5 * (lo + hi);
                if at(mid).delta_d_1_30 < 0.80 {
                    lo = mid
                } else {
                    hi = mid
                }
            }
            let stops = (at(lo).speed / film.nominal_iso).log2();
            assert!(stops.abs() < tolerance, "{film_id}: {stops:+.2} stops");
        }
    }

    /// Kodak's two-stop push (EI 1600: 9½ min instead of 6¾, small tank) buys about one
    /// stop of real speed, half the nominal push: the shadows are still underexposed.
    #[test]
    fn a_two_stop_push_gains_about_one_stop_of_real_speed() {
        let (_, cal) = combination("tri-x", "d76-stock").unwrap();
        let speed = |t: f64| {
            iso_speed(&cal, &cal.resolve(&agitated(30.0, t)))
                .unwrap()
                .speed
        };
        let gain = (speed(9.5) / speed(6.75)).log2();
        assert!((0.6..1.4).contains(&gain), "{gain:+.2} stops");
    }

    /// Kodak's tables give 6¾ min small tank and 7¾ min large tank for the same result.
    #[test]
    fn small_tank_time_matches_the_large_tank_curve() {
        let (_, cal) = combination("tri-x", "d76-stock").unwrap();
        let small = cal.resolve(&agitated(30.0, 6.75));
        assert!(
            (small.tau_min - 7.75).abs() < 0.1,
            "tau {:.2}",
            small.tau_min
        );
    }

    /// A developer fitted from its contrast-index line gives, at Kodak's own normal time
    /// for it, about the contrast D-76 gives at D-76's normal time (all small tank):
    /// the rates agree with Kodak's time tables, which were not used to fit them.
    #[test]
    fn developer_rates_agree_with_kodaks_normal_times() {
        let gradient = |film: &str, dev: &str, t: f64| {
            let (_, cal) = combination(film, dev).unwrap();
            let s = cal.resolve(&agitated(30.0, t));
            cal.net_density(-1.0, &s) - cal.net_density(-2.0, &s)
        };
        for (film, d76_normal, dev, dev_normal) in [
            ("tri-x", 6.75, "hc110-b", 3.75),
            ("tri-x", 6.75, "d76-1+1", 9.75),
            ("tmax-400", 7.5, "hc110-b", 5.5),
            ("tmax-400", 7.5, "d76-1+1", 10.25),
        ] {
            let reference = gradient(film, "d76-stock", d76_normal);
            let other = gradient(film, dev, dev_normal);
            let ratio = other / reference;
            assert!((0.9..1.1).contains(&ratio), "{film} {dev}: {ratio:.3}");
            let (_, cal) = combination(film, dev).unwrap();
            assert_eq!(cal.provenance.level, CalibrationLevel::Sensitometric);
        }
    }

    #[test]
    fn unfitted_combinations_stay_illustrative() {
        assert!(combination("tri-x", "rodinal-1+50").is_none());
        assert!(combination("kentmere-400", "d76-stock").is_none());
        assert!(combination("double-x", "d76-stock").is_none());
    }

    /// Ilford's curves (Level 1): at each developer's normal time the film shows its ISO
    /// speed within half a stop, and the published curve is reproduced in shape.
    #[test]
    fn ilford_films_reach_their_iso_speed_in_every_developer() {
        for (film_id, iso) in [
            ("hp5", 400.0),
            ("fp4", 125.0),
            ("delta-100", 100.0),
            ("delta-400", 400.0),
            ("delta-3200", 1000.0),
        ] {
            for dev in illustrative::DEVELOPERS {
                let Some((_, cal)) = combination(film_id, dev) else {
                    continue;
                };
                assert_eq!(cal.provenance.level, CalibrationLevel::Approximation);
                let t = illustrative::normal_time(film_id, dev).unwrap();
                let speed = iso_speed(&cal, &cal.resolve(&agitated(30.0, t)))
                    .unwrap()
                    .speed;
                // Delta 3200's published times are for EI 3200, a push from ISO 1000 that
                // gains only part of a stop of real speed.
                let stops = (speed / iso).log2();
                println!("{film_id}/{dev}: {speed:.0} ({stops:+.2} stops from ISO {iso})");
                assert!(
                    stops.abs() < 0.5,
                    "{film_id}/{dev}: {stops:+.2} stops from ISO {iso}"
                );
            }
        }
    }

    #[test]
    fn ilford_curves_reproduce_the_published_shape() {
        for (film_id, path, base) in [
            ("hp5", "ilford/hp5-plus_ilfotec-hc-1+31_6.5min.csv", 0.0),
            ("delta-100", "ilford/delta-100_id-11-stock_8.5min.csv", 0.0),
        ] {
            let fitted =
                FittedCurve::parse(ILFORD.iter().find(|f| f.film_id == film_id).unwrap().curve)
                    .unwrap();
            // Net density rise over the plotted range: published vs calibration at its reference.
            let points = published(path);
            let rise = points.last().unwrap().1 - points[0].1;
            let span = points.last().unwrap().0 - points[0].0;
            let g = 1.0 - (-fitted.tau_ref_min / fitted.tau_c0_min).exp();
            let (lo, hi) = fitted.d_inf.domain();
            let model_rise = (fitted.d_inf.eval(lo + span) - fitted.d_inf.eval(lo)) * g;
            assert!(
                (model_rise - rise).abs() < 0.05 + base,
                "{film_id}: {model_rise:.2} vs {rise:.2} ({hi:.1})"
            );
        }
    }
}
