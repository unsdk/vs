//! Native plugin contract support for `vs`.

mod backend;

pub use backend::{WasiBackend, WasiPlugin};

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use vs_plugin_api::Plugin;

    use crate::WasiPlugin;

    #[test]
    fn wasi_plugin_should_load_fixture_versions() -> Result<(), Box<dyn Error>> {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/plugins/deno-wasi");
        let plugin = WasiPlugin::load(&root)?;
        let versions = plugin.available_versions(&[])?;
        assert_eq!(versions[0].version, "1.40.5");
        Ok(())
    }
}
