# Shell Activation

`vs activate <shell>` prints shell code that wires your shell to the hidden helper commands.

## Bash

```bash
eval "$(vs activate bash)"
```

## Zsh

```zsh
eval "$(vs activate zsh)"
```

## Fish

```fish
vs activate fish | source
```

## Nushell

```nu
vs activate nushell | save --force ~/.config/nushell/vs.nu
source ~/.config/nushell/vs.nu
```

## PowerShell

```powershell
Invoke-Expression (& vs activate pwsh)
```

## Clink

```bat
for /f "delims=" %i in ('vs activate clink') do %i
```

## Hidden helpers

The activation scripts delegate to hidden commands:

- `vs __hook-env <shell>`
- `vs __resolve <plugin>`

These helpers are intentionally internal and may change faster than the public command surface.
