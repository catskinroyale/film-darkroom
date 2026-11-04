//! Styles: a film stock, its processing, light effects, grain and the print, as one
//! serialisable recipe. The desktop app and the CLI render through [`render`] and
//! [`render_region`].

use colour_film::{
    Developed, FilmStock, GrainAdjust, LightEffects, PrintStock, Printing, Region, RenderOptions,
};
use fd_core::{CalibrationLevel, FilmGeometry, GeometryError, SceneImage};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::CompressionSpec;
pub use colour_film::{Compression, FilmKind, film_stocks, print_stocks};

#[derive(Debug, Error)]
pub enum StyleError {
    #[error("unknown film stock {0:?}")]
    UnknownFilm(String),
    #[error("unknown print stock {0:?}")]
    UnknownPrint(String),
    #[error(transparent)]
    Geometry(#[from] GeometryError),
}

/// Creative grain adjustments; the default is the film's own grain.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GrainSpec {
    pub amount: f64,
    pub size: f64,
    pub roughness: f64,
}

impl Default for GrainSpec {
    fn default() -> Self {
        let a = GrainAdjust::default();
        Self {
            amount: a.amount,
            size: a.size,
            roughness: a.roughness,
        }
    }
}

impl GrainSpec {
    pub fn adjust(&self) -> GrainAdjust {
        GrainAdjust {
            amount: self.amount,
            size: self.size,
            roughness: self.roughness,
        }
    }

    pub fn is_film_grain(&self) -> bool {
        *self == Self::default()
    }
}

/// Light effects: halation (relative to the film's own), bloom and vignette.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LightSpec {
    pub halation: f64,
    pub halation_size: f64,
    pub bloom: f64,
    pub bloom_size: f64,
    pub vignette: f64,
}

impl Default for LightSpec {
    fn default() -> Self {
        let l = LightEffects::default();
        Self {
            halation: l.halation,
            halation_size: l.halation_size,
            bloom: l.bloom,
            bloom_size: l.bloom_size,
            vignette: l.vignette,
        }
    }
}

impl LightSpec {
    fn effects(&self) -> LightEffects {
        LightEffects {
            halation: self.halation,
            halation_size: self.halation_size,
            bloom: self.bloom,
            bloom_size: self.bloom_size,
            vignette: self.vignette,
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Printing: exposure and colour on top of the grey-timed print.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintSpec {
    /// Print exposure in stops relative to the timed print: more is darker.
    pub exposure_stops: f64,
    /// Printer points of red, green and blue to add to the print.
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    /// Apply the stock's typical lab rendering.
    pub stock_look: bool,
}

impl Default for PrintSpec {
    fn default() -> Self {
        Self {
            exposure_stops: 0.0,
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            stock_look: true,
        }
    }
}

impl PrintSpec {
    fn printing(&self) -> Printing {
        Printing {
            exposure_stops: self.exposure_stops,
            colour_points: [self.red, self.green, self.blue],
            stock_look: self.stock_look,
        }
    }
}

/// A complete film → process → print recipe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub name: String,
    pub film: String,
    /// Print stock id ("scan" for a neutral scan). Ignored for slide film.
    pub print: String,
    /// Camera exposure compensation, stops.
    #[serde(default)]
    pub ev_compensation: f64,
    /// Push (+) or pull (−), stops.
    #[serde(default)]
    pub push: f64,
    /// Multiplier on the film's interlayer (colour saturation) effect.
    #[serde(default = "one")]
    pub interlayer: f64,
    #[serde(default)]
    pub printing: PrintSpec,
    #[serde(default, skip_serializing_if = "CompressionSpec::is_default")]
    pub compression: CompressionSpec,
    #[serde(default, skip_serializing_if = "LightSpec::is_default")]
    pub light: LightSpec,
    #[serde(default, skip_serializing_if = "GrainSpec::is_film_grain")]
    pub grain: GrainSpec,
    #[serde(default = "one_u64")]
    pub seed: u64,
    /// Width of the film frame in mm (36 = 35 mm, 56 = 6×6, 120 = 4×5).
    #[serde(default = "default_format")]
    pub format_mm: f64,
    /// How much of the frame's width the image uses: 2 = the central half enlarged.
    #[serde(default = "one")]
    pub crop: f64,
}

fn one() -> f64 {
    1.0
}

fn one_u64() -> u64 {
    1
}

fn default_format() -> f64 {
    36.0
}

/// How visible the grain will be, on the same scale as BW Darkroom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainReadout {
    /// Diffuse RMS granularity ×1000 at net density 1.0 (green layer), after processing.
    pub granularity: f64,
    /// Granularity scaled to the enlargement of a full 35 mm frame.
    pub at_print: f64,
}

impl GrainReadout {
    pub fn describe(&self) -> &'static str {
        match self.at_print {
            g if g < 8.0 => "very fine",
            g if g < 12.0 => "fine",
            g if g < 17.0 => "moderate",
            g if g < 24.0 => "pronounced",
            g if g < 34.0 => "coarse",
            _ => "very coarse",
        }
    }
}

