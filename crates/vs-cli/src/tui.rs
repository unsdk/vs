//! Interactive terminal prompts and selectors used by the CLI.

use std::io::{IsTerminal, stdin, stdout};

use anyhow::Result;
use dialoguer::{Confirm, FuzzySelect, theme::ColorfulTheme};
use vs_core::App;
use vs_plugin_api::AvailableVersion;

use crate::output::version_label;

/// Returns `true` when interactive prompts are safe to show.
pub fn should_use_interactive_tui() -> bool {
    // Skip prompts in CI even if the streams look interactive so scripted runs stay deterministic.
    stdin().is_terminal() && stdout().is_terminal() && std::env::var_os("CI").is_none()
}

/// Runs the interactive `search` flow for a plugin.
///
/// # Errors
///
/// Returns an error if the selector cannot be rendered or the chosen version fails to install.
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

/// Asks whether the user wants to pick a version interactively.
pub fn prompt_for_version_selection(plugin: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "No {plugin} version provided, do you want to select a version to install?"
        ))
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Asks whether a missing plugin should be added before continuing.
pub fn prompt_for_plugin_addition(plugin: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Plugin {plugin} is not added yet. Do you want to add it now?"
        ))
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Asks whether every configured plugin and SDK should be installed.
pub fn prompt_for_install_all() -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you want to install these plugins and SDKs?")
        .default(true)
        .interact()
        .map_err(Into::into)
}

/// Asks for confirmation before removing a plugin and its installed SDKs.
pub fn prompt_for_remove_confirmation() -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Please confirm")
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Asks whether to upgrade the `vs` binary to the discovered latest version.
pub fn prompt_for_upgrade(current_version: &str, latest_version: &str) -> Result<bool> {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Upgrade vs from {current_version} to {latest_version}?"
        ))
        .default(true)
        .interact()
        .map_err(Into::into)
}

/// Presents available versions for a plugin and returns the selected index.
pub fn select_version(plugin: &str, versions: &[AvailableVersion]) -> Result<Option<usize>> {
    let labels = versions.iter().map(version_label).collect::<Vec<_>>();
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Please select a version of {plugin} to install"))
        .items(&labels)
        .default(0)
        .interact_opt()
        .map_err(Into::into)
}

/// Presents installed versions for a plugin and returns the selected index.
pub fn select_installed_version(plugin: &str, versions: &[String]) -> Result<Option<usize>> {
    FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Please select a version of {plugin} to use"))
        .items(versions)
        .default(0)
        .interact_opt()
        .map_err(Into::into)
}
