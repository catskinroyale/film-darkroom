//! The Develop panel's process sections. Black and white (`mono`): Film, Development,
//! Grain and Print. Colour (`colour`): Film, Film compression, Light, Grain and Print. The
//! Image section, before the film, is shared (`image_panel`).

pub mod colour;
pub mod mono;

use eframe::egui;
use fd_profiles::{ModelSummary, Style};

use crate::widgets;

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
                colour::compression_section(ui, s)
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
