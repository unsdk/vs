use std::io::{IsTerminal, stdin, stdout};

use anyhow::Result;
use dialoguer::{FuzzySelect, theme::ColorfulTheme};
use vs_core::App;
use vs_plugin_api::AvailableVersion;

use crate::output::version_label;

pub fn should_use_interactive_tui() -> bool {
    stdin().is_terminal() && stdout().is_terminal() && std::env::var_os("CI").is_none()
}

pub fn run_search_tui(app: &App, plugin: &str, versions: &[AvailableVersion]) -> Result<i32> {
    if versions.is_empty() {
        return Ok(0);
    }

    let labels = versions.iter().map(version_label).collect::<Vec<_>>();
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Please select a version of {plugin} to install"))
        .items(&labels)
        .default(0)
        .interact_opt()?;

    if let Some(index) = selection {
        let selected = &versions[index];
        let installed = app.install_plugin_version(plugin, Some(&selected.version))?;
        println!(
            "Installed {} {} at {}",
            installed.plugin,
            installed.version,
            installed.install_dir.display()
        );
    }

    Ok(0)
}
