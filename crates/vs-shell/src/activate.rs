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
            r#"export VS_SESSION_ID="$$"
export __VS_ORIG_PATH="${__VS_ORIG_PATH:-$PATH}"
vs_activate() {
  local previous_exit_status=$?
  trap -- '' SIGINT
  eval "$(vs __hook-env bash)"
  trap - SIGINT
  return $previous_exit_status
}
if ! [[ "${PROMPT_COMMAND[*]:-}" =~ vs_activate ]]; then
  PROMPT_COMMAND="vs_activate${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
fi
trap 'vs __cleanup-session 2>/dev/null' EXIT
vs_activate
"#,
        ),
        ShellKind::Zsh => String::from(
            r#"export VS_SESSION_ID="$$"
export __VS_ORIG_PATH="${__VS_ORIG_PATH:-$PATH}"
vs_activate() {
  trap -- '' SIGINT
  eval "$(vs __hook-env zsh)"
  trap - SIGINT
}
typeset -ag precmd_functions
if [[ -z "${precmd_functions[(r)vs_activate]+1}" ]]; then
  precmd_functions=(vs_activate ${precmd_functions[@]})
fi
typeset -ag chpwd_functions
if [[ -z "${chpwd_functions[(r)vs_activate]+1}" ]]; then
  chpwd_functions=(vs_activate ${chpwd_functions[@]})
fi
trap 'vs __cleanup-session 2>/dev/null' EXIT
vs_activate
"#,
        ),
        ShellKind::Fish => String::from(
            r#"set -gx VS_SESSION_ID $fish_pid
if not set -q __VS_ORIG_PATH
    set -gx __VS_ORIG_PATH $PATH
end
function __vs_activate --on-event fish_prompt
    eval (vs __hook-env fish)
end
function __vs_cleanup --on-event fish_exit
    vs __cleanup-session 2>/dev/null
end
"#,
        ),
        ShellKind::Nushell => String::from(
            r#"$env.VS_SESSION_ID = $"($nu.pid)"
$env.__VS_ORIG_PATH = ($env.__VS_ORIG_PATH? | default $env.PATH)
def --env __vs_activate [] {
  vs __hook-env nushell | lines | each {|line| load-env ($line | from json) }
}
do -i { ^vs __cleanup-stale-sessions }
__vs_activate
"#,
        ),
        ShellKind::Pwsh => String::from(
            r#"$env:VS_SESSION_ID = $PID.ToString()
if (-not $env:__VS_ORIG_PATH) { $env:__VS_ORIG_PATH = $env:PATH }
function global:Invoke-VsActivate {
  Invoke-Expression (& vs __hook-env pwsh)
}
if (-not $env:__VS_INITIALIZED) {
  $env:__VS_INITIALIZED = '1'
  $global:__vs_original_prompt = $function:prompt
  function global:prompt {
    Invoke-VsActivate
    & $global:__vs_original_prompt
  }
  Register-EngineEvent PowerShell.Exiting -Action {
    & vs __cleanup-session 2>$null
  }
}
Invoke-VsActivate
"#,
        ),
        ShellKind::Clink => String::from(
            r#"set VS_SESSION_ID=%VS_SESSION_ID%
if "%VS_SESSION_ID%"=="" set VS_SESSION_ID=%RANDOM%
if "%__VS_ORIG_PATH%"=="" set __VS_ORIG_PATH=%PATH%
vs __cleanup-stale-sessions >nul 2>nul
for /f "delims=" %%i in ('vs __hook-env clink') do %%i
"#,
        ),
    }
}
