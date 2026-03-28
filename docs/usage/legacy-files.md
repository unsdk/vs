# Legacy Version Files

`vs` can read several legacy files for compatibility.

## Supported files

- `.tool-versions`
- `.nvmrc`
- `.node-version`
- `.sdkmanrc`

## Behavior

Legacy files are read as project-level inputs.

Examples:

`.tool-versions`

```text
nodejs 20.11.1
java 21-tem
```

`.nvmrc`

```text
20.11.1
```

`.sdkmanrc`

```text
java=21-tem
```

## Interaction with `.vs.toml`

If a supported `.vs.toml` or compatibility TOML file is present, that explicit tool map takes precedence over legacy files.
