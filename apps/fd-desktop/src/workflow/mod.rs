//! The Develop panel's process sections. Black and white (`mono`): Film, Development,
//! Film compression, Grain and Print. Colour (`colour`): Film, Film compression, Light,
//! Grain and Print. Film compression is shared (here), and so is the Image section before
//! the film (`image_panel`).

pub mod colour;
pub mod mono;

use eframe::egui::{self, Pos2, Sense, Stroke, Vec2};
use fd_profiles::{CompressionSpec, ModelSummary, Style};

use crate::widgets::{self, SliderSpec, inset};

mod help {
    pub const COMPRESSION: &str = "Film compression: rolls the photo's highlights off towards the mid-tones, the way a negative\nkeeps absorbing light where a sensor clips. Gives a softer, more analogue highlight range.";
    pub const IMPACT: &str = "How strongly highlights are pulled towards the mid-tones. 0 is off.";
    pub const TONAL_RANGE: &str = "How far down the tones compression reaches: low values touch only the brightest highlights,\n100 reaches almost to the deep shadows.";
    pub const WHITE_POINT: &str = "Top of the compressed range. Lower: more highlight contrast, clipping sooner.\nHigher: flatter, greyer highlights. 100 is neutral.";
    pub const COLOUR_DENSITY: &str = "Saturation of the compressed highlights. 0 fades them towards neutral, like colour negative;\n100 keeps their colour, like slide film.";
    pub const COMPRESSION_CURVE: &str = "Scene stops in (across) against stops out (up), 0 = mid-grey.\nThe diagonal is no change; the line bends where highlights are compressed.";
}

fn spec<'a>(
    id: &'a str,
    label: &'a str,
    help: &'a str,
    (min, max, default, step): (f64, f64, f64, f64),
    decimals: usize,
    suffix: &'a str,
) -> SliderSpec<'a> {
    SliderSpec {
        id,
        label,
        help,
        min,
        max,
        default,
        step,
        decimals,
        suffix,
        signed: false,
        log: false,
    }
}

/// Section and expandable-row keys of both processes (for "Edit…", which opens them all).
pub fn section_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = mono::SECTION_KEYS.to_vec();
    for k in colour::SECTION_KEYS {
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    keys
}

/// The sections for the draft's process.
pub fn sections(ui: &mut egui::Ui, draft: &mut Style, summary: Option<&ModelSummary>) {
    match draft {
        Style::Mono(s) => {
            let summary = summary.and_then(ModelSummary::as_mono);
            widgets::section(ui, "film", "Film", true, |ui| mono::film_section(ui, s));
            widgets::section(ui, "development", "Development", true, |ui| {
                mono::development_section(ui, s, summary)
            });
            widgets::section(ui, "compression", "Film compression", false, |ui| {
                compression_section(ui, &mut s.compression, false)
            });
            widgets::section(ui, "grain", "Grain", false, |ui| {
                mono::grain_section(ui, s, summary)
            });
            widgets::section(ui, "print", "Print", false, |ui| mono::print_section(ui, s));
        }
        Style::Colour(s) => {
            let summary = summary.and_then(ModelSummary::as_colour);
            widgets::section(ui, "film", "Film", true, |ui| {
                colour::film_section(ui, s, summary)
            });
            widgets::section(ui, "compression", "Film compression", false, |ui| {
                compression_section(ui, &mut s.compression, true)
            });
            widgets::section(ui, "light", "Light", false, |ui| {
                colour::light_section(ui, s)
            });
            widgets::section(ui, "grain", "Grain", false, |ui| {
                colour::grain_section(ui, s, summary)
            });
            widgets::section(ui, "print", "Print", true, |ui| {
                colour::print_section(ui, s)
            });
        }
    }
}

/// Film compression: the highlights rolled off before the film (both processes; colour
/// density only for colour).
pub fn compression_section(ui: &mut egui::Ui, c: &mut CompressionSpec, colour: bool) {
    widgets::note(
        ui,
        "Rolls highlights off towards the mid-tones, as film does.",
        help::COMPRESSION,
    );
    let on = c.impact > 0.0;
    widgets::slider(
        ui,
        &spec(
            "compression.impact",
            "Impact",
            help::IMPACT,
            (0.0, 100.0, 0.0, 1.0),
            0,
            "",
        ),
        &mut c.impact,
        true,
    );
    widgets::slider(
        ui,
        &spec(
            "compression.tonal_range",
            "Tonal range",
            help::TONAL_RANGE,
            (0.0, 100.0, 50.0, 1.0),
            0,
            "",
        ),
        &mut c.tonal_range,
        on,
    );
    widgets::slider(
        ui,
        &spec(
            "compression.white_point",
            "White point",
            help::WHITE_POINT,
            (50.0, 120.0, 100.0, 1.0),
            0,
            "",
        ),
        &mut c.white_point,
        on,
    );
    if colour {
        widgets::slider(
            ui,
            &spec(
                "compression.colour_density",
                "Colour density",
                help::COLOUR_DENSITY,
                (0.0, 100.0, 100.0, 1.0),
                0,
                "",
            ),
            &mut c.colour_density,
            on,
        );
    }
    inset(ui, |ui| compression_plot(ui, &c.compression()));
}

/// Stops in against stops out, with the identity diagonal.
fn compression_plot(ui: &mut egui::Ui, c: &fd_core::Compression) {
    let width = ui.available_width().min(320.0);
    let (response, painter) = ui.allocate_painter(Vec2::new(width, 110.0), Sense::hover());
    let rect = response.rect.shrink(4.0);
    let (lo, hi) = (-6.0, 8.0);
    let to_screen = |u: f64, v: f64| {
        Pos2::new(
            rect.min.x + ((u - lo) / (hi - lo)) as f32 * rect.width(),
            rect.max.y - ((v.clamp(lo, hi) - lo) / (hi - lo)) as f32 * rect.height(),
        )
    };
    let grid = Stroke::new(0.5, ui.visuals().weak_text_color().gamma_multiply(0.5));
    for k in [-4.0, -2.0, 0.0, 2.0, 4.0, 6.0] {
        painter.line_segment([to_screen(k, lo), to_screen(k, hi)], grid);
        painter.line_segment([to_screen(lo, k), to_screen(hi, k)], grid);
    }
    painter.line_segment(
        [to_screen(lo, lo), to_screen(hi, hi)],
        Stroke::new(1.0, ui.visuals().weak_text_color()),
    );
    let points: Vec<Pos2> = c
        .curve(lo, hi, 113)
        .into_iter()
        .map(|(u, v)| to_screen(u, v))
        .collect();
    painter.add(egui::Shape::line(
        points,
        Stroke::new(1.8, ui.visuals().strong_text_color()),
    ));
    response.on_hover_text(help::COMPRESSION_CURVE);
}
