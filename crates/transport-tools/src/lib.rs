//! # Transport Simulation Interaction & Input Tools Layer
//!
//! This crate provides frontend-agnostic interaction tools, cursor tracking,
//! dry-run placement validation, station catchment calculations, ghost overlays,
//! and event-to-command translation.

pub mod input;
pub mod preview;
pub mod tool;
pub mod validator;

pub use input::*;
pub use preview::*;
pub use tool::*;
pub use validator::*;
