//! # Transport world simulation core
//!
//! This crate contains the core world simulation:
//! - Map storage and tile accessors
//! - Entity pools for companies, vehicles, stations, etc.
//! - Definitions and specifications
//! - Relationship tracking

pub mod definitions;
pub mod entities;
pub mod map;
pub mod relationships;
pub mod town;

pub use definitions::*;
pub use entities::*;
pub use map::*;
pub use town::*;
