//! Styles for both processes, and the render entry points the desktop app and the CLI use.
//!
//! A [`Style`] is either a black-and-white recipe ([`mono::Style`]: film, developer,
//! development, paper) or a colour one ([`colour::Style`]: film stock, push, light, print
//! stock). Each process keeps its own engine and settings; this layer only dispatches.
//! Serialised, a style carries `"process": "mono"` or `"process": "colour"`; files written
//! by BW Darkroom or Colour Darkroom have no tag and are read with [`Style::from_json`].

pub mod colour;
pub mod mono;

use fd_core::{Plane, SceneImage};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Which photographic process a style uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Process {
    Mono,
    Colour,
}

impl Process {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mono => "Black and white",
            Self::Colour => "Colour",
        }
    }
}

#[derive(Debug, Error)]
pub enum StyleError {
    #[error(transparent)]
    Mono(#[from] mono::StyleError),
    #[error(transparent)]
    Colour(#[from] colour::StyleError),
}

/// A complete recipe for either process.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "process", rename_all = "lowercase")]
pub enum Style {
    Mono(mono::Style),
    Colour(colour::Style),
}

impl From<mono::Style> for Style {
    fn from(s: mono::Style) -> Self {
        Self::Mono(s)
    }
}

impl From<colour::Style> for Style {
    fn from(s: colour::Style) -> Self {
        Self::Colour(s)
    }
}

impl Style {
    /// Read a style, tagged or not. Untagged styles come from BW Darkroom (they name a
    /// developer) or Colour Darkroom.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        Self::from_value(value)
    }

    /// [`Style::from_json`] for an already parsed value.
    pub fn from_value(value: serde_json::Value) -> serde_json::Result<Self> {
        if value.get("process").is_some() {
            return serde_json::from_value(value);
        }
        if value.get("developer").is_some() {
            serde_json::from_value(value).map(Self::Mono)
        } else {
            serde_json::from_value(value).map(Self::Colour)
        }
    }

    pub fn process(&self) -> Process {
        match self {
            Self::Mono(_) => Process::Mono,
            Self::Colour(_) => Process::Colour,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Mono(s) => &s.name,
            Self::Colour(s) => &s.name,
        }
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        match self {
            Self::Mono(s) => s.name = name.into(),
            Self::Colour(s) => s.name = name.into(),
        }
    }

    /// Film stock id.
    pub fn film(&self) -> &str {
        match self {
            Self::Mono(s) => &s.film,
            Self::Colour(s) => &s.film,
        }
    }

    /// The film's display name.
    pub fn film_label(&self) -> String {
        match self {
            Self::Mono(s) => mono::film_label(&s.film),
            Self::Colour(s) => colour::film_label(&s.film),
        }
    }

    /// Width of film the image spans, in mm.
    pub fn frame_width_mm(&self) -> f64 {
        match self {
            Self::Mono(s) => s.frame_width_mm(),
            Self::Colour(s) => s.frame_width_mm(),
        }
    }

    /// Whether the style can be rendered (known film, developer and print).
    pub fn is_available(&self) -> bool {
        match self {
            Self::Mono(s) => s.is_available(),
            Self::Colour(s) => s.film_stock().is_ok() && s.print_stock().is_ok(),
        }
    }

    pub fn as_mono(&self) -> Option<&mono::Style> {
        match self {
            Self::Mono(s) => Some(s),
            Self::Colour(_) => None,
        }
    }

    pub fn as_colour(&self) -> Option<&colour::Style> {
        match self {
            Self::Colour(s) => Some(s),
            Self::Mono(_) => None,
        }
    }

    pub fn model_summary(&self) -> Result<ModelSummary, StyleError> {
        Ok(match self {
            Self::Mono(s) => ModelSummary::Mono(s.model_summary()?),
            Self::Colour(s) => ModelSummary::Colour(s.model_summary()?),
        })
    }
}

/// What the Develop panel shows about the model, per process.
pub enum ModelSummary {
    Mono(mono::ModelSummary),
    Colour(colour::ModelSummary),
}

impl ModelSummary {
    pub fn film_name(&self) -> &str {
        match self {
            Self::Mono(m) => &m.film_name,
            Self::Colour(m) => m.film_name,
        }
    }

    pub fn calibration_label(&self) -> &'static str {
        match self {
            Self::Mono(m) => m.calibration_level.label(),
            Self::Colour(m) => m.level.label(),
        }
    }

    pub fn as_mono(&self) -> Option<&mono::ModelSummary> {
        match self {
            Self::Mono(m) => Some(m),
            Self::Colour(_) => None,
        }
    }

    pub fn as_colour(&self) -> Option<&colour::ModelSummary> {
        match self {
            Self::Colour(m) => Some(m),
            Self::Mono(_) => None,
        }
    }
}

/// A rendered image: a black-and-white print, or a colour print or slide.
pub enum Rendered {
    Mono(mono::Rendered),
    Colour(colour::Rendered),
}

/// Display-linear output, 1.0 = paper (or screen) white.
pub enum Output {
    Grey(Plane),
    Rgb([Plane; 3]),
}

impl Rendered {
    /// The print, dropping the negative.
    pub fn into_output(self) -> Output {
        match self {
            Self::Mono(r) => Output::Grey(r.print),
            Self::Colour(r) => Output::Rgb(r.image),
        }
    }

