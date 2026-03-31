use mlua::{Lua, Result as LuaResult};

pub(super) fn create_file_module(lua: &Lua) -> LuaResult<mlua::Function> {
    lua.create_function(|lua, ()| {
        let table = lua.create_table()?;
        table.set(
            "symlink",
            lua.create_function(|_, (src, dest): (String, String)| {
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(&src, &dest).map_err(mlua::Error::external)?;
                }
                #[cfg(windows)]
                {
                    let src_path = std::path::Path::new(&src);
                    if src_path.is_dir() {
                        std::os::windows::fs::symlink_dir(&src, &dest)
                            .map_err(mlua::Error::external)?;
                    } else {
                        std::os::windows::fs::symlink_file(&src, &dest)
                            .map_err(mlua::Error::external)?;
                    }
                }
                Ok(())
            })?,
        )?;
        Ok(table)
    })
}
