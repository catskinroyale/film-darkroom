//! `film`: render photos through colour and black-and-white film styles from the command
//! line.
//!
//! ```sh
//! film styles                                     # built-in styles, films, developers, prints
//! film render photo.NEF --style "Portra 400 · scan" --out portra.jpg
//! film render photo.NEF --style "Tri-X · D-76" --push 2 --grade 3 --out trix.tif
//! film render photo.NEF --film ektar-100 --print endura --edge 2000 --out ektar.jpg
//! film sheet photo.NEF --process mono --out sheet.jpg   # every built-in style on one sheet
//! film variations --input chart --out study/       # black-and-white process study (HTML)
//! ```

mod chart;
mod report;
mod variations;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use fd_core::{Plane, SceneImage};
use fd_profiles::{Output, Process, Style, builtin_styles};

#[derive(Parser)]
#[command(
    name = "film",
    version,
    about = "Colour and black-and-white film emulation"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum ProcessArg {
    Colour,
    Mono,
}

impl From<ProcessArg> for Process {
    fn from(p: ProcessArg) -> Self {
        match p {
            ProcessArg::Colour => Process::Colour,
            ProcessArg::Mono => Process::Mono,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// List built-in styles, films, developers, papers and print stocks.
    Styles,
    /// Render one image with a style.
    Render {
        input: PathBuf,
        /// Built-in style name (see `film styles`). Default: the style of `--film`'s
        /// process, or "Portra 400 · scan".
        #[arg(long)]
        style: Option<String>,
        /// Film id, overriding the style's (a black-and-white film picks a black-and-white
        /// style, a colour film a colour one).
        #[arg(long)]
        film: Option<String>,
        /// Print: a colour print stock id, or a black-and-white paper id.
        #[arg(long)]
        print: Option<String>,
        /// Developer id (black and white).
        #[arg(long)]
        developer: Option<String>,
        /// Paper grade, −1 to 5 (black and white).
        #[arg(long, allow_hyphen_values = true)]
        grade: Option<f64>,
        /// Push (+) or pull (−), stops.
        #[arg(long, allow_hyphen_values = true)]
        push: Option<f64>,
        /// Camera exposure compensation, stops.
        #[arg(long, allow_hyphen_values = true)]
        ev: Option<f64>,
        /// Film compression impact, 0–100.
        #[arg(long)]
        compression: Option<f64>,
        /// Multiplier on the film's interlayer effect (colour).
        #[arg(long)]
        interlayer: Option<f64>,
        /// Long edge of the output in pixels (default: full size).
        #[arg(long)]
        edge: Option<usize>,
        /// Raw exposure correction, stops (default: automatic for raw files).
        #[arg(long, allow_hyphen_values = true)]
        exposure: Option<f64>,
        /// Defringe edges, 0–100 (raw files).
        #[arg(long, default_value_t = 0.0)]
        defringe: f64,
        /// Save the scene before the film (the app's "Original") instead.
        #[arg(long)]
        original: bool,
        #[arg(long)]
        out: PathBuf,
    },
    /// Render every built-in style small and lay them out on one contact sheet.
    Sheet {
        input: PathBuf,
        /// Only this process's styles (default: both).
        #[arg(long, value_enum)]
        process: Option<ProcessArg>,
        /// Long edge of each frame.
        #[arg(long, default_value_t = 480)]
        edge: usize,
        #[arg(long, allow_hyphen_values = true)]
        exposure: Option<f64>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Render a standard set of black-and-white film, developer and process variations of
    /// one image, with negatives and an HTML sheet of characteristic curves.
    Variations {
        /// Input image (PNG/JPEG/TIFF), or `chart` for the built-in synthetic test scene.
        #[arg(long)]
        input: String,
        /// Output directory.
        #[arg(long)]
        out: PathBuf,
        /// Width of film the image spans, in mm (36 = full 35 mm frame; smaller = enlarged crop).
        #[arg(long, default_value_t = 36.0)]
        frame_mm: f64,
        #[arg(long, value_enum, default_value_t = variations::Encoding::Auto)]
        encoding: variations::Encoding,
        /// Scene value treated as metered mid-grey.
        #[arg(long, default_value_t = 0.18)]
        grey: f32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
}

/// Open a raw or rendered file as a scene reduced to `edge` (if given).
fn load(
    path: &Path,
    edge: Option<usize>,
    exposure: Option<f64>,
    defringe: f64,
) -> Result<SceneImage, String> {
    if fd_raw::is_raw(path) {
        let photo = fd_raw::open(path).map_err(|e| e.to_string())?;
        let photo = match edge {
            Some(e) => photo.downscaled(e),
            None => photo,
        };
        let mut adjust = fd_raw::RawAdjust {
            defringe,
            ..Default::default()
        };
        adjust.exposure_ev = exposure.unwrap_or_else(|| photo.auto_exposure_ev(&adjust));
        Ok(photo.develop(&adjust))
    } else {
        let scene = fd_io::load_scene(path, fd_io::Encoding::Auto).map_err(|e| e.to_string())?;
        let scene = match edge {
            Some(e) => fd_io::downscale(&scene, e),
            None => scene,
        };
        Ok(match exposure {
            Some(ev) => fd_io::scale(&scene, 2f32.powf(ev as f32)),
            None => scene,
        })
    }
}

fn find_style(name: &str) -> Result<Style, String> {
    builtin_styles()
        .into_iter()
        .find(|s| s.name().eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("no built-in style named {name:?}; see `film styles`"))
}

/// The process a film id belongs to.
fn film_process(id: &str) -> Option<Process> {
    if fd_profiles::mono::film_ids().contains(&id) {
        Some(Process::Mono)
    } else if fd_profiles::colour::film_stocks()
        .iter()
        .any(|f| f.id == id)
    {
        Some(Process::Colour)
    } else {
        None
    }
}

fn save(output: &Output, out: &Path) -> Result<(), String> {
    let meta = fd_io::ExportMetadata {
        software: format!("Film Darkroom CLI {}", env!("CARGO_PKG_VERSION")),
        ..Default::default()
    };
    let ext = out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match (output, ext.as_str()) {
        (Output::Rgb(rgb), "jpg" | "jpeg") => fd_io::save_jpeg(rgb, out, 92, &meta),
        (Output::Rgb(rgb), "png") => fd_io::save_png16(rgb, out, &meta),
        (Output::Rgb(rgb), "tif" | "tiff") => fd_io::save_tiff16(rgb, out, &meta),
        (Output::Grey(p), "jpg" | "jpeg") => fd_io::save_grey_jpeg(p, out, 92, &meta),
        (Output::Grey(p), "png") => fd_io::save_grey_png16(p, out, &meta),
        (Output::Grey(p), "tif" | "tiff") => fd_io::save_grey_tiff16(p, out, &meta),
        _ => return Err("output must be .jpg, .png or .tif".into()),
    }
    .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
fn apply_overrides(
    mut style: Style,
    film: Option<String>,
    print: Option<String>,
    developer: Option<String>,
    grade: Option<f64>,
    push: Option<f64>,
    ev: Option<f64>,
    compression: Option<f64>,
    interlayer: Option<f64>,
) -> Result<Style, String> {
    let colour_only = |what: &str| format!("{what} applies to colour styles only");
    let mono_only = |what: &str| format!("{what} applies to black-and-white styles only");
    match &mut style {
        Style::Mono(s) => {
            if let Some(f) = film {
                s.film = f;
            }
            if let Some(d) = developer {
                s.developer = d;
            }
            if let Some(p) = print {
                s.print.paper = p;
            }
            if let Some(g) = grade {
                s.print.grade = g;
            }
            if let Some(ev) = ev {
                s.ev_compensation = ev;
            }
            if let Some(p) = push {
                s.set_push_stops(p);
            }
            if let Some(c) = compression {
                s.compression.impact = c;
            }
            if interlayer.is_some() {
                return Err(colour_only("--interlayer"));
            }
        }
        Style::Colour(s) => {
            if let Some(f) = film {
                s.film = f;
            }
            if let Some(p) = print {
                s.print = p;
            }
            if let Some(ev) = ev {
                s.ev_compensation = ev;
            }
            if let Some(p) = push {
                s.push = p;
            }
            if let Some(c) = compression {
                s.compression.impact = c;
            }
            if let Some(k) = interlayer {
                s.interlayer = k;
            }
            if developer.is_some() {
                return Err(mono_only("--developer"));
            }
            if grade.is_some() {
                return Err(mono_only("--grade"));
            }
        }
    }
    Ok(style)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Command::Styles => {
            for process in [Process::Colour, Process::Mono] {
                println!("{} styles:", process.label());
                for s in builtin_styles().iter().filter(|s| s.process() == process) {
                    println!("  {}", s.name());
                }
                println!();
            }
            println!("Colour film stocks (illustrative):");
            for f in fd_profiles::colour::film_stocks() {
                println!("  {:<16} {}", f.id, f.name);
            }
            println!("\nColour print stocks (illustrative):");
            for p in fd_profiles::colour::print_stocks() {
                println!("  {:<16} {}", p.id, p.name);
            }
            println!("\nBlack-and-white films:");
            for id in fd_profiles::mono::film_ids() {
                println!("  {:<16} {}", id, fd_profiles::mono::film_label(id));
            }
            println!("\nDevelopers:");
            for id in fd_profiles::mono::developer_ids() {
                println!("  {:<16} {}", id, fd_profiles::mono::developer_label(id));
            }
            println!("\nPapers:");
            for (id, name) in fd_profiles::mono::papers() {
                println!("  {id:<16} {name}");
            }
        }
        Command::Render {
            input,
            style,
            film,
            print,
            developer,
            grade,
            push,
            ev,
            compression,
            interlayer,
            edge,
            exposure,
            defringe,
            original,
            out,
        } => {
            let base = match (&style, film.as_deref().and_then(film_process)) {
                (Some(name), _) => find_style(name)?,
                (None, Some(process)) => builtin_styles()
                    .into_iter()
                    .find(|s| s.process() == process)
                    .expect("built-ins of each process"),
                (None, None) => find_style("Portra 400 · scan")?,
            };
            if let (Some(f), Some(p)) = (&film, film.as_deref().and_then(film_process))
                && p != base.process()
            {
                return Err(format!(
                    "film {f:?} is {} but style {:?} is {}",
                    p.label().to_lowercase(),
                    base.name(),
                    base.process().label().to_lowercase()
                )
                .into());
            }
            let s = apply_overrides(
                base,
                film,
                print,
                developer,
                grade,
                push,
                ev,
                compression,
                interlayer,
            )?;
            let scene = load(&input, edge, exposure, defringe)?;
            if original {
                return Ok(save(&Output::Rgb(scene.to_rgb()), &out)?);
            }
            let start = std::time::Instant::now();
            let r = fd_profiles::render(&scene, &s, s.frame_width_mm())?;
            eprintln!("rendered in {} ms", start.elapsed().as_millis());
            save(&r.into_output(), &out)?;
        }
        Command::Sheet {
            input,
            process,
            edge,
            exposure,
            out,
        } => {
            let scene = load(&input, Some(edge), exposure, 0.0)?;
            let process = process.map(Process::from);
            let styles: Vec<Style> = builtin_styles()
                .into_iter()
                .filter(|s| process.is_none_or(|p| s.process() == p))
                .collect();
            let (w, h) = scene.dimensions();
            let cols = 4;
            let rows = styles.len().div_ceil(cols);
            let gap = 8;
            let (sw, sh) = (cols * (w + gap) + gap, rows * (h + gap) + gap);
            let mut sheet: [Plane; 3] =
                std::array::from_fn(|_| Plane::filled(sw, sh, 0.05).expect("non-zero"));
            for (k, s) in styles.iter().enumerate() {
                let r = fd_profiles::render(&scene, s, s.frame_width_mm())
                    .map_err(|e| format!("{}: {e}", s.name()))?;
                let image = match r.into_output() {
                    Output::Rgb(rgb) => rgb,
                    Output::Grey(p) => [p.clone(), p.clone(), p],
                };
                let (ox, oy) = (gap + (k % cols) * (w + gap), gap + (k / cols) * (h + gap));
                for (plane, image) in sheet.iter_mut().zip(&image) {
                    for y in 0..h {
                        for x in 0..w {
                            plane.set(ox + x, oy + y, image.get(x, y));
                        }
                    }
                }
                eprintln!("{:>2}. {}", k + 1, s.name());
            }
            save(&Output::Rgb(sheet), &out)?;
        }
        Command::Variations {
            input,
            out,
            frame_mm,
            encoding,
            grey,
            seed,
        } => variations::run(input, out, frame_mm, encoding, grey, seed)?,
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
