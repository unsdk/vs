use clap::Args;

/// Hidden helper that renders shell env updates.
#[derive(Debug, Args)]
pub struct HookEnvArgs {
    /// Shell name.
    pub shell: String,
}
