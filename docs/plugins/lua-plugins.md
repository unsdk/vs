# Lua Plugins

The Lua plugin runtime reads a plugin directory with a small hook-based layout.

## Directory structure

```text
my-plugin/
  metadata.lua
  hooks/
    available.lua
    pre_install.lua
    env_keys.lua
  packages/
    1.0.0/
      bin/
```

## `metadata.lua`

`metadata.lua` returns a table:

```lua
return {
  name = "nodejs",
  description = "Fixture Node.js plugin",
  aliases = { "node" },
  legacy_filenames = { ".nvmrc", ".node-version" },
}
```

## `hooks/available.lua`

Return the versions the plugin exposes:

```lua
return {
  { version = "20.11.1", note = "Current fixture release" },
  { version = "18.19.0", note = "LTS fixture release" },
}
```

## `hooks/pre_install.lua`

Return a version-to-source map:

```lua
return {
  ["20.11.1"] = { source = "packages/20.11.1" },
}
```

## `hooks/env_keys.lua`

Return environment keys that should be exported:

```lua
return {
  { key = "NODEJS_HOME", value = "{install_dir}" },
}
```

## Notes

- The current runtime is intentionally small and fixture-friendly.
- Hooks are loaded from local files and converted into typed Rust structures.
