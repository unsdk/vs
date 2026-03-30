use clap::{CommandFactory, Parser, Subcommand};

use crate::command::{
    ActivateArgs, AddArgs, AvailableArgs, CdArgs, CompletionArgs, ConfigArgs, CurrentArgs,
    ExecArgs, HookEnvArgs, InfoArgs, InstallArgs, ListArgs, MigrateArgs, RemoveArgs, ResolveArgs,
    SearchArgs, UninstallArgs, UnuseArgs, UpdateArgs, UpgradeArgs, UseArgs,
};

/// `vs` command line interface.
#[derive(Debug, Parser)]
#[command(
    name = "vs",
    version,
    about = "A runtime version manager inspired by vfox",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Parsed command.
    #[command(subcommand)]
    pub command: Commands,
}

/// Supported `vs` subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    Available(AvailableArgs),
    Add(AddArgs),
    Remove(RemoveArgs),
    Update(UpdateArgs),
    Info(InfoArgs),
    Search(SearchArgs),
    #[command(visible_alias = "i")]
    Install(InstallArgs),
    #[command(visible_alias = "un")]
    Uninstall(UninstallArgs),
    #[command(visible_alias = "u")]
    Use(UseArgs),
    Unuse(UnuseArgs),
    #[command(visible_alias = "ls")]
    List(ListArgs),
    #[command(visible_alias = "c")]
    Current(CurrentArgs),
    Config(ConfigArgs),
    Cd(CdArgs),
    Upgrade(UpgradeArgs),
    Activate(ActivateArgs),
    Completion(CompletionArgs),
    #[command(visible_alias = "x")]
    Exec(ExecArgs),
    Migrate(MigrateArgs),
    #[command(hide = true, name = "__hook-env")]
    HookEnv(HookEnvArgs),
    #[command(hide = true, name = "__resolve")]
    Resolve(ResolveArgs),
    #[command(hide = true, name = "__complete")]
    Complete(CompletionArgs),
    #[command(hide = true, name = "__cleanup-session")]
    CleanupSession,
}

impl Cli {
    /// Builds a clap command for shell completion generation.
    pub fn command_factory() -> clap::Command {
        Self::command()
    }
}
