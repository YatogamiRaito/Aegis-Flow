//! Aegis-Plugins: WebAssembly Plugin System
//!
//! Provides a sandboxed plugin system using Wasmtime for extensible request processing.
//!
//! # Features
//! - Wasmtime-based WebAssembly runtime
//! - Sandboxed plugin execution
//! - Plugin registry with hot reload
//!
//! # Example
//! ```rust,ignore
//! use aegis_plugins::{WasmEngine, PluginRegistry};
//!
//! let engine = WasmEngine::new()?;
//! let registry = PluginRegistry::new(engine);
//! registry.load_plugin("my_plugin.wasm")?;
//! ```

pub mod engine;
pub mod interface;
pub mod registry;

pub use engine::WasmEngine;
pub use interface::{PluginRequest, PluginResponse, PluginResult};
pub use registry::{PluginInfo, PluginRegistry};

/// Error types for plugin operations
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Wasmtime error: {0}")]
    WasmtimeError(#[from] wasmtime::Error),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin execution error: {0}")]
    ExecutionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, PluginError>;
