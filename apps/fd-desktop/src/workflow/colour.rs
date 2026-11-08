//! The Develop panel's sections: Film, Light, Grain and Print, with advanced controls
//! in expandable rows. Every control carries a one- or two-line hover explanation of
//! what it does and how it changes the image.

use eframe::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};
use fd_profiles::colour::{FilmKind, GrainSpec, ModelSummary, Style};

use crate::widgets::{self, Icon, Segment, SliderSpec, inset};

mod help {
    pub const FILM: &str = "The film stock. It sets colour rendering, contrast, latitude, grain and halation.\nNegative films are printed or scanned (see Print); slide films are viewed directly.";
    pub const FORMAT: &str = "Film size. A larger negative is enlarged less for the same print, so its grain looks finer\nand its halation tighter.";
    pub const CROP: &str =
        "Use only the centre of the frame. Cropping means enlarging the film more, so grain grows.";
    pub const PUSH: &str = "Develop longer (push) or shorter (pull). Your photo's exposure is kept:\na push adds contrast, saturation and grain; a pull softens. Slides get lighter when pushed.";
    pub const EV: &str = "Under- or over-exposes the film in stops. The lab still prints mid-grey as mid-grey, so for\nnegatives this changes character, not brightness: +1 is the classic richer, cleaner overexposed\nnegative; −2 is thin, muddy and grainy. Slides simply get lighter or darker.";
    pub const INTERLAYER: &str = "Strength of the film's interlayer (DIR coupler) effects: how much each colour layer\nholds back the others. More gives purer, more saturated colour; neutrals never change.";
    pub const CURVES: &str = "Net density of each dye layer against log exposure (0 = the metered mid-grey).\nRed, green and blue lines are the cyan, magenta and yellow dye layers.";
    pub const HALATION: &str = "Light reflected from the film base back into the emulsion: a red-orange glow round\nbright lights. ×1 is the film's own (faint with an anti-halation backing, strong without).";
    pub const HALATION_SIZE: &str = "Spread of the halation glow, relative to the film's own.";
    pub const BLOOM: &str =
        "A soft glow over highlights, like a diffusion filter or an old uncoated lens.\n0 is off.";
    pub const BLOOM_SIZE: &str = "How far the bloom spreads, as a share of the frame width.";
    pub const VIGNETTE: &str = "Lens fall-off: exposure lost in the frame's corners, in stops.";
    pub const NEW_PATTERN: &str = "Same grain amount and character, a different random pattern.";
    pub const GRAIN_SIZE: &str = "Coarseness of the dye clouds, relative to the film's own. Strength stays the same.\n×1 is the film's grain.";
    pub const GRAIN_AMOUNT: &str =
        "Strength of the grain, relative to the film's own. ×1 is the film's grain; 0 removes it.";
    pub const GRAIN_ROUGHNESS: &str = "Character of the grain: below 0 softer and smoother; above 0 grittier, with more\nisolated specks. 0 is the film's grain.";
    pub const RANDOMISE: &str =
        "A new random pattern with random size, amount and roughness, within a usable range.";
    pub const FILM_GRAIN: &str = "Back to the film's own grain: size ×1, amount ×1, roughness 0.";
    pub const GRAIN_READOUT: &str = "Diffuse RMS granularity at this enlargement (the same scale as black and white).\nAround 10 looks smooth; above 25 looks gritty.";
    pub const PRINT: &str = "How the negative becomes a positive: scanned with ideal dyes, printed on colour paper,\nor printed on cinema print film. Each has its own contrast, blacks and dye colours.";
    pub const PRINT_EXPOSURE: &str = "Printer exposure relative to the lab's timing (mid-grey prints as mid-grey).\nMore exposure makes the print darker; less makes it lighter.";
    pub const RED: &str = "Printer points of red: positive takes red light out of the printer, so the print gets redder;\nnegative gives cyan. One point is about 1/12 stop.";
    pub const GREEN: &str =
        "Printer points of green: positive makes the print greener; negative gives magenta.";
    pub const BLUE: &str =
        "Printer points of blue: positive makes the print bluer; negative gives yellow.";
    pub const STOCK_LOOK: &str = "Apply the lab rendering this film is known for (for example Gold's warmth, 400H's mint green),\non top of grey timing. Off: strictly neutral greys.";
    pub const SLIDE_COLOUR: &str = "Slide film has no print: these act as colour-compensating filters on the camera.\nPositive adds that colour.";
    pub const PROJECTION: &str = "Slide brightness on the light box: more makes the slide darker.";
}

