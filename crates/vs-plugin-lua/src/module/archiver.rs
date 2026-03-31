use std::fs;
use std::io::Cursor;
use std::path::Path;

use flate2::read::GzDecoder;
use mlua::{Lua, MultiValue, Result as LuaResult, Value};
use tar::Archive;
use xz2::read::XzDecoder;
use zip::ZipArchive;

pub(super) fn create_archiver_module(lua: &Lua) -> LuaResult<mlua::Function> {
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
            // Ignore entries with path traversal components before joining them to the destination.
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
