use mlua::prelude::*;
use qcm_core::http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub fn lua_table_to_header_map(table: &LuaTable) -> mlua::Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for pair in table.pairs::<String, String>() {
        let (key, value) = pair?;
        headers.insert(
            http::HeaderName::from_str(&key).map_err(|e| mlua::Error::external(e))?,
            http::HeaderValue::from_str(&value).map_err(|e| mlua::Error::external(e))?,
        );
    }
    Ok(headers)
}

pub fn header_map_to_lua_table<'lua>(lua: &'lua Lua, headers: &HeaderMap) -> mlua::Result<LuaTable<'lua>> {
    let table = lua.create_table()?;
    for (key, value) in headers.iter() {
        table.set(
            key.as_str(),
            value.to_str().map_err(|e| mlua::Error::external(e))?,
        )?;
    }
    Ok(table)
}

pub fn json_to_lua_value<'lua, T>(lua: &'lua Lua, value: &T) -> mlua::Result<LuaValue<'lua>>
where
    T: Serialize,
{
    let json = serde_json::to_string(value).map_err(|e| mlua::Error::external(e))?;
    lua.load(&format!("return {}", json)).eval()
}

pub fn lua_value_to_json<'lua, T>(value: LuaValue<'lua>) -> mlua::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let json = lua_value_to_json_string(value)?;
    serde_json::from_str(&json).map_err(|e| mlua::Error::external(e))
}

fn lua_value_to_json_string(value: LuaValue) -> mlua::Result<String> {
    match value {
        LuaValue::Nil => Ok("null".to_string()),
        LuaValue::Boolean(b) => Ok(b.to_string()),
        LuaValue::Integer(i) => Ok(i.to_string()),
        LuaValue::Number(n) => Ok(n.to_string()),
        LuaValue::String(s) => Ok(format!("\"{}\"", s.to_str()?)),
        LuaValue::Table(t) => {
            if t.raw_len() > 0 {
                // Treat as array
                let mut values = Vec::new();
                for i in 1..=t.raw_len() {
                    let value: LuaValue = t.raw_get(i)?;
                    values.push(lua_value_to_json_string(value)?);
                }
                Ok(format!("[{}]", values.join(",")))
            } else {
                // Treat as object
                let mut pairs = Vec::new();
                for pair in t.pairs::<String, LuaValue>() {
                    let (key, value) = pair?;
                    pairs.push(format!("\"{}\":{}", key, lua_value_to_json_string(value)?));
                }
                Ok(format!("{{{}}}", pairs.join(",")))
            }
        }
        _ => Err(mlua::Error::external("Unsupported Lua type")),
    }
}