/// Display order and short names for films.
fn films() -> Vec<(&'static str, &'static str, &'static str, FilmKind)> {
    fd_profiles::colour::film_stocks()
        .into_iter()
        .map(|s| (s.id, s.name, s.about, s.kind))
        .collect()
}

const FORMATS: [(&str, f64, &str); 5] = [
    (
        "35 mm",
        36.0,
        "36 × 24 mm frame: the most grain for a given print size.",
    ),
    (
        "6×6 / 645",
        56.0,
        "56 mm wide medium-format frame (6×6 or 645): about 1.6× less enlargement than 35 mm.",
    ),
    (
        "6×7",
        70.0,
        "70 × 56 mm medium-format frame: about 2× less enlargement than 35 mm.",
    ),
    (
        "4×5",
        120.0,
        "4 × 5 inch sheet film: almost grainless at normal print sizes.",
    ),
    ("8×10", 203.0, "8 × 10 inch sheet film: no visible grain."),
];

const PUSHES: [(&str, f64); 6] = [
    ("Pull 2", -2.0),
    ("Pull 1", -1.0),
    ("Box", 0.0),
    ("Push 1", 1.0),
    ("Push 2", 2.0),
    ("Push 3", 3.0),
];

/// Random grain within a usable range, and a new pattern.
fn randomise_grain(s: &mut Style) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut state = nanos ^ s.seed.rotate_left(17);
    // SplitMix64: plenty for picking slider values.
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut unit = || (next() >> 11) as f64 / (1u64 << 53) as f64;
    let round2 = |v: f64| (v * 100.0).round() / 100.0;
    s.grain = GrainSpec {
        size: round2(0.7 * (2.2f64 / 0.7).powf(unit())),
        amount: round2(0.6 + 1.4 * unit()),
        roughness: round2(-0.6 + 1.4 * unit()),
    };
    s.seed = next() % 100_000;
}

/// Section and expandable-row keys, in panel order (for "Edit…", which opens them all).
pub const SECTION_KEYS: [&str; 8] = [
    "film",
    "film.details",
    "film.curve",
    "compression",
    "light",
    "grain",
    "print",
    "image",
];

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

