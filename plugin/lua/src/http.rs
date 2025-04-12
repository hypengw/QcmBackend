use mlua::prelude::*;
use mlua::{ExternalResult, Lua, Result, UserData, UserDataMethods};
use qcm_core::http::HttpClient;
use reqwest::header::HeaderName;
use reqwest::{header::HeaderMap, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

fn header_map_to_table(lua: &Lua, headers: &HeaderMap) -> Result<LuaTable> {
    let table = lua.create_table()?;
    for (name, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            table.set(name.as_str(), v)?;
        }
    }
    Ok(table)
}

fn table_to_header_map(table: &LuaTable) -> Result<HeaderMap> {
    let mut header_map = HeaderMap::new();
    for pair in table.pairs::<String, String>() {
        let (k, v) = pair?;
        header_map.insert(
            HeaderName::from_str(&k).map_err(mlua::Error::external)?,
            v.parse().map_err(mlua::Error::external)?,
        );
    }
    Ok(header_map)
}

struct LuaResponse(Option<Response>);

impl UserData for LuaResponse {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut("text", |_, mut this, ()| async move {
            this.0
                .take()
                .unwrap()
                .text()
                .await
                .map_err(mlua::Error::external)
        });

        methods.add_async_method_mut("json", |lua, mut this, ()| async move {
            let value: Value = this
                .0
                .take()
                .unwrap()
                .json()
                .await
                .map_err(mlua::Error::external)?;
            lua.to_value(&value)
        });

        methods.add_method("status", |_, this, ()| {
            Ok(this.0.as_ref().unwrap().status().as_u16())
        });

        methods.add_method("headers", |lua, this, ()| {
            let headers = this.0.as_ref().unwrap().headers();
            header_map_to_table(lua, headers)
        });
    }
}

struct LuaClient(HttpClient);

impl UserData for LuaClient {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "get",
            |lua, this, (url, headers): (String, Option<LuaTable>)| async move {
                let req = LuaRequest::build(&this.0, "GET", url, headers, None)?;
                Ok(req)
            },
        );

        methods.add_async_method("post", |lua, this, (url, headers, body): (String, Option<LuaTable>, Option<LuaValue>)| async move {
            let req = LuaRequest::build(&this.0, "POST", url, headers, body)?;
            Ok(req)
        });

        methods.add_async_method("put", |lua, this, (url, headers, body): (String, Option<LuaTable>, Option<LuaValue>)| async move {
            let req = LuaRequest::build(&this.0, "PUT", url, headers, body)?;
            Ok(req)
        });

        methods.add_async_method(
            "delete",
            |lua, this, (url, headers): (String, Option<LuaTable>)| async move {
                let req = LuaRequest::build(&this.0, "DELETE", url, headers, None)?;
                Ok(req)
            },
        );
    }
}

struct LuaRequest(reqwest::RequestBuilder);

impl LuaRequest {
    fn build(
        client: &HttpClient,
        method: &str,
        url: String,
        headers: Option<LuaTable>,
        body: Option<LuaValue>,
    ) -> Result<Self> {
        let mut req = match method {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            _ => return Err(mlua::Error::runtime("Unsupported HTTP method")),
        };

        if let Some(headers_table) = headers {
            req = req.headers(table_to_header_map(&headers_table)?);
        }

        if let Some(body_value) = body {
            req = match body_value {
                LuaValue::String(s) => req.body(s.to_str()?.to_string()),
                LuaValue::Table(t) => {
                    let json_str = serde_json::to_string(&t).map_err(mlua::Error::external)?;
                    req.header("Content-Type", "application/json")
                        .body(json_str)
                }
                _ => return Err(mlua::Error::runtime("Invalid body type")),
            };
        }

        Ok(Self(req))
    }
}

impl UserData for LuaRequest {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut("send", |_, this, ()| async move {
            let response = this
                .0
                .try_clone()
                .unwrap()
                .send()
                .await
                .map_err(mlua::Error::external)?;
            Ok(LuaResponse(Some(response)))
        });
    }
}

fn setup_http_client(lua: &Lua, client: HttpClient) -> Result<()> {
    lua.globals().set("http", LuaClient(client))?;
    Ok(())
}
