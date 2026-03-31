//! Entry point and command dispatcher for the `vs` CLI binary.

mod cli;
mod command;
mod install;
mod output;
mod tui;

use std::io;
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use clap_complete::{generate, shells};
use vs_core::App;

use crate::cli::{Cli, Commands};
use crate::command::{BackendArg, CompletionArgs, ConfigArgs};
use crate::install::{install_all_configured, install_single_spec, parse_tool_spec};
use crate::output::{
    print_available_plugins, print_current_statuses, print_current_tool, print_installed_versions,
    print_plugin_info, print_search_versions, print_status,
};
use crate::tui::{
    prompt_for_remove_confirmation, prompt_for_upgrade, run_search_tui, select_installed_version,
    should_use_interactive_tui,
};

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
            if args.names.len() > 1 && (args.source.is_some() || args.alias.is_some()) {
                anyhow::bail!(
                    "add supports multiple plugin names only without --source or --alias"
                );
            }
            if args.names.is_empty() && args.source.is_none() {
                anyhow::bail!("add requires a plugin name or --source");
            }

            if args.names.len() > 1 {
                for (index, name) in args.names.iter().enumerate() {
                    print_status(&format!(
                        "[{}/{}]: Adding {} plugin...",
                        index + 1,
                        args.names.len(),
                        name
                    ));
                    let entry = app.add_plugin(Some(name), None, None, None)?;
                    print_status(&format!(
                        "Added plugin {} from {}",
                        entry.name, entry.source
                    ));
                }
                return Ok(0);
            }

            let entry = app.add_plugin(
                args.names.first().map(String::as_str),
                args.source,
                backend,
                args.alias.as_deref(),
            )?;
            print_status(&format!(
                "Added plugin {} from {}",
                entry.name, entry.source
            ));
            print_status(&format!(
                "Please use `vs install {}@<version>` to install the version you need.",
                entry.name
            ));
            Ok(0)
        }
        Commands::Remove(args) => {
            println!("Removing this plugin will remove the installed sdk along with the plugin.");
            if !args.yes {
                if !should_use_interactive_tui() {
                    anyhow::bail!(
                        "Use the -y flag to skip confirmation in non-interactive environments"
                    );
                }
                if !prompt_for_remove_confirmation()? {
                    anyhow::bail!("remove canceled");
                }
            }
            let removed = app.remove_plugin(&args.name)?;
            if removed {
                print_status(&format!("Removed plugin {}", args.name));
            } else {
                print_status(&format!("Plugin {} was not present", args.name));
            }
            Ok(0)
        }
        Commands::Update(args) => {
            if args.all {
                let updated = app.update_all_plugins()?;
                print_status(&format!("Updated {} plugins", updated.len()));
            } else {
                let plugin = args
                    .plugin
                    .context("update requires a plugin name or --all")?;
                let entry = app.update_plugin(&plugin)?;
                print_status(&format!("Updated plugin {}", entry.name));
            }
            Ok(0)
        }
        Commands::Info(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            if let Some(version) = version {
                let path = app
                    .plugin_runtime_path(&plugin, &version)?
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| String::from("notfound"));
                if let Some(format) = args.format.as_deref() {
                    println!(
                        "{}",
                        render_template(
                            format,
                            &[
                                ("Name", plugin.as_str()),
                                ("Version", version.as_str()),
                                ("Path", path.as_str()),
                            ],
                        )
                    );
                } else {
                    println!("{path}");
                }
            } else {
                let info = app.plugin_info(&plugin)?;
                if let Some(format) = args.format.as_deref() {
                    println!(
                        "{}",
                        render_template(
                            format,
                            &[
                                ("Name", info.manifest.name.as_str()),
                                (
                                    "Version",
                                    info.manifest.version.as_deref().unwrap_or_default(),
                                ),
                                (
                                    "Homepage",
                                    info.manifest.homepage.as_deref().unwrap_or_default(),
                                ),
                                ("InstallPath", &info.manifest.source.display().to_string(),),
                                (
                                    "Description",
                                    info.manifest.description.as_deref().unwrap_or_default(),
                                ),
                            ],
                        )
                    );
                } else {
                    print_plugin_info(&info);
                }
            }
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
            if args.all {
                install_all_configured(&app, args.yes)?;
                return Ok(0);
            }
            if args.specs.is_empty() {
                anyhow::bail!("sdk name is required");
            }

            let mut failed: Vec<String> = Vec::new();
            for spec in &args.specs {
                match install_single_spec(&app, spec, args.yes) {
                    Ok(Some(installed)) => {
                        print_status(&format!(
                            "Install {}@{} success! ",
                            installed.plugin, installed.version
                        ));
                        print_status(&format!(
                            "Please use `vs use {}@{}` to use it.",
                            installed.plugin, installed.version
                        ));
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("Failed to install {spec}: {error}");
                        failed.push(spec.clone());
                    }
                }
            }
            if !failed.is_empty() {
                anyhow::bail!("failed to install: {}", failed.join(", "));
            }
            Ok(0)
        }
        Commands::Uninstall(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            let version = version.context("uninstall requires plugin@version")?;
            let result = app.uninstall_plugin_version(&plugin, &version)?;
            if result.removed {
                print_status(&format!("Uninstalled {} {}", plugin, version));
                if let Some(switched_to) = result.auto_switched {
                    print_status(&format!("Auto switch to {}@{}.", plugin, switched_to));
                }
            } else {
                print_status(&format!("{} {} was not installed", plugin, version));
            }
            Ok(0)
        }
        Commands::Use(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            let version = if let Some(version) = version {
                version
            } else if let Some(project_version) = app.project_tool_version_for_use(&plugin)? {
                project_version
            } else if should_use_interactive_tui() {
                let installed_versions = app
                    .installed_versions_for_plugin(&plugin)?
                    .into_iter()
                    .map(|installed| installed.version)
                    .collect::<Vec<_>>();
                if installed_versions.is_empty() {
                    anyhow::bail!(
                        "no installed versions available for {}. Please run `vs install {}@<version>` first",
                        plugin,
                        plugin
                    );
                }
                let Some(index) = select_installed_version(&plugin, &installed_versions)? else {
                    return Ok(0);
                };
                installed_versions[index].clone()
            } else {
                anyhow::bail!("Please specify a version to use in non-interactive environments");
            };
            let installed = app.use_tool(&plugin, &version, args.scope(), args.unlink)?;
            print_status(&format!(
                "Now using {}@{}.",
                installed.plugin, installed.version
            ));
            Ok(0)
        }
        Commands::Unuse(args) => {
            app.unuse_tool(&args.plugin, args.scope())?;
            print_status(&format!(
                "Removed {} from {} scope",
                args.plugin,
                args.scope().as_str()
            ));
            Ok(0)
        }
        Commands::List(args) => {
            if let Some(plugin) = args.plugin {
                let installed = app.installed_versions_for_plugin(&plugin)?;
                if installed.is_empty() {
                    anyhow::bail!("no available version");
                }
                let current = app.current_tool(&plugin)?;
                let current = current.into_iter().collect::<Vec<_>>();
                print_installed_versions(&installed, &current);
            } else {
                let installed = app.list_installed_versions()?;
                if installed.is_empty() {
                    anyhow::bail!("you don't have any sdk installed yet");
                }
                let current = app.current_tools()?;
                print_installed_versions(&installed, &current);
            }
            Ok(0)
        }
        Commands::Current(args) => {
            if let Some(plugin) = args.plugin {
                let current = app.current_tool(&plugin)?;
                let current =
                    current.ok_or_else(|| anyhow::anyhow!("no current version of {}", plugin))?;
                print_current_tool(Some(&current), &plugin);
            } else {
                let statuses = app.current_tool_statuses()?;
                print_current_statuses(&statuses);
            }
            Ok(0)
        }
        Commands::Config(args) => {
            handle_config(&app, args)?;
            Ok(0)
        }
        Commands::Version(_) => {
            let info = app.version_info()?;
            print_status(&format!("Version: {}", info.current_version));
            print_status(&format!("Build target: {}", info.build_target));
            print_status(&format!("Build variant: {}", info.build_variant));
            print_status(&format!("Release archive: .{}", info.archive_extension));
            Ok(0)
        }
        Commands::Cd(args) => {
            let path = match args.plugin.as_deref() {
                Some(plugin) if args.plugin_dir => app.plugin_dir(plugin)?,
                Some(plugin) => app.cd_path(plugin)?,
                None => app.home_dir(),
            };
            if should_use_interactive_tui() {
                open_shell_in_dir(&path)
            } else {
                println!("{path}");
                Ok(0)
            }
        }
        Commands::Upgrade(args) => {
            let summary = app.self_upgrade_summary()?;
            print_status(&format!("Current version: {}", summary.current_version));
            print_status(&format!("Latest available: {}", summary.latest_version));
            if !summary.updated {
                print_status("vs is already up to date.");
                return Ok(0);
            }

            if !args.yes {
                if should_use_interactive_tui() {
                    if !prompt_for_upgrade(&summary.current_version, &summary.latest_version)? {
                        print_status("Upgrade cancelled.");
                        return Ok(0);
                    }
                } else {
                    anyhow::bail!(
                        "upgrade requires confirmation in non-interactive environments; rerun with --yes"
                    );
                }
            }

            let summary = app.upgrade_self_to(&summary.latest_version)?;
            print_status(&format!("Updated to version: {}", summary.latest_version));
            Ok(0)
        }
        Commands::Activate(args) => {
            print!("{}", app.activate(&args.shell)?);
            Ok(0)
        }
        Commands::Exec(args) => {
            let (plugin, version) = parse_tool_spec(&args.spec)?;
            Ok(app.exec(&plugin, version.as_deref(), &args.command, &args.args)?)
        }
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
        Commands::CleanupSession => {
            let _ = app.cleanup_session();
            Ok(0)
        }
        Commands::CleanupStaleSessions => {
            let _ = app.cleanup_stale_sessions();
            Ok(0)
        }
        Commands::Completion(_) | Commands::Complete(_) => {
            unreachable!("completion is handled before app initialization")
        }
    }
}

