use mlua::{Lua, LuaSerdeExt, Result as LuaResult, Value};

pub(super) fn create_json_module(lua: &Lua) -> LuaResult<mlua::Function> {
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
