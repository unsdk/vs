mod cli;
mod command;
mod output;
mod tui;

use std::io;
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use clap_complete::{generate, shells};
use vs_core::{App, UseScope};

use crate::cli::{Cli, Commands};
use crate::command::{BackendArg, CompletionArgs, ConfigArgs};
use crate::output::{
    print_available_plugins, print_current_tool, print_current_tools, print_installed_versions,
    print_plugin_info, print_search_versions, print_status,
};
use crate::tui::{prompt_for_version_selection, run_search_tui, should_use_interactive_tui};

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
            let entries = app.available_plugins()?;
            print_available_plugins(&entries);
            Ok(0)
        }
        Commands::Add(args) => {
            let backend = args.backend.map(BackendArg::into);
            let entry = app.add_plugin(&args.name, args.source, backend)?;
            print_status(&format!(
                "Added plugin {} from {}",
                entry.name, entry.source
            ));
            Ok(0)
        }
        Commands::Remove(args) => {
            let removed = app.remove_plugin(&args.name)?;
            if removed {
                print_status(&format!("Removed plugin {}", args.name));
            } else {
                print_status(&format!("Plugin {} was not present", args.name));
            }
            Ok(0)
        }
        Commands::Update(_) => {
            let updated = app.update_registry()?;
            print_status(&format!("Updated {} registry entries", updated));
            Ok(0)
        }
        Commands::Info(args) => {
            let info = app.plugin_info(&args.name)?;
            print_plugin_info(&info);
            Ok(0)
        }
        Commands::Search(args) => {
            let versions = app.search_versions(&args.plugin, &args.args)?;
            let installed_versions = app
                .list_installed_versions()?
                .into_iter()
                .filter(|installed| installed.plugin == args.plugin)
                .map(|installed| installed.version)
                .collect::<Vec<_>>();

            if should_use_interactive_tui() {
                return run_search_tui(&app, &args.plugin, &versions);
            }

            print_search_versions(&args.plugin, &versions, &installed_versions);
            Ok(0)
        }
        Commands::Install(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            if let Some(version) = version {
                let installed = app.install_plugin_version(&plugin, Some(&version))?;
                print_status(&format!(
                    "Install {}@{} success! ",
                    installed.plugin, installed.version
                ));
                print_status(&format!(
                    "Please use `vs use {}@{}` to use it.",
                    installed.plugin, installed.version
                ));
            } else if should_use_interactive_tui() {
                if prompt_for_version_selection(&plugin)? {
                    let versions = app.search_versions(&plugin, &[])?;
                    return run_search_tui(&app, &plugin, &versions);
                }
            } else {
                anyhow::bail!("install requires specifying a version for {}", plugin);
            }
            Ok(0)
        }
        Commands::Uninstall(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            let version = version.context("uninstall requires plugin@version")?;
            let removed = app.uninstall_plugin_version(&plugin, &version)?;
            if removed {
                print_status(&format!("Uninstalled {} {}", plugin, version));
            } else {
                print_status(&format!("{} {} was not installed", plugin, version));
            }
            Ok(0)
        }
        Commands::Use(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            let version = version.context("use requires plugin@version")?;
            let installed = app.use_tool(&plugin, &version, args.scope(), args.unlink)?;
            print_status(&format!(
                "Using {} {} in {} scope",
                installed.plugin,
                installed.version,
                scope_label(args.scope())
            ));
            Ok(0)
        }
        Commands::Unuse(args) => {
            app.unuse_tool(&args.plugin, args.scope())?;
            print_status(&format!(
                "Removed {} from {} scope",
                args.plugin,
                scope_label(args.scope())
            ));
            Ok(0)
        }
        Commands::List(_) => {
            let installed = app.list_installed_versions()?;
            let current = app.current_tools()?;
            print_installed_versions(&installed, &current);
            Ok(0)
        }
        Commands::Current(args) => {
            if let Some(plugin) = args.plugin {
                let current = app.current_tool(&plugin)?;
                print_current_tool(current.as_ref(), &plugin);
            } else {
                let current = app.current_tools()?;
                print_current_tools(&current);
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
            print_status(&format!(
                "Installed latest {} {} at {}",
                installed.plugin,
                installed.version,
                installed.install_dir.display()
            ));
            Ok(0)
        }
        Commands::Activate(args) => {
            print!("{}", app.activate(&args.shell)?);
            Ok(0)
        }
        Commands::Exec(args) => Ok(app.exec(&args.command, &args.args)?),
        Commands::Migrate(args) => {
            let summary = app.migrate(args.source)?;
            print_status(&format!(
                "Migrated {} roots from {}",
                summary.copied_roots,
                summary.source_home.display()
            ));
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

fn scope_label(scope: UseScope) -> &'static str {
    match scope {
        UseScope::Global => "global",
        UseScope::Project => "project",
        UseScope::Session => "session",
    }
}