/// Film: stock, format, enlargement and processing; details and curves in flyouts.
pub fn film_section(ui: &mut egui::Ui, s: &mut Style, summary: Option<&ModelSummary>) {
    let films = films();
    let current = fd_profiles::colour::film_label(&s.film);
    let mut chosen = None;
    widgets::dropdown(ui, "film", "Film", &current, help::FILM, |ui| {
        let mut last_kind = None;
        for (id, label, about, kind) in &films {
            if last_kind.is_some_and(|k| k != *kind) {
                ui.separator();
            }
            last_kind = Some(*kind);
            if ui
                .add(egui::Button::selectable(s.film == *id, *label))
                .on_hover_text(*about)
                .clicked()
            {
                chosen = Some(*id);
            }
        }
    });
    if let Some(id) = chosen {
        s.film = id.to_string();
    }
    if let Some((_, _, about, kind)) = films.iter().find(|(id, ..)| *id == s.film) {
        let kind = match kind {
            FilmKind::Negative => "Colour negative",
            FilmKind::Reversal => "Slide film",
        };
        widgets::note(
            ui,
            format!("{kind}. {}", about.lines().next().unwrap_or("")),
            about,
        );
    }
    widgets::subtitle(ui, "Format", help::FORMAT);
    let formats: Vec<Segment> = FORMATS
        .iter()
        .map(|(label, mm, help)| Segment {
            label,
            selected: (s.format_mm - mm).abs() < 0.5,
            enabled: true,
            help,
        })
        .collect();
    if let Some(i) = widgets::segmented(ui, &formats, 3) {
        s.format_mm = FORMATS[i].1;
    }
    widgets::slider(
        ui,
        &spec(
            "film.enlarge",
            "Enlarge",
            help::CROP,
            (1.0, 4.0, 1.0, 0.1),
            1,
            "×",
        ),
        &mut s.crop,
        true,
    );

    widgets::subtitle(ui, "Push / pull", help::PUSH);
    let segments: Vec<Segment> = PUSHES
        .iter()
        .map(|(label, stops)| Segment {
            label,
            selected: (s.push - stops).abs() < 0.17,
            enabled: true,
            help: help::PUSH,
        })
        .collect();
    if let Some(i) = widgets::segmented(ui, &segments, 3) {
        s.push = PUSHES[i].1;
    }
    let ev = SliderSpec {
        signed: true,
        ..spec(
            "film.ev",
            "Exposure",
            help::EV,
            (-3.0, 3.0, 0.0, 1.0 / 3.0),
            2,
            " EV",
        )
    };
    widgets::slider(ui, &ev, &mut s.ev_compensation, true);
    ui.add_space(4.0);

    widgets::flyout(
        ui,
        "film.details",
        "Colour layers",
        "Interlayer effect: how strongly the dye layers hold each other back",
        Icon::Dial,
        |ui| {
            widgets::slider(
                ui,
                &spec(
                    "film.interlayer",
                    "Interlayer",
                    help::INTERLAYER,
                    (0.0, 2.5, 1.0, 0.05),
                    2,
                    "×",
                ),
                &mut s.interlayer,
                true,
            );
        },
    );
    widgets::flyout(
        ui,
        "film.curve",
        "Characteristic curves",
        help::CURVES,
        Icon::Curve,
        |ui| inset(ui, |ui| model_panel(ui, summary)),
    );
}

/// Light: halation, bloom and vignette.
pub fn light_section(ui: &mut egui::Ui, s: &mut Style) {
    let l = &mut s.light;
    widgets::slider(
        ui,
        &spec(
            "light.halation",
            "Halation",
            help::HALATION,
            (0.0, 4.0, 1.0, 0.05),
            2,
            "×",
        ),
        &mut l.halation,
        true,
    );
    let size = SliderSpec {
        log: true,
        ..spec(
            "light.halation_size",
            "Halation size",
            help::HALATION_SIZE,
            (0.25, 4.0, 1.0, 0.01),
            2,
            "×",
        )
    };
    widgets::slider(ui, &size, &mut l.halation_size, l.halation > 0.0);
    let mut bloom = l.bloom * 100.0;
    if widgets::slider(
        ui,
        &spec(
            "light.bloom",
            "Bloom",
            help::BLOOM,
            (0.0, 60.0, 0.0, 1.0),
            0,
            "%",
        ),
        &mut bloom,
        true,
    ) {
        l.bloom = bloom / 100.0;
    }
    let mut bloom_size = l.bloom_size * 100.0;
    let bs = SliderSpec {
        log: true,
        ..spec(
            "light.bloom_size",
            "Bloom size",
            help::BLOOM_SIZE,
            (0.3, 10.0, 2.0, 0.1),
            1,
            "%",
        )
    };
    if widgets::slider(ui, &bs, &mut bloom_size, l.bloom > 0.0) {
        l.bloom_size = bloom_size / 100.0;
    }
    widgets::slider(
        ui,
        &spec(
            "light.vignette",
            "Vignette",
            help::VIGNETTE,
            (0.0, 2.0, 0.0, 0.05),
            2,
            " stops",
        ),
        &mut l.vignette,
        true,
    );
}

