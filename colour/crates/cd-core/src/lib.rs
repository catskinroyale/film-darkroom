//! Numerical foundations for the colour film emulation engine.
//!
//! This crate is deliberately free of photographic policy and I/O. It provides the
//! primitives the film, print and dye models are built from:
//!
//! - [`Plane`]: planar `f32` image storage; [`SceneImage`]: scene-linear RGB.
//! - [`FilmGeometry`]: the mapping between pixels and film-plane micrometres, so that
//!   grain, irradiation and adjacency are specified in physical units (BW Darkroom design 0001 §2.1).
//! - [`MonotoneCurve`] and [`Lut1D`]: characteristic-curve representation (BW Darkroom design 0001 §3.2).
//! - [`noise`]: counter-based deterministic Gaussian noise (BW Darkroom design 0001 §5.3, D8).
//! - [`blur`]: separable Gaussian filtering; [`blur::gaussian_blur_wide`] for halation and bloom.

pub mod blur;
pub mod curve;
pub mod geometry;
pub mod noise;
pub mod plane;
pub mod provenance;
pub mod scene;

pub use curve::{CurveError, Lut1D, MonotoneCurve};
pub use geometry::{FilmGeometry, GeometryError};
pub use plane::{Plane, PlaneError};
pub use provenance::{CalibrationLevel, Provenance};
pub use scene::SceneImage;
