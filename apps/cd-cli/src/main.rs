//! `colour`: render photos through film styles from the command line.
//!
//! ```sh
//! colour styles                                   # list built-in styles, films and prints
//! colour render photo.NEF --style "Portra 400 · scan" --out portra.jpg
//! colour render photo.NEF --film ektar-100 --print endura --edge 2000 --out ektar.jpg
//! colour sheet photo.NEF --out sheet.jpg          # every built-in style on one sheet
//! ```

use std::path::{Path, PathBuf};

use cd_core::{Plane, SceneImage};
use cd_profiles::{Style, builtin_styles};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "colour", version, about = "Colour film emulation")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List built-in styles, film stocks and print stocks.
    Styles,
    /// Render one image with a style.
    Render {
        input: PathBuf,
        /// Built-in style name (see `colour styles`).
        #[arg(long)]
        style: Option<String>,
        /// Film stock id, overriding the style's.
        #[arg(long)]
        film: Option<String>,
        /// Print stock id, overriding the style's.
        #[arg(long)]
        print: Option<String>,
        /// Camera exposure compensation, stops.
        #[arg(long, allow_hyphen_values = true)]
        ev: Option<f64>,
        /// Long edge of the output in pixels (default: full size).
        #[arg(long)]
        edge: Option<usize>,
        /// Raw exposure correction, stops (default: automatic for raw files).
        #[arg(long, allow_hyphen_values = true)]
        exposure: Option<f64>,
        /// Defringe edges, 0–100 (raw files).
        #[arg(long, default_value_t = 0.0)]
        defringe: f64,
        /// Multiplier on the film's interlayer effect.
        #[arg(long)]
        interlayer: Option<f64>,
        /// Save the scene before the film (the app's "Original") instead.
        #[arg(long)]
        original: bool,
        #[arg(long)]
        out: PathBuf,
    },
    /// Render every built-in style small and lay them out on one contact sheet.
    Sheet {
        input: PathBuf,
        /// Long edge of each frame.
        #[arg(long, default_value_t = 480)]
        edge: usize,
        #[arg(long, allow_hyphen_values = true)]
        exposure: Option<f64>,
        #[arg(long)]
        out: PathBuf,
    },
}

/// Open a raw or rendered file as a scene reduced to `edge` (if given).
fn load(
    path: &Path,
    edge: Option<usize>,
    exposure: Option<f64>,
    defringe: f64,
) -> Result<SceneImage, String> {
    if cd_raw::is_raw(path) {
        let photo = cd_raw::open(path).map_err(|e| e.to_string())?;
        let photo = match edge {
            Some(e) => photo.downscaled(e),
            None => photo,
        };
        let mut adjust = cd_raw::RawAdjust {
            defringe,
            ..Default::default()
        };
        adjust.exposure_ev = exposure.unwrap_or_else(|| photo.auto_exposure_ev(&adjust));
        Ok(photo.develop(&adjust))
    } else {
        let scene = cd_io::load_scene(path, cd_io::Encoding::Auto).map_err(|e| e.to_string())?;
        let scene = match edge {
            Some(e) => cd_io::downscale(&scene, e),
            None => scene,
        };
        Ok(match exposure {
            Some(ev) => cd_io::scale(&scene, 2f32.powf(ev as f32)),
            None => scene,
        })
    }
}

fn find_style(name: &str) -> Result<Style, String> {
    builtin_styles()
        .into_iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("no built-in style named {name:?}; see `colour styles`"))
}

fn save(rgb: &[Plane; 3], out: &Path) -> Result<(), String> {
    let meta = cd_io::ExportMetadata {
        software: format!("Colour Darkroom CLI {}", env!("CARGO_PKG_VERSION")),
        ..Default::default()
    };
    let ext = out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => cd_io::save_jpeg(rgb, out, 92, &meta),
        "png" => cd_io::save_png16(rgb, out, &meta),
        "tif" | "tiff" => cd_io::save_tiff16(rgb, out, &meta),
        _ => return Err("output must be .jpg, .png or .tif".into()),
    }
    .map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    match Cli::parse().command {
        Command::Styles => {
            println!("Styles:");
            for s in builtin_styles() {
                println!("  {}", s.name);
            }
            println!("\nFilm stocks (illustrative):");
            for f in cd_profiles::film_stocks() {
                println!("  {:<16} {}", f.id, f.name);
            }
            println!("\nPrint stocks (illustrative):");
            for p in cd_profiles::print_stocks() {
                println!("  {:<16} {}", p.id, p.name);
            }
        }
        Command::Render {
            input,
            style,
            film,
            print,
            ev,
            edge,
            exposure,
            defringe,
            interlayer,
            original,
            out,
        } => {
            let mut s = find_style(style.as_deref().unwrap_or("Portra 400 · scan"))?;
            if let Some(f) = film {
                s.film = f;
            }
            if let Some(p) = print {
                s.print = p;
            }
            if let Some(ev) = ev {
                s.ev_compensation = ev;
            }
            if let Some(k) = interlayer {
                s.interlayer = k;
            }
            let scene = load(&input, edge, exposure, defringe)?;
            if original {
                let rgb = scene.to_rgb();
                return save(&rgb, &out);
            }
            let start = std::time::Instant::now();
            let r =
                cd_profiles::render(&scene, &s, s.frame_width_mm()).map_err(|e| e.to_string())?;
            eprintln!("rendered in {} ms", start.elapsed().as_millis());
            save(&r.image, &out)?;
        }
        Command::Sheet {
            input,
            edge,
            exposure,
            out,
        } => {
            let scene = load(&input, Some(edge), exposure, 0.0)?;
            let styles = builtin_styles();
            let (w, h) = scene.dimensions();
            let cols = 4;
            let rows = styles.len().div_ceil(cols);
            let gap = 8;
            let (sw, sh) = (cols * (w + gap) + gap, rows * (h + gap) + gap);
            let mut sheet: [Plane; 3] =
                std::array::from_fn(|_| Plane::filled(sw, sh, 0.05).expect("non-zero"));
            for (k, s) in styles.iter().enumerate() {
                let r = cd_profiles::render(&scene, s, s.frame_width_mm())
                    .map_err(|e| format!("{}: {e}", s.name))?;
                let (ox, oy) = (gap + (k % cols) * (w + gap), gap + (k / cols) * (h + gap));
                for (plane, image) in sheet.iter_mut().zip(&r.image) {
                    for y in 0..h {
                        for x in 0..w {
                            plane.set(ox + x, oy + y, image.get(x, y));
                        }
                    }
                }
                eprintln!("{:>2}. {}", k + 1, s.name);
            }
            save(&sheet, &out)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