/// Grain: size, amount and roughness relative to the film's own grain.
pub fn grain_section(ui: &mut egui::Ui, s: &mut Style, summary: Option<&ModelSummary>) {
    let size = SliderSpec {
        log: true,
        ..spec(
            "grain.size",
            "Size",
            help::GRAIN_SIZE,
            (0.5, 3.0, 1.0, 0.01),
            2,
            "×",
        )
    };
    widgets::slider(ui, &size, &mut s.grain.size, true);
    widgets::slider(
        ui,
        &spec(
            "grain.amount",
            "Amount",
            help::GRAIN_AMOUNT,
            (0.0, 3.0, 1.0, 0.01),
            2,
            "×",
        ),
        &mut s.grain.amount,
        true,
    );
    let mut rough = s.grain.roughness * 100.0;
    let roughness = SliderSpec {
        signed: true,
        ..spec(
            "grain.roughness",
            "Roughness",
            help::GRAIN_ROUGHNESS,
            (-100.0, 100.0, 0.0, 1.0),
            0,
            "",
        )
    };
    if widgets::slider(ui, &roughness, &mut rough, true) {
        s.grain.roughness = rough / 100.0;
    }
    inset(ui, |ui| {
        ui.horizontal(|ui| {
            if widgets::text_button(ui, "Film's grain", false, !s.grain.is_film_grain())
                .on_hover_text(help::FILM_GRAIN)
                .clicked()
            {
                s.grain = GrainSpec::default();
            }
            if widgets::text_button(ui, "Randomise", false, true)
                .on_hover_text(help::RANDOMISE)
                .clicked()
            {
                randomise_grain(s);
            }
            if widgets::text_button(ui, "New pattern", false, true)
                .on_hover_text(help::NEW_PATTERN)
                .clicked()
            {
                s.seed = s.seed.wrapping_add(1);
            }
        });
    });
    ui.add_space(6.0);
    if let Some(m) = summary {
        let adjusted = if s.grain.is_film_grain() {
            ""
        } else {
            " · adjusted from the film"
        };
        widgets::note(
            ui,
            format!(
                "Grain at print: {} ({:.0}){adjusted}",
                m.grain.describe(),
                m.grain.at_print
            ),
            help::GRAIN_READOUT,
        );
    }
    widgets::note(ui, "Press D and click the image to judge grain at 1:1.", "");
}

/// Print: print stock, exposure and colour in printer points (or slide filters).
pub fn print_section(ui: &mut egui::Ui, s: &mut Style) {
    let slide = s.is_slide();
    if slide {
        widgets::note(
            ui,
            "Slide film is viewed directly: no print stock.",
            help::SLIDE_COLOUR,
        );
    } else {
        let prints = fd_profiles::colour::print_stocks();
        let current = fd_profiles::colour::print_label(&s.print);
        let mut chosen = None;
        widgets::dropdown(ui, "print", "Print", &current, help::PRINT, |ui| {
            for p in &prints {
                if ui
                    .add(egui::Button::selectable(s.print == p.id, p.name))
                    .on_hover_text(p.about)
                    .clicked()
                {
                    chosen = Some(p.id);
                }
            }
        });
        if let Some(id) = chosen {
            s.print = id.to_string();
        }
        if let Some(p) = prints.iter().find(|p| p.id == s.print) {
            widgets::note(ui, p.about, "");
        }
    }
    let p = &mut s.printing;
    let exposure = SliderSpec {
        signed: true,
        ..spec(
            "print.exposure",
            if slide { "Light box" } else { "Print exposure" },
            if slide {
                help::PROJECTION
            } else {
                help::PRINT_EXPOSURE
            },
            (-2.0, 2.0, 0.0, 1.0 / 12.0),
            2,
            " stops",
        )
    };
    widgets::slider(ui, &exposure, &mut p.exposure_stops, true);
    widgets::subtitle(
        ui,
        if slide {
            "Colour filters"
        } else {
            "Printer lights"
        },
        if slide { help::SLIDE_COLOUR } else { help::RED },
    );
    let colour = |id, label, help| SliderSpec {
        signed: true,
        ..spec(id, label, help, (-15.0, 15.0, 0.0, 0.5), 1, " pt")
    };
    widgets::slider(ui, &colour("print.red", "Red", help::RED), &mut p.red, true);
    widgets::slider(
        ui,
        &colour("print.green", "Green", help::GREEN),
        &mut p.green,
        true,
    );
    widgets::slider(
        ui,
        &colour("print.blue", "Blue", help::BLUE),
        &mut p.blue,
        true,
    );
    inset(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut p.stock_look, "Film's lab look")
                .on_hover_text(help::STOCK_LOOK);
            let neutral = p.red == 0.0 && p.green == 0.0 && p.blue == 0.0;
            if widgets::text_button(ui, "Neutral", false, !neutral)
                .on_hover_text("Set all three printer lights back to 0.")
                .clicked()
            {
                p.red = 0.0;
                p.green = 0.0;
                p.blue = 0.0;
            }
        });
    });
}

