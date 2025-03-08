pub struct PluginContext {

}

pub trait Plugin {
    fn name(&self) -> &str;
}
