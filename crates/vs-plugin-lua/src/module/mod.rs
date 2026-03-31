//! Built-in Lua modules that emulate the helper APIs expected by vfox-compatible plugins.

mod archiver;
mod file;
mod html;
mod http;
mod json;
mod strings;

use std::path::Path;

use mlua::{Lua, Table};

use vs_plugin_api::{IntoPluginResult, PluginError};

/// Registers the built-in compatibility modules exposed to Lua plugins.
///
/// # Errors
///
/// Returns an error when the Lua `package.preload` table cannot be populated.
pub fn register_builtin_modules(
    lua: &Lua,
    user_agent: &str,
    proxy_url: Option<&str>,
) -> Result<(), PluginError> {
    let globals = lua.globals();
    let package: Table = globals.get("package").into_plugin_result()?;
    let preload: Table = package.get("preload").into_plugin_result()?;

    preload
        .set("json", json::create_json_module(lua).into_plugin_result()?)
        .into_plugin_result()?;
    preload
        .set(
            "http",
            http::create_http_module(lua, user_agent, proxy_url).into_plugin_result()?,
        )
        .into_plugin_result()?;
    preload
        .set("html", html::create_html_module(lua).into_plugin_result()?)
        .into_plugin_result()?;
    preload
        .set(
            "vfox.strings",
            strings::create_strings_module(lua).into_plugin_result()?,
        )
        .into_plugin_result()?;
    preload
        .set(
            "vfox.archiver",
            archiver::create_archiver_module(lua).into_plugin_result()?,
        )
        .into_plugin_result()?;
    preload
        .set("file", file::create_file_module(lua).into_plugin_result()?)
        .into_plugin_result()?;

    Ok(())
}

/// Prepends plugin-local hook and library directories to `package.path`.
///
/// # Errors
///
/// Returns an error when the Lua `package` table cannot be read or updated.
pub fn set_package_paths(lua: &Lua, plugin_root: &Path) -> Result<(), PluginError> {
    let globals = lua.globals();
    let package: Table = globals.get("package").into_plugin_result()?;
    let current_path: String = package.get("path").into_plugin_result()?;

    let hook_path = plugin_root.join("hooks").join("?.lua");
    let lib_path = plugin_root.join("lib").join("?.lua");
    // Prepend plugin-local paths so helper modules override any broader Lua search path entries.
    let new_path = format!(
        "{};{};{}",
        hook_path.display(),
        lib_path.display(),
        current_path
    );
    package.set("path", new_path).into_plugin_result()
}
