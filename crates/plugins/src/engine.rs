//! Wasmtime Engine Wrapper
//!
//! Manages the Wasmtime runtime and module compilation.

use crate::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};
use wasmtime::{Config, Engine, Module, Store};

/// Configuration for the Wasm engine
#[derive(Debug, Clone)]
pub struct WasmEngineConfig {
    /// Enable module caching
    pub cache_modules: bool,
    /// Maximum memory per instance (bytes)
    pub max_memory_bytes: usize,
    /// Enable fuel metering for CPU limits
    pub enable_fuel: bool,
    /// Initial fuel amount
    pub initial_fuel: u64,
}

impl Default for WasmEngineConfig {
    fn default() -> Self {
        Self {
            cache_modules: true,
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            enable_fuel: true,
            initial_fuel: 1_000_000,
        }
    }
}

/// Wasmtime engine wrapper with module caching
pub struct WasmEngine {
    /// Wasmtime engine
    engine: Engine,
    /// Module cache
    module_cache: Arc<RwLock<HashMap<String, Module>>>,
    /// Configuration
    config: WasmEngineConfig,
}

impl WasmEngine {
    /// Create a new Wasm engine with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(WasmEngineConfig::default())
    }

    /// Create a new Wasm engine with custom configuration
    pub fn with_config(config: WasmEngineConfig) -> Result<Self> {
        let mut wasmtime_config = Config::new();

        // Enable fuel metering if configured
        if config.enable_fuel {
            wasmtime_config.consume_fuel(true);
        }

        // Optimize for performance
        wasmtime_config.cranelift_opt_level(wasmtime::OptLevel::Speed);

        let engine = Engine::new(&wasmtime_config)?;

        info!("🔌 Wasm engine initialized");

        Ok(Self {
            engine,
            module_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Get the underlying Wasmtime engine
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Compile a Wasm module from bytes
    #[allow(clippy::collapsible_if)]
    pub fn compile_module(&self, name: &str, wasm_bytes: &[u8]) -> Result<Module> {
        // Check cache first
        if self.config.cache_modules {
            if let Ok(cache) = self.module_cache.read() {
                if let Some(module) = cache.get(name) {
                    debug!("Cache hit for module: {}", name);
                    return Ok(module.clone());
                }
            }
        }

        // Compile the module
        let module = Module::new(&self.engine, wasm_bytes)?;

        // Cache the module
        if self.config.cache_modules {
            if let Ok(mut cache) = self.module_cache.write() {
                cache.insert(name.to_string(), module.clone());
                debug!("Cached module: {}", name);
            }
        }

        info!("✅ Compiled Wasm module: {}", name);
        Ok(module)
    }

    /// Load and compile a Wasm module from file
    pub fn load_module(&self, path: &Path) -> Result<Module> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let wasm_bytes = std::fs::read(path)?;
        self.compile_module(name, &wasm_bytes)
    }

    /// Create a new store for module execution
    pub fn create_store<T: Default>(&self) -> Store<T> {
        let mut store = Store::new(&self.engine, T::default());

        // Set fuel if enabled
        if self.config.enable_fuel {
            let _ = store.set_fuel(self.config.initial_fuel);
        }

        store
    }

    /// Clear the module cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.module_cache.write() {
            cache.clear();
            info!("🗑️ Module cache cleared");
        }
    }

    /// Get the number of cached modules
    pub fn cache_size(&self) -> usize {
        self.module_cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Get configuration
    pub fn config(&self) -> &WasmEngineConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = WasmEngine::new().unwrap();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_engine_with_config() {
        let config = WasmEngineConfig {
            cache_modules: false,
            max_memory_bytes: 32 * 1024 * 1024,
            enable_fuel: false,
            initial_fuel: 0,
        };

        let engine = WasmEngine::with_config(config).unwrap();
        assert!(!engine.config().cache_modules);
    }

    #[test]
    fn test_compile_minimal_module() {
        let engine = WasmEngine::new().unwrap();

        // Minimal valid Wasm module (empty)
        let wasm_bytes = wat::parse_str("(module)").unwrap();

        let module = engine.compile_module("test", &wasm_bytes).unwrap();
        assert!(module.exports().count() == 0);
    }

    #[test]
    fn test_module_caching() {
        let engine = WasmEngine::new().unwrap();
        let wasm_bytes = wat::parse_str("(module)").unwrap();

        // First compilation
        let _ = engine.compile_module("cached", &wasm_bytes).unwrap();
        assert_eq!(engine.cache_size(), 1);

        // Second compilation should use cache
        let _ = engine.compile_module("cached", &wasm_bytes).unwrap();
        assert_eq!(engine.cache_size(), 1);

        // Clear cache
        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }
}
