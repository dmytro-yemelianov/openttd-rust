//! # Transport Simulation Agnostic Presentation & Rendering Layer
//!
//! This crate provides frontend-agnostic rendering primitives, isometric camera
//! transformations, sub-tick visual interpolation, and Painter's depth sorting.
//!
//! The simulation core remains 100% headless and decoupled from windowing or GPU frameworks.
//! Any visual client (Terminal ANSI, WebGL/Canvas, WGPU, Godot, etc.) implements
//! the [`RenderBackend`] trait to display the simulation.

pub mod backend;
pub mod camera;
pub mod depth;
pub mod interpolator;
pub mod scene;
pub mod software;

pub use backend::*;
pub use camera::*;
pub use depth::*;
pub use interpolator::*;
pub use scene::*;
pub use software::*;
