use std::collections::BTreeMap;

use vs_core::{CurrentTool, InstalledVersion, PluginInfo};
use vs_plugin_api::{AvailableVersion, PluginBackendKind};
use vs_registry::RegistryEntry;

pub fn print_heading(title: &str) {
    println!("{title}");
    println!();
}

pub fn print_available_plugins(entries: &[RegistryEntry]) {
    print_heading("AVAILABLE PLUGINS");
    let name_width = entries
        .iter()
        .map(|entry| entry.name.len())
        .max()
        .unwrap_or(0)
        + 2;

    for entry in entries {
        println!(
            "  {:name_width$} [{}]  {}",
            entry.name,
            backend_label(entry.backend),
            entry.description.as_deref().unwrap_or("No description"),
            name_width = name_width
        );
    }
    if entries.is_empty() {
        println!("  No plugins available.");
    }
}

pub fn print_plugin_info(info: &PluginInfo) {
    println!("Plugin Info:");
    println!("Name     -> {}", info.manifest.name);
    if let Some(version) = &info.manifest.version {
        println!("Version  -> {version}");
    }
    if let Some(homepage) = &info.manifest.homepage {
        println!("Homepage -> {homepage}");
    }
    println!(
        "Desc     -> {}",
        info.manifest
            .description
            .as_deref()
            .unwrap_or("No description")
    );
    if !info.manifest.notes.is_empty() {
        for note in &info.manifest.notes {
            println!("{note}");
        }
    }
}

pub fn print_search_versions(
    plugin: &str,
    versions: &[AvailableVersion],
    installed_versions: &[String],
) {
    print_heading(&format!("AVAILABLE VERSIONS FOR {plugin}"));
    if versions.is_empty() {
        println!("  No available versions.");
        return;
    }
    for version in versions {
        let mut label = version_label(version);
        if installed_versions
            .iter()
            .any(|installed| installed == &version.version)
        {
            label.push_str(" (installed)");
        }
        println!("  - {label}");
    }
}

pub fn print_current_tool(current: Option<&CurrentTool>, plugin: &str) {
    if let Some(current) = current {
        println!("-> v{}", current.version);
    } else {
        println!("{plugin} -> N/A");
    }
}

pub fn print_installed_versions(installed: &[InstalledVersion], current_tools: &[CurrentTool]) {
    if installed.is_empty() {
        return;
    }

    let current_map = current_tools
        .iter()
        .map(|current| (current.plugin.as_str(), current.version.as_str()))
        .collect::<BTreeMap<_, _>>();

    let mut grouped = BTreeMap::<&str, Vec<&InstalledVersion>>::new();
    for entry in installed {
        grouped.entry(&entry.plugin).or_default().push(entry);
    }

    for (plugin, versions) in grouped {
        println!("{plugin}");
        for version in versions {
            let marker = if current_map
                .get(plugin)
                .is_some_and(|current| current == &version.version.as_str())
            {
                " <— current"
            } else {
                ""
            };
            println!("  -> v{}{}", version.version, marker);
        }
    }
}

pub fn print_current_statuses(statuses: &[(String, Option<String>)]) {
    for (plugin, version) in statuses {
        if let Some(version) = version {
            println!("{plugin} -> v{version}");
        } else {
            println!("{plugin} -> N/A");
        }
    }
}

pub fn print_status(message: &str) {
    println!("{message}");
}

pub fn version_label(version: &AvailableVersion) -> String {
    let note_suffix = version
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
        .map(|note| format!(" ({note})"))
        .unwrap_or_default();
    let additions_suffix = if version.additions.is_empty() {
        String::new()
    } else {
        let additions = version
            .additions
            .iter()
            .map(|addition| {
                if addition.version.trim().is_empty() {
                    addition.name.clone()
                } else {
                    format!("{} {}", addition.name, addition.version)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(" [{additions}]")
    };
    format!("{}{}{}", version.version, note_suffix, additions_suffix)
}

pub fn backend_label(backend: PluginBackendKind) -> &'static str {
    match backend {
        PluginBackendKind::Lua => "lua",
        PluginBackendKind::Wasi => "wasi",
    }
}
