//! An opened image: a rendered raster file or a camera raw, plus its per-image
//! adjustments stored in a sidecar next to it (`<file>.fd.json`), so edits are
//! non-destructive and travel with the photo.

use std::path::{Path, PathBuf};

use fd_core::SceneImage;
use fd_raw::{CaptureInfo, RawAdjust, RawPhoto};
use serde::{Deserialize, Serialize};

pub enum Source {
    Raster(SceneImage),
    Raw(Box<RawPhoto>),
}

impl Source {
    pub fn open(path: &Path) -> Result<Source, String> {
        if fd_raw::is_raw(path) {
            fd_raw::open(path)
                .map(|p| Source::Raw(Box::new(p)))
                .map_err(|e| e.to_string())
        } else {
            fd_io::load_scene(path, fd_io::Encoding::Auto)
                .map(Source::Raster)
                .map_err(|e| e.to_string())
        }
    }

    /// The scene after per-image adjustments, at full resolution.
    pub fn develop(&self, adjust: &RawAdjust) -> SceneImage {
        match self {
            Source::Raster(scene) => fd_raw::adjust_rendered(scene, adjust),
            Source::Raw(photo) => photo.develop(adjust),
        }
    }

    /// A reduced copy (long edge ≤ `edge`) to develop previews from quickly.
    pub fn downscaled(&self, edge: usize) -> Source {
        match self {
            Source::Raster(scene) => Source::Raster(fd_io::downscale(scene, edge)),
            Source::Raw(photo) => Source::Raw(Box::new(photo.downscaled(edge))),
        }
    }

    pub fn capture(&self) -> Option<&CaptureInfo> {
        match self {
            Source::Raw(photo) => Some(&photo.info),
            Source::Raster(_) => None,
        }
    }

    pub fn raw(&self) -> Option<&RawPhoto> {
        match self {
            Source::Raw(photo) => Some(photo),
            Source::Raster(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Sidecar {
    version: u32,
    adjust: RawAdjust,
}

/// Sidecars the single-process apps write: read when a photo has no `.fd.json` yet, so
/// their adjustments carry over, but never written or removed.
const OTHER_SIDECARS: [&str; 2] = [".bw.json", ".cd.json"];

fn with_suffix(image: &Path, suffix: &str) -> PathBuf {
    let mut name = image.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    image.with_file_name(name)
}

pub fn sidecar_path(image: &Path) -> PathBuf {
    with_suffix(image, ".fd.json")
}

fn read_sidecar(path: &Path) -> Option<RawAdjust> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<Sidecar>(&s).ok())
        .map(|s| s.adjust)
}

/// The single-process apps' sidecars for `image` that exist, newest first.
fn other_sidecars(image: &Path) -> Vec<PathBuf> {
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = OTHER_SIDECARS
        .iter()
        .map(|suffix| with_suffix(image, suffix))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    found.sort_by_key(|a| std::cmp::Reverse(a.0));
    found.into_iter().map(|(_, p)| p).collect()
}

/// The image's saved adjustments: its own sidecar, else the newest BW Darkroom or Colour
/// Darkroom sidecar, else the defaults.
pub fn load_adjust(image: &Path) -> RawAdjust {
    read_sidecar(&sidecar_path(image))
        .or_else(|| other_sidecars(image).iter().find_map(|p| read_sidecar(p)))
        .unwrap_or_default()
}

/// Save adjustments. Default adjustments remove the sidecar instead of writing one,
/// unless an older app's sidecar would then be read in its place.
pub fn save_adjust(image: &Path, adjust: &RawAdjust) -> std::io::Result<()> {
    let path = sidecar_path(image);
    if *adjust == RawAdjust::default() && other_sidecars(image).is_empty() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let json = serde_json::to_string_pretty(&Sidecar {
        version: 1,
        adjust: *adjust,
    })?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_round_trip_and_cleanup() {
        let dir = std::env::temp_dir().join(format!("fd-sidecar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let image = dir.join("DSC_0001.NEF");
        assert_eq!(load_adjust(&image), RawAdjust::default());
        let adjust = RawAdjust {
            exposure_ev: 0.5,
            rotation: 1,
            ..Default::default()
        };
        save_adjust(&image, &adjust).unwrap();
        assert!(dir.join("DSC_0001.NEF.fd.json").exists());
        assert_eq!(load_adjust(&image), adjust);
        save_adjust(&image, &RawAdjust::default()).unwrap();
        assert!(!dir.join("DSC_0001.NEF.fd.json").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_but_never_touches_the_single_process_sidecars() {
        let dir = std::env::temp_dir().join(format!("fd-old-sidecar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let image = dir.join("DSC_0002.NEF");
        let bw = serde_json::to_string(&Sidecar {
            version: 1,
            adjust: RawAdjust {
                exposure_ev: 0.7,
                rotation: 2,
                ..Default::default()
            },
        })
        .unwrap();
        let old = dir.join("DSC_0002.NEF.bw.json");
        std::fs::write(&old, &bw).unwrap();
        let loaded = load_adjust(&image);
        assert_eq!((loaded.exposure_ev, loaded.rotation), (0.7, 2));
        // Resetting writes our own defaults so the old sidecar is not read again, and
        // leaves the old one exactly as it was.
        save_adjust(&image, &RawAdjust::default()).unwrap();
        assert_eq!(load_adjust(&image), RawAdjust::default());
        assert_eq!(std::fs::read_to_string(&old).unwrap(), bw);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
