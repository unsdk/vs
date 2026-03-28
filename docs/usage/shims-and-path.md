# Shims and PATH

`vs` manages runtime paths through a combination of install directories, optional links, and shell environment updates.

## Install layout

Installed runtimes live under:

`$VS_HOME/cache/<plugin>/versions/<version>`

## Current links

Global scope may use:

`$VS_HOME/cache/<plugin>/current`

Project scope may use:

`<project>/.vs/sdks/<plugin>`

If project scope is activated with `--unlink`, the version is still recorded in `.vs.toml`, but the project link is skipped.

## PATH precedence

When `vs` builds the runtime environment, it follows:

1. Project
2. Session
3. Global
4. Existing system PATH

The CLI currently prepends each runtime `bin/` directory before the inherited PATH.

## Direct execution

You do not need shell activation to run a tool through `vs`.

Example:

```bash
vs exec node
```
