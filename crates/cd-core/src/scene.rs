//! Scene-linear images: what the camera (or a rendered file) hands the film.

use crate::plane::Plane;

/// A scene-referred image.
#[derive(Clone, Debug)]
pub enum SceneImage {
    /// White-balanced, scene-linear Rec.709/sRGB primaries.
    Rgb { r: Plane, g: Plane, b: Plane },
    /// Native monochrome sensor response (e.g. a Leica Monochrom). The colour film
    /// sees it as a neutral scene.
    Mono { y: Plane },
}

impl SceneImage {
    pub fn dimensions(&self) -> (usize, usize) {
        let p = match self {
            Self::Rgb { r, .. } => r,
            Self::Mono { y } => y,
        };
        (p.width(), p.height())
    }

    /// The three channels, copying a monochrome plane into all three.
    pub fn to_rgb(&self) -> [Plane; 3] {
        match self {
            Self::Rgb { r, g, b } => [r.clone(), g.clone(), b.clone()],
            Self::Mono { y } => [y.clone(), y.clone(), y.clone()],
        }
    }
}
