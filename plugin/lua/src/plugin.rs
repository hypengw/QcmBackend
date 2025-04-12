use super::provider::LuaProvider;
use qcm_core::plugin::Plugin;
use qcm_core::provider::{Creator, Provider, ProviderMeta};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Deserialize, Serialize)]
struct PluginJson {
    name: String,
    type_name: String,
    script_path: PathBuf,
    svg_path: PathBuf,
}

pub struct LuaPlugin {
    plugins_dir: Vec<String>,
}

impl LuaPlugin {
    pub fn new() -> Self {
        let mut plugin_dirs = Vec::new();

        if let Ok(exec_path) = env::current_exe() {
            if let Some(exec_dir) = exec_path.parent() {
                plugin_dirs.push(exec_dir.join("plugins").to_string_lossy().to_string());
            }
        }

        #[cfg(target_os = "linux")]
        plugin_dirs.push("/usr/share/QcmPlugins".to_string());

        Self {
            plugins_dir: plugin_dirs,
        }
    }

    fn load_plugin_json(&self, path: &Path) -> Option<PluginJson> {
        if let Ok(content) = fs::read_to_string(path.join("plugin.json")) {
            if let Ok(mut plugin_json) = serde_json::from_str::<PluginJson>(&content) {
                // Read the SVG file
                if let Ok(svg_content) = fs::read_to_string(path.join(&plugin_json.svg_path)) {
                    return Some(plugin_json);
                }
            }
        }
        None
    }

    fn create_provider(&self, plugin_json: PluginJson, svg_content: String) -> ProviderMeta {
        let script_path = plugin_json.script_path.clone();
        let creator: Arc<Creator> =
            Arc::new(move |id, name, device_id| -> Result<Arc<dyn Provider>> {
                LuaProvider::new(id, name, device_id, &script_path).map(|p| Arc::new(p))
            });

        ProviderMeta::new(&plugin_json.type_name, Arc::new(svg_content), creator)
    }
}

impl Plugin for LuaPlugin {
    fn id(&self) -> &str {
        return "qcm.plugin.lua";
    }
    fn name(&self) -> &str {
        return "lua";
    }
    fn provider_metas(&self) -> Vec<ProviderMeta> {
        let mut metas = Vec::new();

        for plugins_dir in &self.plugins_dir {
            if let Ok(entries) = fs::read_dir(plugins_dir) {
                for entry in entries.flatten() {
                    if let Some(plugin_json) = self.load_plugin_json(&entry.path()) {
                        if let Ok(svg_content) =
                            fs::read_to_string(entry.path().join(&plugin_json.svg_path))
                        {
                            metas.push(self.create_provider(plugin_json, svg_content));
                        }
                    }
                }
            }
        }

        metas
    }
}
