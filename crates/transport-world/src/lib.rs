//! # Transport world simulation core
//!
//! This crate contains the core world simulation:
//! - Map storage and tile accessors
//! - Entity pools for companies, vehicles, stations, etc.
//! - Definitions and specifications
//! - Relationship tracking

pub mod map;
pub mod entities;
pub mod definitions;
pub mod relationships;

pub use map::*;
pub use entities::*;
pub use definitions::*;