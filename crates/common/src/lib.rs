//! Aegis-Common: Common utilities and types for Aegis-Flow
//!
//! This crate provides shared types, error handling, and utility functions
//! used across the Aegis-Flow project.

pub mod error;
pub mod types;

pub use error::{AegisError, Result};
