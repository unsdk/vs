use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_FULL");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_LUA");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_WASI");

    let target = match env::var("TARGET") {
        Ok(target) => target,
        Err(error) => panic!("TARGET should be set by Cargo: {error}"),
    };
    let has_full = env::var_os("CARGO_FEATURE_FULL").is_some();
    let has_lua = env::var_os("CARGO_FEATURE_LUA").is_some();
    let has_wasi = env::var_os("CARGO_FEATURE_WASI").is_some();

    let variant = if has_full || (has_lua && has_wasi) {
        "full"
    } else if has_lua {
        "lua"
    } else if has_wasi {
        "wasi"
    } else {
        "bare"
    };

    let archive_extension = if target.contains("windows") {
        "zip"
    } else {
        "tar.gz"
    };

    println!("cargo:rustc-env=VS_BUILD_TARGET={target}");
    println!("cargo:rustc-env=VS_BUILD_VARIANT={variant}");
    println!("cargo:rustc-env=VS_BUILD_ARCHIVE_EXT={archive_extension}");
}
