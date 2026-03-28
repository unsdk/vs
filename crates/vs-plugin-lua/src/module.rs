use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use flate2::read::GzDecoder;
use mlua::{
    Lua, LuaSerdeExt, MultiValue, Result as LuaResult, Table, UserData, UserDataMethods, Value,
};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use tar::Archive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

use vs_plugin_api::PluginError;

#[derive(Clone, Debug)]
struct HtmlSelection {
    fragments: Vec<String>,
}

impl HtmlSelection {
    fn new(fragments: Vec<String>) -> Self {
        Self { fragments }
    }
}

impl UserData for HtmlSelection {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("find", |_, this, selector: String| {
            let selector = Selector::parse(&selector)
                .map_err(|error| mlua::Error::external(error.to_string()))?;
            let mut matches = Vec::new();
            for fragment in &this.fragments {
                let html = parse_wrapped_fragment(fragment);
                for element in html.select(&selector) {
                    matches.push(element.html());
                }
            }
            Ok(HtmlSelection::new(matches))
        });
        methods.add_method("eq", |_, this, index: usize| {
            Ok(this
                .fragments
                .get(index)
                .map(|fragment| HtmlSelection::new(vec![fragment.clone()]))
                .unwrap_or_else(|| HtmlSelection::new(Vec::new())))
        });
        methods.add_method("first", |_, this, ()| {
            Ok(this
                .fragments
                .first()
                .map(|fragment| HtmlSelection::new(vec![fragment.clone()]))
                .unwrap_or_else(|| HtmlSelection::new(Vec::new())))
        });
        methods.add_method("last", |_, this, ()| {
            Ok(this
                .fragments
                .last()
                .map(|fragment| HtmlSelection::new(vec![fragment.clone()]))
                .unwrap_or_else(|| HtmlSelection::new(Vec::new())))
        });
        methods.add_method("html", |_, this, ()| Ok(this.fragments.join("")));
        methods.add_method("text", |_, this, ()| {
            let mut result = String::new();
            for fragment in &this.fragments {
                let html = parse_wrapped_fragment(fragment);
                for text in html.root_element().text() {
                    result.push_str(text);
                }
            }
            Ok(result)
        });
        methods.add_method("attr", |_, this, attribute: String| {
            for fragment in &this.fragments {
                let html = parse_wrapped_fragment(fragment);
                if let Ok(selector) = Selector::parse("*") {
                    if let Some(element) = html.select(&selector).next() {
                        if let Some(value) = element.value().attr(attribute.as_str()) {
                            return Ok(Some(value.to_string()));
                        }
                    }
                }
            }
            Ok(None::<String>)
        });
        methods.add_method("each", |_lua, this, callback: mlua::Function| {
            for (index, fragment) in this.fragments.iter().enumerate() {
                callback
                    .call::<()>((index + 1, HtmlSelection::new(vec![fragment.clone()])))
                    .map_err(|error| mlua::Error::external(error.to_string()))?;
            }
            Ok(Value::Nil)
        });
    }
}

#[derive(serde::Deserialize)]
struct HttpRequest {
    url: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
}

pub fn register_builtin_modules(lua: &Lua, user_agent: &str) -> Result<(), PluginError> {
    let globals = lua.globals();
    let package: Table = globals
        .get("package")
        .map_err(|error| PluginError::Backend(error.to_string()))?;
    let preload: Table = package
        .get("preload")
        .map_err(|error| PluginError::Backend(error.to_string()))?;

    preload
        .set(
            "json",
            create_json_module(lua).map_err(|error| PluginError::Backend(error.to_string()))?,
        )
        .map_err(|error| PluginError::Backend(error.to_string()))?;
    preload
        .set(
            "http",
            create_http_module(lua, user_agent)
                .map_err(|error| PluginError::Backend(error.to_string()))?,
        )
        .map_err(|error| PluginError::Backend(error.to_string()))?;
    preload
        .set(
            "html",
            create_html_module(lua).map_err(|error| PluginError::Backend(error.to_string()))?,
        )
        .map_err(|error| PluginError::Backend(error.to_string()))?;
    preload
        .set(
            "vfox.strings",
            create_strings_module(lua).map_err(|error| PluginError::Backend(error.to_string()))?,
        )
        .map_err(|error| PluginError::Backend(error.to_string()))?;
    preload
        .set(
            "vfox.archiver",
            create_archiver_module(lua).map_err(|error| PluginError::Backend(error.to_string()))?,
        )
        .map_err(|error| PluginError::Backend(error.to_string()))?;

    Ok(())
}

pub fn set_package_paths(lua: &Lua, plugin_root: &Path) -> Result<(), PluginError> {
    let globals = lua.globals();
    let package: Table = globals
        .get("package")
        .map_err(|error| PluginError::Backend(error.to_string()))?;
    let current_path: String = package
        .get("path")
        .map_err(|error| PluginError::Backend(error.to_string()))?;

    let hook_path = plugin_root.join("hooks").join("?.lua");
    let lib_path = plugin_root.join("lib").join("?.lua");
    let new_path = format!(
        "{};{};{}",
        hook_path.display(),
        lib_path.display(),
        current_path
    );
    package
        .set("path", new_path)
        .map_err(|error| PluginError::Backend(error.to_string()))
}

