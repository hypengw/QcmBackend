use crate::error::FromLuaError;
use crate::util::to_lua;
use mlua::prelude::*;
use qcm_core::db::{self, DbChunkOper};
use qcm_core::model as sqlm;
use qcm_core::provider::{
    AuthResult, HasCommonData, ProviderCommon, ProviderCommonData, ProviderSession, QrInfo,
};
use qcm_core::{
    anyhow,
    db::DbOper,
    error::ProviderError,
    event::Event as CoreEvent,
    http::{CookieStoreRwLock, HasCookieJar, HeaderMap, HttpClient},
    model::type_enum::ImageType,
    provider::{AuthInfo, Context, Provider},
    Result,
};
use reqwest::Response;
use sea_orm::*;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::crypto::create_crypto_module;
use crate::http::LuaClient;

struct LuaProviderInner {
    common: ProviderCommonData,
    client: qcm_core::http::HttpClient,
    jar: Arc<CookieStoreRwLock>,
}

struct LuaInner(Arc<LuaProviderInner>);

impl LuaUserData for LuaInner {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.0.common.id()));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("device_id", |_, this, _: ()| {
            Ok(this.0.common.device_id.clone())
        });
    }
}

struct LuaImpl {
    load: LuaFunction,
    save: LuaFunction,
    check: LuaFunction,
    login: LuaFunction,
    sync: LuaFunction,
    qr: Option<LuaFunction>,
    image: LuaFunction,
    audio: LuaFunction,
}

pub struct LuaProvider {
    inner: Arc<LuaProviderInner>,
    funcs: LuaImpl,
    lua: Lua,
}

impl LuaProvider {
    pub fn new(
        id: Option<i64>,
        name: &str,
        device_id: &str,
        meta_type: &str,
        script_path: &Path,
    ) -> Result<Self> {
        let jar = Arc::new(CookieStoreRwLock::default());

        let lua = Lua::new();

        let client = qcm_core::http::client_builder_with_jar(jar.clone())
            .build()
            .unwrap();
        let inner = Arc::new(LuaProviderInner {
            common: ProviderCommonData::new(id, name, device_id, meta_type),
            client: client.clone(),
            jar: jar,
        });
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
            qcm_table.set("crypto", create_crypto_module(&lua)?)?;
            qcm_table.set("json", create_json_module(&lua)?)?;

            let inner = inner.clone();
            qcm_table.set(
                "get_http_client",
                lua.create_function(move |_, ()| {
                    Ok(LuaClient(inner.client.clone(), inner.jar.clone()))
                })?,
            )?;
            qcm_table.set(
                "debug",
                lua.create_function(
                    |_, val: LuaValue| match serde_json::to_string_pretty(&val) {
                        Ok(val) => {
                            log::info!("{}", val);
                            Ok(())
                        }
                        Err(e) => Err(mlua::Error::external(e)),
                    },
                )?,
            )?;
            lua.globals().set("qcm", qcm_table)?;
        }

        // Load the provider script
        let func = lua.load(script_path).into_function()?;
        let provider_table = func.call::<LuaTable>(())?;

        let provider = Self {
            inner,
            funcs: LuaImpl {
                save: provider_table
                    .get::<LuaFunction>("save")
                    .map_err(|_| anyhow!("save func not found"))?,
                load: provider_table
                    .get::<LuaFunction>("load")
                    .map_err(|_| anyhow!("load func not found"))?,
                check: provider_table
                    .get::<LuaFunction>("check")
                    .map_err(|_| anyhow!("check func not found"))?,
                login: provider_table
                    .get::<LuaFunction>("login")
                    .map_err(|_| anyhow!("login func not found"))?,
                sync: provider_table
                    .get::<LuaFunction>("sync")
                    .map_err(|_| anyhow!("sync func not found"))?,
                qr: provider_table.get::<LuaFunction>("qr").ok(),
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
        return self.inner.client.clone();
    }
}

impl HasCookieJar for LuaProvider {
    fn jar(&self) -> Arc<CookieStoreRwLock> {
        self.inner.jar.clone()
    }
}

impl HasCommonData for LuaProvider {
    fn common<'a>(&'a self) -> &'a ProviderCommonData {
        &self.inner.common
    }
}

#[async_trait::async_trait]
impl Provider for LuaProvider {
    fn load(&self, data: &str) {
        let _ = self.funcs.load.call::<()>(data.to_string());
    }

    fn save(&self) -> String {
        self.funcs.save.call::<String>(()).unwrap_or_default()
    }

    async fn check(&self, _ctx: &Context) -> Result<(), ProviderError> {
        self.funcs
            .check
            .call_async::<()>(())
            .await
            .map_err(ProviderError::from_err)
    }

    async fn auth(&self, _ctx: &Context, info: &AuthInfo) -> Result<AuthResult, ProviderError> {
        let res = self
            .funcs
            .login
            .call_async::<LuaValue>(to_lua(&self.lua, info))
            .await
            .and_then(|v| self.lua.from_value(v))
            .map_err(ProviderError::from_err);

        if let Ok(AuthResult::Ok) = &res {
            self.load_auth_info(&info.server_url, Some(info.method.clone()));
        }
        res
    }

