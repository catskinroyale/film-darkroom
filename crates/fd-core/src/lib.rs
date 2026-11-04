//! Numerical foundations for the film emulation engines (colour and black-and-white).
//!
//! This crate is deliberately free of photographic policy and I/O. It provides the
//! primitives the film, developer, print and dye models are built from:
//!
//! - [`Plane`]: planar `f32` image storage; [`SceneImage`]: scene-linear RGB.
//! - [`FilmGeometry`]: the mapping between pixels and film-plane micrometres, so that
//!   grain, irradiation and adjacency are specified in physical units (mono design 0001 §2.1).
//! - [`MonotoneCurve`] and [`Lut1D`]: characteristic-curve representation (mono design 0001 §3.2).
//! - [`noise`]: counter-based deterministic Gaussian noise (mono design 0001 §5.3, D8).
//! - [`blur`]: separable Gaussian filtering; [`blur::gaussian_blur_wide`] for halation and bloom.

pub mod blur;
pub mod compression;
pub mod curve;
pub mod geometry;
pub mod noise;
pub mod plane;
pub mod provenance;
pub mod scene;

pub use compression::Compression;
pub use curve::{CurveError, Lut1D, MonotoneCurve};
pub use geometry::{FilmGeometry, GeometryError};
pub use plane::{Plane, PlaneError};
pub use provenance::{CalibrationLevel, Provenance};
pub use scene::SceneImage;