fn create_json_module(lua: &Lua) -> LuaResult<mlua::Function> {
    lua.create_function(|lua, ()| {
        let table = lua.create_table()?;
        table.set(
            "encode",
            lua.create_function(|lua, value: Value| {
                let json_value: serde_json::Value = lua.from_value(value)?;
                serde_json::to_string(&json_value).map_err(mlua::Error::external)
            })?,
        )?;
        table.set(
            "decode",
            lua.create_function(|lua, input: String| {
                let json_value: serde_json::Value =
                    serde_json::from_str(&input).map_err(mlua::Error::external)?;
                lua.to_value(&json_value)
            })?,
        )?;
        Ok(table)
    })
}

fn create_http_module(lua: &Lua, user_agent: &str) -> LuaResult<mlua::Function> {
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

fn create_html_module(lua: &Lua) -> LuaResult<mlua::Function> {
    lua.create_function(|lua, ()| {
        let table = lua.create_table()?;
        table.set(
            "parse",
            lua.create_function(|_, input: String| Ok(HtmlSelection::new(vec![input])))?,
        )?;
        Ok(table)
    })
}

fn create_strings_module(lua: &Lua) -> LuaResult<mlua::Function> {
    lua.create_function(|lua, ()| {
        let table = lua.create_table()?;
        table.set(
            "split",
            lua.create_function(|_, (input, separator): (String, String)| {
                Ok(input
                    .split(&separator)
                    .map(String::from)
                    .collect::<Vec<_>>())
            })?,
        )?;
        table.set(
            "has_prefix",
            lua.create_function(|_, (input, prefix): (String, String)| {
                Ok(input.starts_with(&prefix))
            })?,
        )?;
        table.set(
            "has_suffix",
            lua.create_function(|_, (input, suffix): (String, String)| {
                Ok(input.ends_with(&suffix))
            })?,
        )?;
        table.set(
            "trim",
            lua.create_function(|_, (input, cutset): (String, String)| {
                Ok(input
                    .trim_matches(|character| cutset.contains(character))
                    .to_string())
            })?,
        )?;
        table.set(
            "trim_space",
            lua.create_function(|_, input: String| Ok(input.trim().to_string()))?,
        )?;
        table.set(
            "contains",
            lua.create_function(
                |_, (input, needle): (String, String)| Ok(input.contains(&needle)),
            )?,
        )?;
        table.set(
            "join",
            lua.create_function(|_, (values, separator): (Vec<String>, String)| {
                Ok(values.join(&separator))
            })?,
        )?;
        Ok(table)
    })
}

fn create_archiver_module(lua: &Lua) -> LuaResult<mlua::Function> {
    lua.create_function(|lua, ()| {
        let table = lua.create_table()?;
        table.set(
            "decompress",
            lua.create_function(|lua, (archive_path, destination): (String, String)| {
                let result = decompress_archive(Path::new(&archive_path), Path::new(&destination));
                match result {
                    Ok(()) => Ok(MultiValue::from_vec(vec![Value::Nil])),
                    Err(error) => Ok(MultiValue::from_vec(vec![Value::String(
                        lua.create_string(error)?,
                    )])),
                }
            })?,
        )?;
        Ok(table)
    })
}

fn decompress_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let bytes = fs::read(archive_path).map_err(|error| error.to_string())?;
    let name = archive_path.display().to_string();
    if name.ends_with(".zip") {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
            let Some(relative_path) = file.enclosed_name() else {
                continue;
            };
            let output_path = destination.join(relative_path);
            if file.name().ends_with('/') {
                fs::create_dir_all(&output_path).map_err(|error| error.to_string())?;
                continue;
            }
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let mut output = fs::File::create(output_path).map_err(|error| error.to_string())?;
            std::io::copy(&mut file, &mut output).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let decoder = GzDecoder::new(Cursor::new(bytes));
        let mut archive = Archive::new(decoder);
        return archive
            .unpack(destination)
            .map_err(|error| error.to_string());
    }
    if name.ends_with(".tar.xz") {
        let decoder = XzDecoder::new(Cursor::new(bytes));
        let mut archive = Archive::new(decoder);
        return archive
            .unpack(destination)
            .map_err(|error| error.to_string());
    }
    if name.ends_with(".tar") {
        let mut archive = Archive::new(Cursor::new(bytes));
        return archive
            .unpack(destination)
            .map_err(|error| error.to_string());
    }
    Err(String::from("unsupported archive format"))
}

fn parse_wrapped_fragment(fragment: &str) -> Html {
    Html::parse_fragment(&format!("<vs-root>{fragment}</vs-root>"))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use mlua::Lua;

    use super::{register_builtin_modules, set_package_paths};

    #[test]
    fn html_each_should_iterate_matches() -> Result<(), Box<dyn Error>> {
        let lua = Lua::new();
        let current_dir = std::env::current_dir()?;
        set_package_paths(&lua, &current_dir)?;
        register_builtin_modules(&lua, "vs-test/0.1.0")?;

        lua.load(
            r#"
            local html = require("html")
            local doc = html.parse("<div id='a'>A</div><div id='b'>B</div>")
            result = {}
            doc:find("div"):each(function(i, selection)
              table.insert(result, selection:attr("id"))
            end)
            "#,
        )
        .exec()?;

        let result: mlua::Table = lua.globals().get("result")?;
        let first: Option<String> = result.get(1)?;
        let second: Option<String> = result.get(2)?;
        assert_eq!(first.as_deref(), Some("a"));
        assert_eq!(second.as_deref(), Some("b"));

        Ok(())
    }
}
