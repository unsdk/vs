use mlua::{Lua, Result as LuaResult};

pub(super) fn create_strings_module(lua: &Lua) -> LuaResult<mlua::Function> {
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
        table.set(
            "trim_prefix",
            lua.create_function(|_, (input, prefix): (String, String)| {
                Ok(input
                    .strip_prefix(prefix.as_str())
                    .unwrap_or(&input)
                    .to_string())
            })?,
        )?;
        table.set(
            "trim_suffix",
            lua.create_function(|_, (input, suffix): (String, String)| {
                Ok(input
                    .strip_suffix(suffix.as_str())
                    .unwrap_or(&input)
                    .to_string())
            })?,
        )?;
        table.set(
            "fields",
            lua.create_function(|_, input: String| {
                Ok(input
                    .split_whitespace()
                    .map(String::from)
                    .collect::<Vec<_>>())
            })?,
        )?;
        Ok(table)
    })
}
