//! Film Darkroom: browse a folder, preview colour and black-and-white film styles on an
//! image, and export the chosen film → process → print workflow at full resolution.
//!
//! The app is a thin shell over the engine. All photographic work happens in the
//! engine crates; rendering runs on background threads so the UI stays responsive.

// Windows: a GUI app, no console window behind it (debug builds keep one for logs).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

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
            .with_title("Film Darkroom"),
        ..Default::default()
    };
    // macOS takes the icon from the app bundle; elsewhere the window carries it.
    #[cfg(not(target_os = "macos"))]
    let options = eframe::NativeOptions {
        viewport: options.viewport.with_icon(window_icon()),
        ..options
    };
    eframe::run_native(
        "Film Darkroom",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, start)))),
    )
}

/// The app icon for the window and taskbar (Windows, Linux).
#[cfg(not(target_os = "macos"))]
fn window_icon() -> eframe::egui::IconData {
    let png = include_bytes!("../../../docs/images/icon-256.png");
    let rgba = image::load_from_memory(png)
        .expect("embedded icon is a valid PNG")
        .into_rgba8();
    eframe::egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }
}