fn model_panel(ui: &mut egui::Ui, summary: Option<&ModelSummary>) {
    let Some(summary) = summary else {
        ui.colored_label(ui.visuals().warn_fg_color, "Unknown film or print stock.");
        return;
    };
    let title = match summary.kind {
        FilmKind::Negative => "Negative · characteristic curves",
        FilmKind::Reversal => "Slide · characteristic curves",
    };
    ui.label(egui::RichText::new(title).strong())
        .on_hover_text(help::CURVES);
    curve_plot(ui, summary);
    egui::Grid::new("model")
        .num_columns(2)
        .spacing([12.0, 2.0])
        .show(ui, |ui| {
            ui.label("Print");
            ui.label(summary.print_name);
            ui.end_row();
            ui.label("Calibration").on_hover_text(
                "How the film and print parameters were obtained. Illustrative means chosen by eye, not fitted to data.",
            );
            ui.label(summary.level.label());
            ui.end_row();
        });
}

/// Density vs log exposure per layer, with the metered mid-grey marked.
fn curve_plot(ui: &mut egui::Ui, summary: &ModelSummary) {
    let width = ui.available_width().min(320.0);
    let (response, painter) = ui.allocate_painter(Vec2::new(width, 120.0), Sense::hover());
    let rect = response.rect.shrink(4.0);
    let (x0, x1, y1) = (-3.0, 2.5, 3.5);
    let to_screen = |lh: f64, d: f64| {
        Pos2::new(
            rect.min.x + ((lh - x0) / (x1 - x0)) as f32 * rect.width(),
            rect.max.y - (d.min(y1) / y1) as f32 * rect.height(),
        )
    };
    let grid = Stroke::new(0.5, ui.visuals().weak_text_color().gamma_multiply(0.5));
    for d in [0.0, 1.0, 2.0, 3.0] {
        painter.line_segment([to_screen(x0, d), to_screen(x1, d)], grid);
    }
    for lh in [-3.0, -2.0, -1.0, 0.0, 1.0, 2.0] {
        painter.line_segment([to_screen(lh, 0.0), to_screen(lh, y1)], grid);
    }
    let grey = Stroke::new(1.0, ui.visuals().weak_text_color());
    painter.line_segment([to_screen(0.0, 0.0), to_screen(0.0, y1)], grey);
    let colours = [
        Color32::from_rgb(220, 80, 70),
        Color32::from_rgb(80, 180, 90),
        Color32::from_rgb(80, 130, 230),
    ];
    for (curve, colour) in summary.curves.iter().zip(colours) {
        let points: Vec<Pos2> = curve.iter().map(|&(lh, d)| to_screen(lh, d)).collect();
        painter.add(egui::Shape::line(points, Stroke::new(1.5, colour)));
    }
    response.on_hover_text(help::CURVES);
}
