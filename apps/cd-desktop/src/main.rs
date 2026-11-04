//! Colour Darkroom: browse a folder, preview colour film looks on an image, and
//! export the chosen film → process → print workflow at full resolution.
//!
//! The app is a thin shell over the engine. All photographic work happens in the
//! engine crates; rendering runs on background threads so the UI stays responsive.

mod about;
mod app;
mod browser;
mod export;
mod folders;
mod image_panel;
#[cfg(target_os = "macos")]
mod menu;
mod source;
mod style_menu;
mod styles;
mod theme;
mod updates;
mod version;
mod widgets;
mod worker;
mod workflow;

fn main() -> eframe::Result {
    // Optional first argument: a folder to open, or an image to open and select.
    let start = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists());
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1480.0, 940.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("Colour Darkroom"),
        ..Default::default()
    };
    eframe::run_native(
        "Colour Darkroom",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, start)))),
    )
}
