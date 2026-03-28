# Configuration

`vs` reads configuration from the active home and from project files.

## Home configuration

The main home is:

- `VS_HOME`, when set
- otherwise `~/.vs`

Legacy homes such as `~/.vfox`, `~/.version-fox`, and `VFOX_HOME` are treated as migration candidates.

## Global config file

The home config file is:

`$VS_HOME/config.yaml`

Current keys:

- `legacyVersionFile`
- `registry.source`

Example:

```yaml
legacyVersionFile: true
registry:
  source: /absolute/path/to/fixtures/registry/index.json
```

## Tool version files

`vs` reads project files in this order:

1. `.vs.toml`
2. `vs.toml`
3. `.vfox.toml`
4. `vfox.toml`

The format is:

```toml
[tools]
nodejs = "20.11.1"
deno = "1.40.5"
```

## Scope precedence

Active versions resolve in this order:

1. Project
2. Session
3. Global
4. System

## Config commands

List current values:

```bash
vs config --list
```

Set a value:

```bash
vs config registry.source /absolute/path/to/index.json
```

Unset a value:

```bash
vs config --unset registry.source
```