/// What the Develop panel shows about the model.
pub struct ModelSummary {
    pub film_name: &'static str,
    pub print_name: &'static str,
    pub kind: FilmKind,
    pub level: CalibrationLevel,
    pub grain: GrainReadout,
    /// Net density against log exposure (0 = metered grey), per layer R, G, B.
    pub curves: [Vec<(f64, f64)>; 3],
}

impl Style {
    pub fn frame_width_mm(&self) -> f64 {
        self.format_mm / self.crop.max(1.0)
    }

    pub fn film_stock(&self) -> Result<FilmStock, StyleError> {
        colour_film::film_stock(&self.film)
            .ok_or_else(|| StyleError::UnknownFilm(self.film.clone()))
    }

    pub fn print_stock(&self) -> Result<PrintStock, StyleError> {
        colour_film::print_stock(&self.print)
            .ok_or_else(|| StyleError::UnknownPrint(self.print.clone()))
    }

    pub fn is_slide(&self) -> bool {
        self.film_stock()
            .is_ok_and(|s| s.kind == FilmKind::Reversal)
    }

    pub fn model_summary(&self) -> Result<ModelSummary, StyleError> {
        let stock = self.film_stock()?;
        let print = self.print_stock()?;
        let developed = Developed::new(&stock, self.push);
        let granularity = developed.grain_rms * stock.grain.layers[1] * self.grain.amount.max(0.0);
        Ok(ModelSummary {
            film_name: stock.name,
            print_name: if stock.kind == FilmKind::Reversal {
                "viewed directly"
            } else {
                print.name
            },
            kind: stock.kind,
            level: stock.level.min(print.level),
            grain: GrainReadout {
                granularity,
                at_print: granularity * 36.0 / self.frame_width_mm(),
            },
            curves: developed.curves(-3.0, 2.5, 111),
        })
    }

    fn options(&self, geometry: FilmGeometry, region: Option<Region>) -> RenderOptions {
        RenderOptions {
            seed: self.seed,
            geometry,
            ev_compensation: self.ev_compensation,
            push: self.push,
            interlayer: self.interlayer,
            compression: self.compression.compression(),
            grain: self.grain.adjust(),
            light: self.light.effects(),
            printing: self.printing.printing(),
            region,
        }
    }
}

pub use colour_film::Rendered;

/// Render a whole frame. `frame_width_mm` is the width of film the image spans.
pub fn render(
    scene: &SceneImage,
    style: &Style,
    frame_width_mm: f64,
) -> Result<Rendered, StyleError> {
    let (w, h) = scene.dimensions();
    let geometry = FilmGeometry::from_frame_width(frame_width_mm, w, h)?;
    let (stock, print) = (style.film_stock()?, style.print_stock()?);
    Ok(colour_film::render(
        scene,
        &stock,
        &print,
        &style.options(geometry, None),
    ))
}

/// Render a crop whose top-left pixel sits at `origin` in a frame of `frame` pixels (at
/// the crop's resolution), spanning `crop_width_mm` of film. Grain and vignette are that
/// region's, so a 1:1 detail view matches the export. Halation and bloom see only the
/// crop, so light from outside it does not spill in.
pub fn render_region(
    scene: &SceneImage,
    style: &Style,
    crop_width_mm: f64,
    origin: [u32; 2],
    frame: [usize; 2],
) -> Result<Rendered, StyleError> {
    let (w, h) = scene.dimensions();
    let geometry = FilmGeometry::from_frame_width(crop_width_mm, w, h)?;
    let (stock, print) = (style.film_stock()?, style.print_stock()?);
    let region = Some(Region { origin, frame });
    Ok(colour_film::render(
        scene,
        &stock,
        &print,
        &style.options(geometry, region),
    ))
}

/// Everything needed to reproduce an export, written next to it as JSON.
#[derive(Serialize)]
pub struct ExportRecord<'a> {
    pub engine_version: &'static str,
    pub source: String,
    pub style: &'a Style,
    pub frame_width_mm: f64,
    pub calibration_level: &'static str,
    pub note: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_adjustments: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
}

