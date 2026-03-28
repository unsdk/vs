# Configuration

`vs` reads configuration from the active home and from project files.

## Home configuration

The main home is:

- `VS_HOME`, when set
- otherwise `~/.vs`

The default path is still `~/.vs`, but the internal layout is kept close to `vfox` for compatibility work. Legacy homes such as `~/.vfox`, `~/.version-fox`, and `VFOX_HOME` are treated as migration candidates.

## Global config file

The home config file is:

`$VS_HOME/config.yaml`

Current keys:

- `proxy.enable`
- `proxy.url`
- `storage.sdkPath`
- `registry.address`
- `legacyVersionFile.enable`
- `legacyVersionFile.strategy`
- `cache.availableHookDuration`

If `vs` is built with the `lua` feature and `registry.address` is not set, the effective default is:

`https://version-fox.github.io/vfox-plugins`

Example:

```yaml
proxy:
  enable: false
  url: ""
storage:
  sdkPath: ""
registry:
  address: /absolute/path/to/fixtures/registry/index.json
legacyVersionFile:
  enable: true
  strategy: specified
cache:
  availableHookDuration: 12h
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
vs config registry.address /absolute/path/to/index.json
```

Unset a value:

```bash
vs config --unset registry.address
```