    /// Warnings from development (mono only).
    pub fn warnings(&self) -> Vec<String> {
        match self {
            Self::Mono(r) => r.negative.warnings.clone(),
            Self::Colour(_) => Vec::new(),
        }
    }
}

/// Render a whole frame. `frame_width_mm` is the width of film the image spans.
pub fn render(
    scene: &SceneImage,
    style: &Style,
    frame_width_mm: f64,
) -> Result<Rendered, StyleError> {
    Ok(match style {
        Style::Mono(s) => Rendered::Mono(mono::render(scene, s, frame_width_mm)?),
        Style::Colour(s) => Rendered::Colour(colour::render(scene, s, frame_width_mm)?),
    })
}

/// Render a crop whose top-left pixel sits at `origin` in a frame of `frame` pixels (at
/// the crop's resolution), spanning `crop_width_mm` of film, so a 1:1 detail view has
/// exactly the export's grain.
pub fn render_region(
    scene: &SceneImage,
    style: &Style,
    crop_width_mm: f64,
    origin: [u32; 2],
    frame: [usize; 2],
) -> Result<Rendered, StyleError> {
    Ok(match style {
        Style::Mono(s) => Rendered::Mono(mono::render_region(scene, s, crop_width_mm, origin)?),
        Style::Colour(s) => Rendered::Colour(colour::render_region(
            scene,
            s,
            crop_width_mm,
            origin,
            frame,
        )?),
    })
}

/// Everything needed to reproduce an export, written next to it as JSON.
#[derive(Serialize)]
pub struct ExportRecord<'a> {
    pub engine_version: &'static str,
    pub source: String,
    pub style: &'a Style,
    /// Development time actually used, minutes (black and white).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_time_min: Option<f64>,
    pub frame_width_mm: f64,
    pub calibration_level: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub note: &'static str,
    /// Per-image adjustments made before the film (raw exposure, white balance, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_adjustments: Option<serde_json::Value>,
    /// Camera and capture settings, for raw sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
}

impl<'a> ExportRecord<'a> {
    pub fn new(source: String, style: &'a Style, frame_width_mm: f64, rendered: &Rendered) -> Self {
        let calibration_level = style
            .model_summary()
            .map(|m| m.calibration_label())
            .unwrap_or("unknown");
        let (resolved_time_min, note) = match style {
            Style::Mono(s) => (
                Some(s.processing().time_min),
                "Film and developer parameters are emulations of published data or illustrative, \
                 not measurements of the named products.",
            ),
            Style::Colour(_) => (
                None,
                "Film and print stock parameters are illustrative, not fitted to manufacturer data.",
            ),
        };
        Self {
            engine_version: env!("CARGO_PKG_VERSION"),
            source,
            style,
            resolved_time_min,
            frame_width_mm,
            calibration_level,
            warnings: rendered.warnings(),
            note,
            image_adjustments: None,
            camera: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("record is serialisable")
    }
}

/// Every built-in style: black and white first, then colour.
pub fn builtin_styles() -> Vec<Style> {
    mono::builtin_styles()
        .into_iter()
        .map(Style::Mono)
        .chain(colour::builtin_styles().into_iter().map(Style::Colour))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_names_are_unique_across_processes() {
        let styles = builtin_styles();
        let mut names: Vec<&str> = styles.iter().map(Style::name).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "duplicate built-in style names");
        assert!(styles.iter().any(|s| s.process() == Process::Mono));
        assert!(styles.iter().any(|s| s.process() == Process::Colour));
    }

    #[test]
    fn styles_round_trip_with_their_process() {
        for style in builtin_styles() {
            let json = serde_json::to_string(&style).unwrap();
            let tag = match style.process() {
                Process::Mono => "\"process\":\"mono\"",
                Process::Colour => "\"process\":\"colour\"",
            };
            assert!(json.contains(tag), "{json}");
            assert_eq!(Style::from_json(&json).unwrap(), style);
        }
    }

    #[test]
    fn reads_untagged_styles_from_the_single_process_apps() {
        let bw = serde_json::to_string(&mono::builtin_styles()[0]).unwrap();
        assert!(!bw.contains("process"));
        assert_eq!(Style::from_json(&bw).unwrap().process(), Process::Mono);
        let colour = serde_json::to_string(&colour::builtin_styles()[0]).unwrap();
        assert!(!colour.contains("process"));
        assert_eq!(
            Style::from_json(&colour).unwrap().process(),
            Process::Colour
        );
    }

    #[test]
    fn renders_both_processes() {
        let scene = SceneImage::Rgb {
            r: Plane::filled(64, 48, 0.18).unwrap(),
            g: Plane::filled(64, 48, 0.18).unwrap(),
            b: Plane::filled(64, 48, 0.18).unwrap(),
        };
        for style in [
            Style::Mono(mono::builtin_styles().remove(0)),
            Style::Colour(colour::builtin_styles().remove(0)),
        ] {
            let out = render(&scene, &style, 36.0).unwrap().into_output();
            match (style.process(), out) {
                (Process::Mono, Output::Grey(p)) => assert_eq!(p.width(), 64),
                (Process::Colour, Output::Rgb(rgb)) => assert_eq!(rgb[0].height(), 48),
                _ => panic!("wrong output kind"),
            }
        }
    }
}
