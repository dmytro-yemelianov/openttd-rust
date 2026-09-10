//! # Transport simulation primitive types and IDs
//!
//! This crate provides the foundational types used across the transport simulation:
//! - Strongly-typed IDs for entities
//! - Numeric units with compile-time safety
//! - Primitive enums for domain concepts
//! - Error types

pub mod capability;
pub mod enum_;
pub mod error;
pub mod id;
pub mod service;
pub mod unit;

pub use capability::*;
pub use enum_::*;
pub use error::*;
pub use id::*;
pub use service::*;
pub use unit::*;
