use std::fs;

use crate::{App, CoreError};

impl App {
    /// Removes the session tools file for the current session.
    ///
    /// Called by the shell EXIT trap via `vs __cleanup-session` so that
    /// the home directory is resolved at runtime rather than hardcoded
    /// in the activation script.
    pub fn cleanup_session(&self) -> Result<(), CoreError> {
        let session_file = self.session_file()?;
        if session_file.exists() {
            fs::remove_file(&session_file)?;
        }
        Ok(())
    }
}
