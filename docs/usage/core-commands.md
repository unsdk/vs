# Core Commands

This page summarizes the main `vs` commands.

## Registry commands

Refresh the searchable plugin index:

```bash
vs update
```

List available plugins:

```bash
vs available
```

Search by name or description:

```bash
vs search node
```

Add a plugin to the local home:

```bash
vs add nodejs
vs add deno --source /path/to/plugin --backend wasi
```

Remove a plugin from the local home:

```bash
vs remove nodejs
```

## Installation commands

Install a specific version:

```bash
vs install nodejs@20.11.1
```

Install the first available version:

```bash
vs install nodejs
```

Uninstall a version:

```bash
vs uninstall nodejs@20.11.1
```

Upgrade a plugin to its newest available version:

```bash
vs upgrade nodejs
```

## Activation commands

Use a version globally:

```bash
vs use nodejs@20.11.1 -g
```

Use a version in the current project:

```bash
vs use nodejs@20.11.1 -p
```

Use a version only for the current session:

```bash
VS_SESSION_ID=my-shell vs use nodejs@20.11.1 -s
```

Use an already installed version interactively:

```bash
vs use nodejs
```

`vs use` only activates installed versions. Install first with `vs install`.

If you omit the scope flag, `vs use` targets the current session by default.
In non-interactive environments, `vs use` requires an explicit version.

Use project scope without creating the `.vs/sdks/<plugin>` link:

```bash
vs use deno@1.40.5 -p --unlink
```

Remove an active version:

```bash
vs unuse nodejs -g
vs unuse nodejs -p
vs unuse nodejs -s
```

## Inspection commands

Show plugin metadata:

```bash
vs info nodejs
```

Show current versions:

```bash
vs current
vs current nodejs
```

List installed versions:

```bash
vs list
```

Print the active runtime directory:

```bash
vs cd
vs cd nodejs
vs cd nodejs --plugin
```

## Execution commands

Run a command with the resolved runtime environment:

```bash
vs exec nodejs node -v
vs exec nodejs@20.11.1 node -v
```

Generate shell activation:

```bash
vs activate bash
```

Generate completion:

```bash
vs completion bash
```
