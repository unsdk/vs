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

    /// Removes session files for processes that no longer exist.
    ///
    /// Called during activation for shells that lack an EXIT trap
    /// (nushell, clink) via `vs __cleanup-stale-sessions`.
    pub fn cleanup_stale_sessions(&self) -> Result<(), CoreError> {
        let sessions_dir = self.home().join("sessions");
        if !sessions_dir.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            // Skip our own session file.
            if let Some(session_id) = &self.session_id {
                if path.file_stem().and_then(|s| s.to_str()) == Some(session_id.as_str()) {
                    continue;
                }
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };
            // Try to interpret the filename as a PID.
            if let Ok(pid) = stem.parse::<u32>() {
                if !process_alive(pid) {
                    let _ = fs::remove_file(&path);
                }
            }
        }
        Ok(())
    }
}

/// Checks whether a process with the given PID is still running.
fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // `kill -0` checks process existence without sending a signal.
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        // Cannot reliably check on non-unix; assume alive.
        let _ = pid;
        true
    }
}
