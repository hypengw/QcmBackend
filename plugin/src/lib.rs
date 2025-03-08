use qcm_core::{global as qg, plugin::Plugin};

pub fn init() {
    {
        use qcm_plugin_jellyfin::plugin::JellyfinPlugin;
        qg::add_plugin(Box::new(JellyfinPlugin::new()));
    }
}
