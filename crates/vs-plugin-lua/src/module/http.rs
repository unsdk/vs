use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use mlua::{Lua, LuaSerdeExt, MultiValue, Result as LuaResult, Table, Value};
use reqwest::blocking::Client;

#[derive(serde::Deserialize)]
struct HttpRequest {
    url: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
}

pub(super) fn create_http_module(lua: &Lua, user_agent: &str) -> LuaResult<mlua::Function> {
    let user_agent = user_agent.to_string();
    lua.create_function(move |lua, ()| {
        let client = Client::builder()
            .user_agent(user_agent.clone())
            .build()
            .map_err(mlua::Error::external)?;
        let table = lua.create_table()?;

        let client_get = client.clone();
        table.set(
            "get",
            lua.create_function(move |lua, params: Table| {
                http_request(lua, &client_get, "GET", params)
            })?,
        )?;

        let client_head = client.clone();
        table.set(
            "head",
            lua.create_function(move |lua, params: Table| {
                http_request(lua, &client_head, "HEAD", params)
            })?,
        )?;

        let client_download = client;
        table.set(
            "download_file",
            lua.create_function(move |lua, (params, destination): (Table, String)| {
                let request: HttpRequest = lua.from_value(Value::Table(params))?;
                let mut builder = client_download.get(&request.url);
                for (key, value) in request.headers {
                    builder = builder.header(key, value);
                }
                let response = builder
                    .send()
                    .and_then(reqwest::blocking::Response::error_for_status);
                let response = match response {
                    Ok(response) => response,
                    Err(error) => {
                        return Ok(MultiValue::from_vec(vec![Value::String(
                            lua.create_string(error.to_string())?,
                        )]));
                    }
                };
                if let Some(parent) = Path::new(&destination).parent() {
                    fs::create_dir_all(parent).map_err(mlua::Error::external)?;
                }
                let bytes = response.bytes().map_err(mlua::Error::external)?;
                fs::write(destination, bytes).map_err(mlua::Error::external)?;
                Ok(MultiValue::from_vec(vec![Value::Nil]))
            })?,
        )?;
        Ok(table)
    })
}

fn http_request(lua: &Lua, client: &Client, method: &str, params: Table) -> LuaResult<MultiValue> {
    let request: HttpRequest = lua.from_value(Value::Table(params))?;
    let mut builder = match method {
        "HEAD" => client.head(&request.url),
        _ => client.get(&request.url),
    };
    for (key, value) in request.headers {
        builder = builder.header(key, value);
    }
    let response = builder
        .send()
        .and_then(reqwest::blocking::Response::error_for_status);
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return Ok(MultiValue::from_vec(vec![
                Value::Nil,
                Value::String(lua.create_string(error.to_string())?),
            ]));
        }
    };

    let headers = lua.create_table()?;
    for (key, value) in response.headers() {
        headers.set(key.as_str(), value.to_str().unwrap_or_default())?;
    }
    let result = lua.create_table()?;
    result.set("status_code", response.status().as_u16())?;
    result.set("headers", headers)?;
    result.set("content_length", response.content_length().unwrap_or(0))?;
    if method != "HEAD" {
        let body = response.text().map_err(mlua::Error::external)?;
        result.set("body", body)?;
    }
    Ok(MultiValue::from_vec(vec![Value::Table(result), Value::Nil]))
}
