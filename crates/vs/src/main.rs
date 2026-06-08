//! Entry point for the `vs` binary; delegates to the `vs-cli` library.

use anyhow::Result;

fn main() -> Result<()> {
    vs_cli::run()
}
