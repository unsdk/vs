//! Install workflow helpers extracted from the main dispatcher.

use anyhow::Result;
use vs_core::App;

use crate::output::print_status;
use crate::tui::{
    prompt_for_install_all, prompt_for_plugin_addition, prompt_for_version_selection,
    run_search_tui, should_use_interactive_tui,
};

pub fn parse_tool_spec(spec: &str) -> Result<(String, Option<String>)> {
    let spec = spec.trim();
    if spec.is_empty() {
        anyhow::bail!("tool spec cannot be empty");
    }
    if let Some((plugin, version)) = spec.split_once('@') {
        let plugin = plugin.trim().to_ascii_lowercase();
        let version = version.trim().trim_start_matches('v');
        if plugin.is_empty() || version.is_empty() {
            anyhow::bail!("invalid tool spec: {spec}");
        }
        Ok((plugin, Some(version.to_string())))
    } else {
        Ok((spec.to_ascii_lowercase(), None))
    }
}

pub fn install_single_spec(
    app: &App,
    spec: &str,
    yes: bool,
) -> Result<Option<vs_core::InstalledVersion>> {
    let (plugin, version) = parse_tool_spec(spec)?;
    ensure_plugin_added_for_install(app, &plugin, yes)?;

    let version = resolve_install_version(app, &plugin, version.as_deref())?;
    let Some(version) = version else {
        return Ok(None);
    };
    let installed = app.install_plugin_version(&plugin, Some(&version))?;
    Ok(Some(installed))
}

pub fn install_all_configured(app: &App, yes: bool) -> Result<()> {
    let configured = app.configured_tools_for_install()?;
    let mut pending = Vec::new();

    for (plugin, version) in configured {
        let installed = app
            .installed_versions_for_plugin(&plugin)?
            .into_iter()
            .any(|installed| installed.version == version);
        if !installed {
            pending.push((plugin, version));
        }
    }

    if pending.is_empty() {
        print_status("All plugins and SDKs are already installed");
        return Ok(());
    }

    if !yes {
        if !should_use_interactive_tui() {
            anyhow::bail!(
                "Use the -y flag to automatically confirm installation in non-interactive environments"
            );
        }
        print_status("Install the following plugins and SDKs:");
        for (plugin, version) in &pending {
            print_status(&format!("  {}@{}", plugin, version));
        }
        if !prompt_for_install_all()? {
            return Ok(());
        }
    }

    let mut failed: Vec<String> = Vec::new();
    for (plugin, version) in pending {
        if let Err(error) = ensure_plugin_added_for_install(app, &plugin, true) {
            eprintln!("Failed to add {plugin}: {error}");
            failed.push(format!("{plugin}@{version}"));
            continue;
        }
        match app.install_plugin_version(&plugin, Some(&version)) {
            Ok(installed) => {
                print_status(&format!(
                    "Install {}@{} success! ",
                    installed.plugin, installed.version
                ));
                print_status(&format!(
                    "Please use `vs use {}@{}` to use it.",
                    installed.plugin, installed.version
                ));
            }
            Err(error) => {
                eprintln!("Failed to install {plugin}@{version}: {error}");
                failed.push(format!("{plugin}@{version}"));
            }
        }
    }
    if !failed.is_empty() {
        anyhow::bail!("failed to install: {}", failed.join(", "));
    }
    Ok(())
}

pub fn ensure_plugin_added_for_install(app: &App, plugin: &str, yes: bool) -> Result<()> {
    if app
        .added_plugins()?
        .iter()
        .any(|entry| entry.matches(plugin))
    {
        return Ok(());
    }

    if yes {
        print_status(&format!(
            "[{}] is not added yet, automatically proceeding with installation.",
            plugin
        ));
        let _ = app.add_plugin(Some(plugin), None, None, None)?;
        return Ok(());
    }

    if !should_use_interactive_tui() {
        anyhow::bail!(
            "Plugin {} is not installed. Use the -y flag to automatically install plugins in non-interactive environments",
            plugin
        );
    }

    if !prompt_for_plugin_addition(plugin)? {
        anyhow::bail!(
            "Plugin {} is not installed. Installation cancelled by user",
            plugin
        );
    }
    let _ = app.add_plugin(Some(plugin), None, None, None)?;
    Ok(())
}

pub fn resolve_install_version(
    app: &App,
    plugin: &str,
    version: Option<&str>,
) -> Result<Option<String>> {
    if let Some(version) = version {
        if version == "latest" {
            let latest = app
                .search_versions(plugin, &[])?
                .into_iter()
                .next()
                .map(|version| version.version)
                .ok_or_else(|| anyhow::anyhow!("no available versions for {}", plugin))?;
            print_status(&format!("Using latest version: {}", latest));
            return Ok(Some(latest));
        }
        return Ok(Some(version.to_string()));
    }

    if let Some(version) = app.project_tool_version(plugin)? {
        return Ok(Some(version));
    }

    if !should_use_interactive_tui() {
        anyhow::bail!("install requires specifying a version for {}", plugin);
    }

    if prompt_for_version_selection(plugin)? {
        let versions = app.search_versions(plugin, &[])?;
        let _ = run_search_tui(app, plugin, &versions)?;
    }
    Ok(None)
}
