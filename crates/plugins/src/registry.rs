//! Plugin Registry
//!
//! Manages plugin loading, lifecycle, and execution.

use crate::engine::WasmEngine;
use crate::{PluginError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{info, warn};
use wasmtime::Module;

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Plugin path
    pub path: PathBuf,
    /// Whether plugin is enabled
    pub enabled: bool,
    /// Load timestamp
    pub loaded_at: std::time::SystemTime,
}

/// Plugin registry for managing loaded plugins
pub struct PluginRegistry {
    /// Wasm engine
    engine: Arc<WasmEngine>,
    /// Loaded plugins
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    /// Plugin directory
    plugin_dir: Option<PathBuf>,
}

/// A loaded and ready-to-execute plugin
struct LoadedPlugin {
    /// Plugin info
    info: PluginInfo,
    /// Compiled module
    #[allow(dead_code)]
    module: Module,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(engine: WasmEngine) -> Self {
        Self {
            engine: Arc::new(engine),
            plugins: RwLock::new(HashMap::new()),
            plugin_dir: None,
        }
    }

    /// Set the plugin directory for auto-loading
    pub fn with_plugin_dir(mut self, dir: PathBuf) -> Self {
        self.plugin_dir = Some(dir);
        self
    }

    /// Load a plugin from bytes
    pub fn load_plugin_bytes(&self, name: &str, wasm_bytes: &[u8]) -> Result<()> {
        let module = self.engine.compile_module(name, wasm_bytes)?;

        let info = PluginInfo {
            name: name.to_string(),
            path: PathBuf::new(),
            enabled: true,
            loaded_at: std::time::SystemTime::now(),
        };

        let loaded = LoadedPlugin { info, module };

        if let Ok(mut plugins) = self.plugins.write() {
            plugins.insert(name.to_string(), loaded);
            info!("📦 Loaded plugin: {}", name);
        }

        Ok(())
    }

    /// Load a plugin from file
    #[allow(clippy::collapsible_if)]
    pub fn load_plugin(&self, path: &Path) -> Result<()> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PluginError::NotFound("Invalid plugin path".to_string()))?;

        let wasm_bytes = std::fs::read(path)?;
        self.load_plugin_bytes(name, &wasm_bytes)?;

        // Update path in plugin info
        if let Ok(mut plugins) = self.plugins.write() {
            if let Some(plugin) = plugins.get_mut(name) {
                plugin.info.path = path.to_path_buf();
            }
        }

        Ok(())
    }

    /// Unload a plugin
    pub fn unload_plugin(&self, name: &str) -> Result<()> {
        if let Ok(mut plugins) = self.plugins.write() {
            if plugins.remove(name).is_some() {
                info!("🗑️ Unloaded plugin: {}", name);
                Ok(())
            } else {
                Err(PluginError::NotFound(name.to_string()))
            }
        } else {
            Err(PluginError::ExecutionError("Lock error".to_string()))
        }
    }

    /// Reload a plugin
    pub fn reload_plugin(&self, name: &str) -> Result<()> {
        let path = {
            let plugins = self
                .plugins
                .read()
                .map_err(|_| PluginError::ExecutionError("Lock error".to_string()))?;

            plugins
                .get(name)
                .map(|p| p.info.path.clone())
                .ok_or_else(|| PluginError::NotFound(name.to_string()))?
        };

        if path.exists() {
            // Clear from engine cache
            self.engine.clear_cache();

            // Reload
            self.load_plugin(&path)?;
            info!("🔄 Reloaded plugin: {}", name);
        }

        Ok(())
    }

    /// Get list of loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .read()
            .map(|p| p.values().map(|lp| lp.info.clone()).collect())
            .unwrap_or_default()
    }

    /// Check if a plugin is loaded
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins
            .read()
            .map(|p| p.contains_key(name))
            .unwrap_or(false)
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.plugins.read().map(|p| p.len()).unwrap_or(0)
    }

    /// Load all plugins from the plugin directory
    pub fn load_all_plugins(&self) -> Result<usize> {
        let dir = self
            .plugin_dir
            .as_ref()
            .ok_or_else(|| PluginError::NotFound("Plugin directory not set".to_string()))?;

        let mut count = 0;

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                    match self.load_plugin(&path) {
                        Ok(()) => count += 1,
                        Err(e) => warn!("Failed to load plugin {:?}: {}", path, e),
                    }
                }
            }
        }

        info!("📦 Loaded {} plugins from {:?}", count, dir);
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> PluginRegistry {
        let engine = WasmEngine::new().unwrap();
        PluginRegistry::new(engine)
    }

    #[test]
    fn test_registry_creation() {
        let registry = create_test_registry();
        assert_eq!(registry.plugin_count(), 0);
    }

    #[test]
    fn test_load_plugin_bytes() {
        let registry = create_test_registry();
        let wasm_bytes = wat::parse_str("(module)").unwrap();

        registry.load_plugin_bytes("test", &wasm_bytes).unwrap();

        assert!(registry.has_plugin("test"));
        assert_eq!(registry.plugin_count(), 1);
    }

    #[test]
    fn test_unload_plugin() {
        let registry = create_test_registry();
        let wasm_bytes = wat::parse_str("(module)").unwrap();

        registry.load_plugin_bytes("test", &wasm_bytes).unwrap();
        assert!(registry.has_plugin("test"));

        registry.unload_plugin("test").unwrap();
        assert!(!registry.has_plugin("test"));
    }

    #[test]
    fn test_list_plugins() {
        let registry = create_test_registry();
        let wasm_bytes = wat::parse_str("(module)").unwrap();

        registry.load_plugin_bytes("plugin1", &wasm_bytes).unwrap();
        registry.load_plugin_bytes("plugin2", &wasm_bytes).unwrap();

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 2);
    }
}
