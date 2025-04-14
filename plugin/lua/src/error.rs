use qcm_core::error::ProviderError;

pub trait FromLuaError<F> {
    fn from_err(err: F) -> ProviderError;
}

impl FromLuaError<mlua::Error> for ProviderError {
    fn from_err(err: mlua::Error) -> ProviderError {
        ProviderError::Lua(err.to_string())
    }
}