impl<'a> ExportRecord<'a> {
    pub fn new(source: String, style: &'a Style, frame_width_mm: f64) -> Self {
        let level = style
            .model_summary()
            .map(|m| m.level.label())
            .unwrap_or("unknown");
        Self {
            engine_version: env!("CARGO_PKG_VERSION"),
            source,
            style,
            frame_width_mm,
            calibration_level: level,
            note: "Film and print stock parameters are illustrative, not fitted to manufacturer data.",
            image_adjustments: None,
            camera: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("record is serialisable")
    }
}

pub fn film_label(id: &str) -> String {
    colour_film::film_stock(id).map_or_else(|| id.to_string(), |s| s.name.to_string())
}

pub fn print_label(id: &str) -> String {
    colour_film::print_stock(id).map_or_else(|| id.to_string(), |s| s.name.to_string())
}

/// The shipped styles: each film on its natural print, plus some classic combinations.
pub fn builtin_styles() -> Vec<Style> {
    let s = |name: &str, film: &str, print: &str| Style {
        name: name.into(),
        film: film.into(),
        print: print.into(),
        ev_compensation: 0.0,
        push: 0.0,
        interlayer: 1.0,
        printing: PrintSpec::default(),
        compression: CompressionSpec::default(),
        light: LightSpec::default(),
        grain: GrainSpec::default(),
        seed: 1,
        format_mm: 36.0,
        crop: 1.0,
    };
    let mut styles = vec![
        s("Portra 400 · scan", "portra-400", "scan"),
        s("Portra 400 · Endura", "portra-400", "endura"),
        s("Portra 160 · scan", "portra-160", "scan"),
        s("Portra 800 · scan", "portra-800", "scan"),
        s("Ektar 100 · scan", "ektar-100", "scan"),
        s("Gold 200 · scan", "gold-200", "scan"),
        s("UltraMax 400 · scan", "ultramax-400", "scan"),
        s("Pro 400H · Crystal Archive", "pro-400h", "crystal-archive"),
        s("Superia 400 · scan", "superia-400", "scan"),
        s("Vision3 250D · 2383", "vision3-250d", "2383"),
        s("Vision3 500T · 2383", "vision3-500t", "2383"),
        s("Vision3 500T · 3513", "vision3-500t", "3513"),
        s("CineStill 800T · scan", "cinestill-800t", "scan"),
        s("Ektachrome E100", "ektachrome-e100", "scan"),
        s("Velvia 50", "velvia-50", "scan"),
        s("Provia 100F", "provia-100f", "scan"),
    ];
    let mut over = s("Portra 400 overexposed +1", "portra-400", "scan");
    over.ev_compensation = 1.0;
    styles.insert(2, over);
    let mut pushed = s("Portra 800 pushed 1 stop", "portra-800", "scan");
    pushed.push = 1.0;
    styles.insert(5, pushed);
    let mut medium = s("Portra 160 · 6×7", "portra-160", "scan");
    medium.format_mm = 70.0;
    styles.insert(4, medium);
    let mut soft = s("Portra 400 · film compression", "portra-400", "scan");
    soft.compression = CompressionSpec {
        impact: 60.0,
        colour_density: 40.0,
        ..Default::default()
    };
    styles.insert(3, soft);
    let mut dreamy = s("CineStill 800T · bloom", "cinestill-800t", "scan");
    dreamy.light.bloom = 0.15;
    dreamy.light.vignette = 0.5;
    styles.push(dreamy);
    styles
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_core::Plane;

    #[test]
    fn builtins_resolve_and_have_unique_names() {
        let styles = builtin_styles();
        for (i, s) in styles.iter().enumerate() {
            assert!(s.model_summary().is_ok(), "{}", s.name);
            assert!(
                styles[i + 1..].iter().all(|o| o.name != s.name),
                "{}",
                s.name
            );
        }
    }

    #[test]
    fn styles_round_trip_and_old_files_get_defaults() {
        let mut s = builtin_styles().remove(0);
        s.light.bloom = 0.2;
        s.grain.amount = 1.5;
        s.printing.red = 3.0;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Style>(&json).unwrap(), s);
        let minimal: Style =
            serde_json::from_str(r#"{"name":"x","film":"portra-400","print":"scan"}"#).unwrap();
        assert_eq!(minimal.interlayer, 1.0);
        assert_eq!(minimal.format_mm, 36.0);
        assert!(minimal.printing.stock_look);
        assert!(minimal.grain.is_film_grain());
        assert!(minimal.compression.is_default());
    }

    #[test]
    fn region_render_matches_the_frame_away_from_light_effects() {
        let mut style = builtin_styles().remove(0);
        style.light.halation = 0.0;
        let (w, h) = (120, 80);
        let p =
            |k: f32| Plane::from_fn(w, h, move |x, y| k * (0.05 + (x + y) as f32 / 400.0)).unwrap();
        let scene = SceneImage::Rgb {
            r: p(1.0),
            g: p(0.9),
            b: p(0.7),
        };
        let full = render(&scene, &style, 3.0).unwrap();
        let crop_scene = SceneImage::Rgb {
            r: Plane::from_fn(40, 30, |x, y| 0.05 + (x + 50 + y + 20) as f32 / 400.0).unwrap(),
            g: Plane::from_fn(40, 30, |x, y| {
                0.9 * (0.05 + (x + 50 + y + 20) as f32 / 400.0)
            })
            .unwrap(),
            b: Plane::from_fn(40, 30, |x, y| {
                0.7 * (0.05 + (x + 50 + y + 20) as f32 / 400.0)
            })
            .unwrap(),
        };
        let part = render_region(&crop_scene, &style, 1.0, [50, 20], [w, h]).unwrap();
        // Away from the crop edge (where edge effects see different neighbours) it matches.
        let a = part.image[1].get(20, 15);
        let b = full.image[1].get(70, 35);
        assert!((a - b).abs() < 2e-3, "{a} vs {b}");
    }
}
