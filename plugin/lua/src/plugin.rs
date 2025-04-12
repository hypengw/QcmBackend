use qcm_core::plugin::Plugin;
use qcm_core::provider::{Provider, ProviderMeta};
use std::sync::Arc;

pub struct LuaPlugin {}

impl LuaPlugin {
    pub fn new() -> Self {
        Self {}
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
        Vec::new()
    }
}
