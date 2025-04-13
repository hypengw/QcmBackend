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
use reqwest::Response;
use sea_orm::*;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::crypto::create_crypto_module;
use crate::http::LuaClient;

struct LuaProviderInner {
    client: qcm_core::http::HttpClient,
    id: Option<i64>,
    name: String,
    device_id: String,
}

struct LuaInner(Arc<RwLock<LuaProviderInner>>);

impl LuaUserData for LuaInner {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("device_id", |_, this, _: ()| {
            Ok(this.0.read().unwrap().device_id.clone())
        });
    }
}

struct LuaImpl {
    login: LuaFunction,
    sync: LuaFunction,
    image: LuaFunction,
    audio: LuaFunction,
}

pub struct LuaProvider {
    jar: Arc<CookieStoreRwLock>,
    inner: Arc<RwLock<LuaProviderInner>>,
    funcs: LuaImpl,
    lua: Lua,
}

impl LuaProvider {
    pub fn new(id: Option<i64>, name: &str, device_id: &str, script_path: &Path) -> Result<Self> {
        let jar = Arc::new(CookieStoreRwLock::default());

        let lua = Lua::new();

        let client = qcm_core::http::client_builder_with_jar(jar.clone())
            .build()
            .unwrap();
        let inner = Arc::new(RwLock::new(LuaProviderInner {
            client: client.clone(),
            id,
            name: name.to_string(),
            device_id: device_id.to_string(),
        }));
        {
            // set package.path
            let package = lua.globals().get::<LuaTable>("package")?;
            package.set(
                "path",
                script_path
                    .parent()
                    .and_then(|p| p.to_str())
                    .map(|p| format!("{}/?.lua", p)),
            )?;

            // qcm table
            let qcm_table = lua.create_table()?;
            qcm_table.set("inner", LuaInner(inner.clone()))?;
            qcm_table.set("http", LuaClient(client))?;
            qcm_table.set("ssl", create_crypto_module(&lua)?)?;
            qcm_table.set(
                "json_encode",
                lua.create_function(|_, v: mlua::Value| {
                    serde_json::to_string(&v).map_err(|e| mlua::Error::external(e))
                })?,
            )?;
            qcm_table.set(
                "json_decode",
                lua.create_function(|lua, str: String| {
                    let v: serde_json::Value =
                        serde_json::from_str(&str).map_err(|e| mlua::Error::external(e))?;
                    lua.to_value(&v)
                })?,
            )?;
            lua.globals().set("qcm", qcm_table)?;
        }

        // Load the provider script
        let func = lua.load(script_path).into_function()?;
        let provider_table = func.call::<LuaTable>(())?;

        let provider = Self {
            jar: jar.clone(),
            inner,
            funcs: LuaImpl {
                login: provider_table
                    .get::<LuaFunction>("login")
                    .map_err(|_| anyhow!("login func not found"))?,
                sync: provider_table
                    .get::<LuaFunction>("sync")
                    .map_err(|_| anyhow!("sync func not found"))?,
                image: provider_table
                    .get::<LuaFunction>("image")
                    .map_err(|_| anyhow!("image func not found"))?,
                audio: provider_table
                    .get::<LuaFunction>("audio")
                    .map_err(|_| anyhow!("audio func not found"))?,
            },
            lua,
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
        // let inner = self.inner.write().unwrap();
        // if let Some(provider_table) = &inner.provider_table {
        //     // Call Lua from_model function if it exists
        //     if let Ok(from_model) = provider_table.get::<LuaFunction>("from_model") {
        //         from_model.call::<()>(model.custom.clone())?;
        //     }
        // }
        Ok(())
    }

    fn to_model(&self) -> sqlm::provider::ActiveModel {
        // let inner = self.inner.read().unwrap();
        // let custom = if let Some(provider_table) = &inner.provider_table {
        //     if let Ok(to_model) = provider_table.get::<LuaFunction>("to_model") {
        //         to_model.call::<String>(()).unwrap_or_default()
        //     } else {
        //         String::new()
        //     }
        // } else {
        //     String::new()
        // };

        sqlm::provider::ActiveModel {
            provider_id: match self.id() {
                Some(id) => Set(id),
                None => NotSet,
            },
            name: Set(self.name()),
            type_: Set(self.type_name().to_string()),
            base_url: Set(String::new()),
            cookie: Set(String::new()),
            custom: Set(String::new()),
            edit_time: Set(chrono::Local::now().naive_local()),
        }
    }

    async fn login(&self, ctx: &Context, info: &AuthInfo) -> Result<()> {
        self.funcs.login.call_async(()).await?;
        Ok(())
    }

    async fn sync(&self, ctx: &Context) -> Result<()> {
        self.funcs.sync.call_async(()).await?;
        Ok(())
    }

    async fn image(
        &self,
        ctx: &Context,
        item_id: &str,
        image_type: ImageType,
    ) -> Result<Response, ConnectError> {
        let response = self
            .funcs
            .image
            .call_async::<LuaTable>((item_id,))
            .await
            .map_err(|e| anyhow!(e))?;
        Err(ConnectError::NotImplemented)
    }

    async fn audio(
        &self,
        ctx: &Context,
        item_id: &str,
        headers: Option<HeaderMap>,
    ) -> Result<Response, ConnectError> {
        let response = self
            .funcs
            .audio
            .call_async::<LuaTable>((item_id,))
            .await
            .map_err(|e| anyhow!(e))?;
        Err(ConnectError::NotImplemented)
    }
}
