# vs-shell

`vs-shell` contains shell activation logic and runtime path helpers.

## Responsibilities

- render activation scripts for supported shells
- compute canonical home, cache, shim, and link paths
- manage project and global symlink targets
- model environment deltas before command execution

## Supported shells

- Bash
- Zsh
- Fish
- Nushell
- PowerShell
- Clink
