mod cli;
mod command;

use std::io;
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use clap_complete::{generate, shells};
use vs_core::{App, UseScope};

use crate::cli::{Cli, Commands};
use crate::command::{BackendArg, CompletionArgs, ConfigArgs};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let exit_code = run(cli)?;
    if exit_code != 0 {
        process::exit(exit_code);
    }
    Ok(())
}

fn run(cli: Cli) -> Result<i32> {
    match cli.command {
        Commands::Completion(args) | Commands::Complete(args) => {
            print_completion(args)?;
            Ok(0)
        }
        command => {
            let app = App::from_env().context("failed to initialize vs")?;
            run_with_app(app, command)
        }
    }
}

fn run_with_app(app: App, command: Commands) -> Result<i32> {
    match command {
        Commands::Available(_) => {
            for entry in app.available_plugins()? {
                println!(
                    "{} [{}] {}",
                    entry.name,
                    backend_label(entry.backend),
                    entry
                        .description
                        .unwrap_or_else(|| String::from("No description"))
                );
            }
            Ok(0)
        }
        Commands::Add(args) => {
            let backend = args.backend.map(BackendArg::into);
            let entry = app.add_plugin(&args.name, args.source, backend)?;
            println!("Added plugin {} from {}", entry.name, entry.source);
            Ok(0)
        }
        Commands::Remove(args) => {
            let removed = app.remove_plugin(&args.name)?;
            if removed {
                println!("Removed plugin {}", args.name);
            } else {
                println!("Plugin {} was not present", args.name);
            }
            Ok(0)
        }
        Commands::Update(_) => {
            let updated = app.update_registry()?;
            println!("Updated {} registry entries", updated);
            Ok(0)
        }
        Commands::Info(args) => {
            let info = app.plugin_info(&args.name)?;
            println!("Name: {}", info.manifest.name);
            println!("Backend: {}", backend_label(info.manifest.backend));
            println!("Source: {}", info.manifest.source.display());
            println!(
                "Description: {}",
                info.manifest
                    .description
                    .unwrap_or_else(|| String::from("No description"))
            );
            println!(
                "Available versions: {}",
                info.available_versions
                    .iter()
                    .map(|version| version.version.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            println!(
                "Installed versions: {}",
                if info.installed_versions.is_empty() {
                    String::from("<none>")
                } else {
                    info.installed_versions.join(", ")
                }
            );
            Ok(0)
        }
        Commands::Search(args) => {
            for version in app.search_versions(&args.plugin, &args.args)? {
                let note_suffix = version
                    .note
                    .as_deref()
                    .map(|note| format!(" ({note})"))
                    .unwrap_or_default();
                let additions_suffix = if version.additions.is_empty() {
                    String::new()
                } else {
                    let additions = version
                        .additions
                        .iter()
                        .map(|addition| format!("{} {}", addition.name, addition.version))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(" [{additions}]")
                };
                println!("{}{}{}", version.version, note_suffix, additions_suffix);
            }
            Ok(0)
        }
        Commands::Install(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            let installed = app.install_plugin_version(&plugin, version.as_deref())?;
            println!(
                "Installed {} {} at {}",
                installed.plugin,
                installed.version,
                installed.install_dir.display()
            );
            Ok(0)
        }
        Commands::Uninstall(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            let version = version.context("uninstall requires plugin@version")?;
            let removed = app.uninstall_plugin_version(&plugin, &version)?;
            if removed {
                println!("Uninstalled {} {}", plugin, version);
            } else {
                println!("{} {} was not installed", plugin, version);
            }
            Ok(0)
        }
        Commands::Use(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            let version = version.context("use requires plugin@version")?;
            let installed = app.use_tool(&plugin, &version, args.scope(), args.unlink)?;
            println!(
                "Using {} {} in {} scope",
                installed.plugin,
                installed.version,
                scope_label(args.scope())
            );
            Ok(0)
        }
        Commands::Unuse(args) => {
            app.unuse_tool(&args.plugin, args.scope())?;
            println!(
                "Removed {} from {} scope",
                args.plugin,
                scope_label(args.scope())
            );
            Ok(0)
        }
        Commands::List(_) => {
            for entry in app.list_installed_versions()? {
                println!(
                    "{} {} {}",
                    entry.plugin,
                    entry.version,
                    entry.install_dir.display()
                );
            }
            Ok(0)
        }
        Commands::Current(args) => {
            if let Some(plugin) = args.plugin {
                if let Some(current) = app.current_tool(&plugin)? {
                    println!(
                        "{} {} {} {}",
                        current.plugin,
                        current.version,
                        scope_name(current.scope),
                        current.source.display()
                    );
                } else {
                    println!("{} is not active", plugin);
                }
            } else {
                for current in app.current_tools()? {
                    println!(
                        "{} {} {} {}",
                        current.plugin,
                        current.version,
                        scope_name(current.scope),
                        current.source.display()
                    );
                }
            }
            Ok(0)
        }
        Commands::Config(args) => {
            handle_config(&app, args)?;
            Ok(0)
        }
        Commands::Cd(args) => {
            println!("{}", app.cd_path(&args.plugin)?);
            Ok(0)
        }
        Commands::Upgrade(args) => {
            let installed = app.upgrade_plugin(&args.plugin)?;
            println!(
                "Installed latest {} {} at {}",
                installed.plugin,
                installed.version,
                installed.install_dir.display()
            );
            Ok(0)
        }
        Commands::Activate(args) => {
            print!("{}", app.activate(&args.shell)?);
            Ok(0)
        }
        Commands::Exec(args) => Ok(app.exec(&args.command, &args.args)?),
        Commands::Migrate(args) => {
            let summary = app.migrate(args.source)?;
            println!(
                "Migrated {} roots from {}",
                summary.copied_roots,
                summary.source_home.display()
            );
            Ok(0)
        }
        Commands::HookEnv(args) => {
            print!("{}", app.hook_env(&args.shell)?);
            Ok(0)
        }
        Commands::Resolve(args) => {
            println!("{}", app.cd_path(&args.plugin)?);
            Ok(0)
        }
        Commands::Completion(_) | Commands::Complete(_) => {
            unreachable!("completion is handled before app initialization")
        }
    }
}

fn handle_config(app: &App, args: ConfigArgs) -> Result<()> {
    if args.list || (args.key.is_none() && args.value.is_none() && !args.unset) {
        for (key, value) in app.list_config()? {
            println!("{key}={value}");
        }
        return Ok(());
    }

    let key = args.key.context("config requires a key")?;
    if args.unset {
        app.unset_config_value(&key)?;
        println!("Unset config key {}", key);
        return Ok(());
    }

    let value = args
        .value
        .context("config requires a value unless --unset is used")?;
    app.set_config_value(&key, &value)?;
    println!("Set config key {}", key);
    Ok(())
}

fn print_completion(args: CompletionArgs) -> Result<()> {
    let mut command = Cli::command_factory();
    match args.shell.as_str() {
        "bash" => generate(shells::Bash, &mut command, "vs", &mut io::stdout()),
        "zsh" => generate(shells::Zsh, &mut command, "vs", &mut io::stdout()),
        "fish" => generate(shells::Fish, &mut command, "vs", &mut io::stdout()),
        "pwsh" | "powershell" => {
            generate(shells::PowerShell, &mut command, "vs", &mut io::stdout())
        }
        "nushell" => {
            println!("# Nushell completion generation is not implemented in this build.");
        }
        "clink" => {
            println!(":: clink completion generation is not implemented in this build.");
        }
        other => anyhow::bail!("unsupported completion shell: {other}"),
    }
    Ok(())
}

fn parse_tool_spec(spec: &str) -> Result<(String, Option<String>)> {
    let spec = spec.trim();
    if spec.is_empty() {
        anyhow::bail!("tool spec cannot be empty");
    }
    if let Some((plugin, version)) = spec.split_once('@') {
        if plugin.is_empty() || version.is_empty() {
            anyhow::bail!("invalid tool spec: {spec}");
        }
        Ok((plugin.to_string(), Some(version.to_string())))
    } else {
        Ok((spec.to_string(), None))
    }
}

fn backend_label(backend: vs_plugin_api::PluginBackendKind) -> &'static str {
    match backend {
        vs_plugin_api::PluginBackendKind::Lua => "lua",
        vs_plugin_api::PluginBackendKind::Wasi => "wasi",
    }
}

fn scope_label(scope: UseScope) -> &'static str {
    match scope {
        UseScope::Global => "global",
        UseScope::Project => "project",
        UseScope::Session => "session",
    }
}

fn scope_name(scope: vs_config::Scope) -> &'static str {
    match scope {
        vs_config::Scope::Project => "project",
        vs_config::Scope::Session => "session",
        vs_config::Scope::Global => "global",
        vs_config::Scope::System => "system",
    }
}
