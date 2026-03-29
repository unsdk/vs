use std::io::{IsTerminal, stdin, stdout};

use anyhow::Result;
use dialoguer::{Confirm, FuzzySelect, theme::ColorfulTheme};
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

    let selection = select_version(plugin, versions)?;

    if let Some(index) = selection {
        let selected = &versions[index];
        let installed = app.install_plugin_version(plugin, Some(&selected.version))?;
        println!(
            "Install {}@{} success! ",
            installed.plugin, installed.version
        );
        println!(
            "Please use `vs use {}@{}` to use it.",
            installed.plugin, installed.version
        );
    }

    Ok(0)
}

pub fn prompt_for_version_selection(plugin: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "No {plugin} version provided, do you want to select a version to install?"
        ))
        .default(false)
        .interact()
        .map_err(Into::into)
}

pub fn prompt_for_plugin_addition(plugin: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Plugin {plugin} is not added yet. Do you want to add it now?"
        ))
        .default(false)
        .interact()
        .map_err(Into::into)
}

pub fn prompt_for_install_all() -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to install these plugins and SDKs?")
        .default(true)
        .interact()
        .map_err(Into::into)
}

pub fn prompt_for_upgrade(current_version: &str, latest_version: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Upgrade vs from {current_version} to {latest_version}?"
        ))
        .default(true)
        .interact()
        .map_err(Into::into)
}

pub fn select_version(plugin: &str, versions: &[AvailableVersion]) -> Result<Option<usize>> {
    let labels = versions.iter().map(version_label).collect::<Vec<_>>();
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Please select a version of {plugin} to install"))
        .items(&labels)
        .default(0)
        .interact_opt()
        .map_err(Into::into)
}

pub fn select_installed_version(plugin: &str, versions: &[String]) -> Result<Option<usize>> {
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Please select a version of {plugin} to use"))
        .items(versions)
        .default(0)
        .interact_opt()
        .map_err(Into::into)
}