    async fn sync(&self, ctx: &Context) -> Result<(), ProviderError> {
        let now = chrono::Utc::now();

        self.funcs
            .sync
            .call_async::<()>(LuaContext(ctx.clone(), self.id()))
            .await
            .map_err(ProviderError::from_err)?;

        if let Some(id) = self.id() {
            let txn = ctx.db.begin().await?;
            db::sync::sync_drop_before(&txn, id, now).await?;
            txn.commit().await?;
        }
        Ok(())
    }

    async fn qr(&self, _ctx: &Context) -> Result<QrInfo, ProviderError> {
        let func = self
            .funcs
            .qr
            .as_ref()
            .ok_or(ProviderError::NotImplemented)?;
        func.call_async::<LuaValue>(())
            .await
            .and_then(|v| self.lua.from_value(v))
            .map_err(ProviderError::from_err)
    }

    async fn image(
        &self,
        _ctx: &Context,
        item_id: &str,
        _image_type: ImageType,
    ) -> Result<Response, ProviderError> {
        let response = self
            .funcs
            .image
            .call_async::<LuaTable>((item_id,))
            .await
            .map_err(|e| anyhow!(e))?;
        Err(ProviderError::NotImplemented)
    }

    async fn audio(
        &self,
        _ctx: &Context,
        item_id: &str,
        _headers: Option<HeaderMap>,
    ) -> Result<Response, ProviderError> {
        let response = self
            .funcs
            .audio
            .call_async::<LuaTable>((item_id,))
            .await
            .map_err(|e| anyhow!(e))?;
        Err(ProviderError::NotImplemented)
    }
}

fn create_json_module(lua: &Lua) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set(
        "encode",
        lua.create_function(|_, v: mlua::Value| {
            serde_json::to_string(&v).map_err(|e| mlua::Error::external(e))
        })?,
    )?;
    t.set(
        "decode",
        lua.create_function(|lua, str: String| {
            let v: serde_json::Value =
                serde_json::from_str(&str).map_err(|e| mlua::Error::external(e))?;
            to_lua(&lua, &v)
        })?,
    )?;
    Ok(t)
}

struct LuaContext(Context, /* provider_id */ Option<i64>);

impl LuaUserData for LuaContext {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("sync_libraries", |lua, this, models: LuaValue| async move {
            let models: Vec<sqlm::library::Model> = lua.from_value(models)?;

            let txn = this.0.db.begin().await.map_err(mlua::Error::external)?;
            let conflict = [
                sqlm::library::Column::ProviderId,
                sqlm::library::Column::NativeId,
            ];

            let exclude = [sqlm::library::Column::LibraryId];

            let iter = models.into_iter().map(|i| -> sqlm::library::ActiveModel {
                let id = i.library_id;
                let mut a: sqlm::library::ActiveModel = i.into();
                if id == -1 {
                    a.library_id = NotSet
                }
                a
            });

            DbOper::insert(&txn, iter, &conflict, &exclude)
                .await
                .map_err(mlua::Error::external)?;
            txn.commit().await.map_err(mlua::Error::external)?;

            let ids: Vec<i64> = sqlm::library::Entity::find()
                .filter(sqlm::library::Column::ProviderId.eq(this.1))
                .select_only()
                .column(sqlm::library::Column::LibraryId)
                .into_tuple()
                .all(&this.0.db)
                .await
                .map_err(mlua::Error::external)?;
            Ok(ids)
        });
        methods.add_async_method("sync_albums", |lua, this, models: LuaValue| async move {
            let models: Vec<sqlm::album::Model> = lua.from_value(models)?;

            let txn = this.0.db.begin().await.map_err(mlua::Error::external)?;
            let conflict = [
                sqlm::album::Column::LibraryId,
                sqlm::album::Column::NativeId,
            ];
            let exclude = [sqlm::album::Column::Id];
            let iter = models.into_iter().map(|i| {
                let mut a: sqlm::album::ActiveModel = i.into();
                a.id = NotSet;
                a
            });

            DbChunkOper::<50>::insert(&txn, iter, &conflict, &exclude)
                .await
                .map_err(mlua::Error::external)?;

            txn.commit().await.map_err(mlua::Error::external)?;
            Ok(())
        });
        methods.add_async_method("sync_artists", |lua, this, models: LuaValue| async move {
            let models: Vec<sqlm::artist::Model> = lua.from_value(models)?;

            let txn = this.0.db.begin().await.map_err(mlua::Error::external)?;
            let conflict = [
                sqlm::artist::Column::LibraryId,
                sqlm::artist::Column::NativeId,
            ];
            let exclude = [sqlm::artist::Column::Id];
            let iter = models.into_iter().map(|i| {
                let mut a: sqlm::artist::ActiveModel = i.into();
                a.id = NotSet;
                a
            });

            DbChunkOper::<50>::insert(&txn, iter, &conflict, &exclude)
                .await
                .map_err(mlua::Error::external)?;

            txn.commit().await.map_err(mlua::Error::external)?;
            Ok(())
        });
    }
}