fn handle_config(app: &App, args: ConfigArgs) -> Result<()> {
    if args.list {
        for (key, value) in app.list_config()? {
            println!("{key} = {value}");
        }
        return Ok(());
    }

    let key = args.key.context("config requires a key or --list")?;
    if args.unset {
        app.unset_config_value(&key)?;
        return Ok(());
    }

    if let Some(value) = args.value {
        app.set_config_value(&key, &value)?;
        return Ok(());
    }

    for (entry_key, value) in app.config_entries_for_key(&key)? {
        if entry_key == key {
            println!("{value}");
        } else {
            println!("{entry_key} = {value}");
        }
    }
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

fn render_template(template: &str, values: &[(&str, &str)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{{.{key}}}}}"), value);
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
}

fn open_shell_in_dir(path: &str) -> Result<i32> {
    let shell = if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| String::from("cmd.exe"))
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"))
    };
    let status = process::Command::new(&shell)
        .current_dir(path)
        .status()
        .with_context(|| format!("failed to open shell in {}", path))?;
    Ok(status.code().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::parse_tool_spec;

    #[test]
    fn parse_tool_spec_should_normalize_name_and_trim_v_prefix() {
        let parsed = match parse_tool_spec("NodeJS@v20.11.1") {
            Ok(parsed) => parsed,
            Err(error) => panic!("tool spec should parse: {error}"),
        };
        assert_eq!(
            parsed,
            (String::from("nodejs"), Some(String::from("20.11.1")))
        );
    }
}
