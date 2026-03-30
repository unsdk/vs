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
            r#"export VS_SESSION_ID="${VS_SESSION_ID:-$$}"
export __VS_ORIG_PATH="${__VS_ORIG_PATH:-$PATH}"
vs_activate() {
  eval "$(vs __hook-env bash)"
}
PROMPT_COMMAND="vs_activate${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
trap 'vs __cleanup-session 2>/dev/null' EXIT
"#,
        ),
        ShellKind::Zsh => String::from(
            r#"export VS_SESSION_ID="${VS_SESSION_ID:-$$}"
export __VS_ORIG_PATH="${__VS_ORIG_PATH:-$PATH}"
vs_activate() {
  eval "$(vs __hook-env zsh)"
}
autoload -U add-zsh-hook
add-zsh-hook chpwd vs_activate
precmd_functions+=(vs_activate)
trap 'vs __cleanup-session 2>/dev/null' EXIT
"#,
        ),
        ShellKind::Fish => String::from(
            r#"if not set -q VS_SESSION_ID
    set -gx VS_SESSION_ID $fish_pid
end
if not set -q __VS_ORIG_PATH
    set -gx __VS_ORIG_PATH $PATH
end
function __vs_activate --on-variable PWD
    eval (vs __hook-env fish)
end
function __vs_cleanup --on-event fish_exit
    vs __cleanup-session 2>/dev/null
end
__vs_activate
"#,
        ),
        ShellKind::Nushell => String::from(
            r#"$env.VS_SESSION_ID = ($env.VS_SESSION_ID? | default $"(sys host | get pid)")
$env.__VS_ORIG_PATH = ($env.__VS_ORIG_PATH? | default $env.PATH)
def --env __vs_activate [] {
  vs __hook-env nushell | lines | each {|line| load-env ($line | from json) }
}
__vs_activate
"#,
        ),
        ShellKind::Pwsh => String::from(
            r#"$env:VS_SESSION_ID = if ($env:VS_SESSION_ID) { $env:VS_SESSION_ID } else { $PID.ToString() }
if (-not $env:__VS_ORIG_PATH) { $env:__VS_ORIG_PATH = $env:PATH }
function global:Invoke-VsActivate {
  Invoke-Expression (& vs __hook-env pwsh)
}
Register-EngineEvent PowerShell.Exiting -Action {
  & vs __cleanup-session 2>$null
}
Invoke-VsActivate
"#,
        ),
        ShellKind::Clink => String::from(
            r#"set VS_SESSION_ID=%VS_SESSION_ID%
if "%VS_SESSION_ID%"=="" set VS_SESSION_ID=%RANDOM%
if "%__VS_ORIG_PATH%"=="" set __VS_ORIG_PATH=%PATH%
for /f "delims=" %%i in ('vs __hook-env clink') do %%i
"#,
        ),
    }
}
