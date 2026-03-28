use crate::ShellError;

/// Supported interactive shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    /// POSIX bash shell.
    Bash,
    /// Z shell.
    Zsh,
    /// Fish shell.
    Fish,
    /// Nushell.
    Nushell,
    /// PowerShell.
    Pwsh,
    /// Clink on Windows CMD.
    Clink,
}

impl ShellKind {
    /// Parses a shell kind from a CLI string.
    pub fn parse(input: &str) -> Result<Self, ShellError> {
        match input {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "nushell" => Ok(Self::Nushell),
            "pwsh" | "powershell" => Ok(Self::Pwsh),
            "clink" => Ok(Self::Clink),
            _ => Err(ShellError::UnknownShell(input.to_string())),
        }
    }
}

/// Renders the activation script for a shell.
pub fn render_activation(shell: ShellKind) -> String {
    match shell {
        ShellKind::Bash => String::from(
            r#"vs_activate() {
  export VS_SESSION_ID="${VS_SESSION_ID:-$$}"
  eval "$(vs __hook-env bash)"
}
PROMPT_COMMAND="vs_activate${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
"#,
        ),
        ShellKind::Zsh => String::from(
            r#"vs_activate() {
  export VS_SESSION_ID="${VS_SESSION_ID:-$$}"
  eval "$(vs __hook-env zsh)"
}
autoload -U add-zsh-hook
add-zsh-hook chpwd vs_activate
precmd_functions+=(vs_activate)
"#,
        ),
        ShellKind::Fish => String::from(
            r#"function __vs_activate --on-variable PWD
    if not set -q VS_SESSION_ID
        set -gx VS_SESSION_ID $fish_pid
    end
    eval (vs __hook-env fish)
end
__vs_activate
"#,
        ),
        ShellKind::Nushell => String::from(
            r#"$env.VS_SESSION_ID = ($env.VS_SESSION_ID? | default $"(sys host | get pid)")
def --env __vs_activate [] {
  vs __hook-env nushell | lines | each {|line| load-env ($line | from json) }
}
__vs_activate
"#,
        ),
        ShellKind::Pwsh => String::from(
            r#"$env:VS_SESSION_ID = if ($env:VS_SESSION_ID) { $env:VS_SESSION_ID } else { $PID.ToString() }
function global:Invoke-VsActivate {
  Invoke-Expression (& vs __hook-env pwsh)
}
Invoke-VsActivate
"#,
        ),
        ShellKind::Clink => String::from(
            r#"set VS_SESSION_ID=%VS_SESSION_ID%
if "%VS_SESSION_ID%"=="" set VS_SESSION_ID=%RANDOM%
for /f "delims=" %%i in ('vs __hook-env clink') do %%i
"#,
        ),
    }
}
