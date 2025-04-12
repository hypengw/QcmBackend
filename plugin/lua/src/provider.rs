use mlua::prelude::*;
use qcm_core::model as sqlm;
use qcm_core::{
    anyhow,
    error::ConnectError,
    event::Event as CoreEvent,
    http::{CookieStoreRwLock, HasCookieJar, HeaderMap, HttpClient},
    model::type_enum::ImageType,
    provider::{AuthInfo, Context, Provider},
    Result,
};
use sea_orm::*;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

struct LuaProviderInner {
    client: qcm_core::http::HttpClient,
    id: Option<i64>,
    name: String,
    lua: Lua,
    provider_table: Option<LuaTable>,
}

pub struct LuaProvider {
    jar: Arc<CookieStoreRwLock>,
    inner: RwLock<LuaProviderInner>,
}

impl LuaProvider {
    pub fn new(id: Option<i64>, name: &str, script_path: &Path) -> Result<Self> {
        let jar = Arc::new(CookieStoreRwLock::default());

        // Create new Lua instance
        let lua = Lua::new();

        // setup_http_client(&lua)?;

        // Load the provider script
        lua.load(script_path).exec()?;

        // Get provider table
        // let provider_table = provider
        //     .inner
        //     .read()
        //     .unwrap()
        //     .lua
        //     .globals()
        //     .get("provider")?;
        // provider.inner.write().unwrap().provider_table = Some(provider_table);

        // Create provider instance
        let provider = Self {
            jar: jar.clone(),
            inner: RwLock::new(LuaProviderInner {
                client: qcm_core::http::client_builder_with_jar(jar.clone())
                    .build()
                    .unwrap(),
                id,
                name: name.to_string(),
                lua,
                provider_table: None,
            }),
        };
        Ok(provider)
    }

    pub fn type_name() -> &'static str {
        "lua"
    }

    pub fn client(&self) -> HttpClient {
        return self.inner.read().unwrap().client.clone();
    }
}

impl HasCookieJar for LuaProvider {
    fn jar(&self) -> Arc<CookieStoreRwLock> {
        self.jar.clone()
    }
}

#[async_trait::async_trait]
impl Provider for LuaProvider {
    fn id(&self) -> Option<i64> {
        self.inner.read().unwrap().id
    }

    fn set_id(&self, id: Option<i64>) {
        let mut inner = self.inner.write().unwrap();
        inner.id = id;
    }

    fn name(&self) -> String {
        self.inner.read().unwrap().name.clone()
    }

    fn type_name(&self) -> &str {
        LuaProvider::type_name()
    }

    fn from_model(&self, model: &sqlm::provider::Model) -> Result<()> {
        let inner = self.inner.write().unwrap();
        if let Some(provider_table) = &inner.provider_table {
            // Call Lua from_model function if it exists
            if let Ok(from_model) = provider_table.get::<_, LuaFunction>("from_model") {
                from_model.call::<_, ()>(model.custom.clone())?;
            }
        }
        Ok(())
    }

    fn to_model(&self) -> sqlm::provider::ActiveModel {
        let inner = self.inner.read().unwrap();
        let custom = if let Some(provider_table) = &inner.provider_table {
            if let Ok(to_model) = provider_table.get::<_, LuaFunction>("to_model") {
                to_model.call::<_, String>(()).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        sqlm::provider::ActiveModel {
            provider_id: match self.id() {
                Some(id) => Set(id),
                None => NotSet,
            },
            name: Set(self.name()),
            type_: Set(self.type_name().to_string()),
            base_url: Set(String::new()),
            cookie: Set(String::new()),
            custom: Set(custom),
            edit_time: Set(chrono::Local::now().naive_local()),
        }
    }

    async fn login(&self, ctx: &Context, info: &AuthInfo) -> Result<()> {
        let inner = self.inner.read().unwrap();
        if let Some(provider_table) = &inner.provider_table {
            if let Ok(login) = provider_table.get::<_, LuaFunction>("login") {
                // Convert AuthInfo to Lua table
                let auth_table = inner.lua.create_table()?;
                auth_table.set("server_url", info.server_url.clone())?;
                // Add other auth info fields as needed

                login.call::<_, ()>(auth_table)?;
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("Login not implemented"))
    }

    async fn sync(&self, ctx: &Context) -> Result<()> {
        let inner = self.inner.read().unwrap();
        if let Some(provider_table) = &inner.provider_table {
            if let Ok(sync) = provider_table.get::<_, LuaFunction>("sync") {
                sync.call::<_, ()>(())?;
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("Sync not implemented"))
    }

    async fn image(
        &self,
        ctx: &Context,
        item_id: &str,
        image_type: ImageType,
    ) -> Result<Response, ConnectError> {
        let inner = self.inner.read().unwrap();
        if let Some(provider_table) = &inner.provider_table {
            if let Ok(get_image) = provider_table.get::<_, LuaFunction>("get_image") {
                // Convert the response from Lua to reqwest Response
                // This is a placeholder - you'll need to implement proper conversion
                let response = get_image.call::<_, LuaTable>((item_id, image_type.to_string()))?;
                // Convert Lua response to reqwest Response
                unimplemented!("Need to implement response conversion");
            }
        }
        Err(ConnectError::NotImplemented)
    }

    async fn audio(
        &self,
        ctx: &Context,
        item_id: &str,
        headers: Option<HeaderMap>,
    ) -> Result<Response, ConnectError> {
        let inner = self.inner.read().unwrap();
        if let Some(provider_table) = &inner.provider_table {
            if let Ok(get_audio) = provider_table.get::<_, LuaFunction>("get_audio") {
                // Convert the response from Lua to reqwest Response
                // This is a placeholder - you'll need to implement proper conversion
                let response = get_audio.call::<_, LuaTable>((item_id,))?;
                // Convert Lua response to reqwest Response
                unimplemented!("Need to implement response conversion");
            }
        }
        Err(ConnectError::NotImplemented)
    }
}
