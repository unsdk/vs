use std::process::Command;

use crate::{App, CoreError};

impl App {
    /// Executes a command with the currently resolved runtime environment.
    pub fn exec(&self, command: &str, args: &[String]) -> Result<i32, CoreError> {
        let delta = self.build_env()?;
        let path_value = self.path_with_delta(&delta)?;

        let mut child = Command::new(command);
        child.args(args);
        child.env("PATH", path_value);
        for (key, value) in delta.vars {
            child.env(key, value);
        }

        let status = child
            .status()
            .map_err(|error| CoreError::CommandExecution {
                command: command.to_string(),
                message: error.to_string(),
            })?;

        Ok(status.code().unwrap_or(1))
    }
}
