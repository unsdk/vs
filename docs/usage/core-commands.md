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
vs cd nodejs
```

## Execution commands

Run a command with the resolved runtime environment:

```bash
vs exec node
```

Generate shell activation:

```bash
vs activate bash
```

Generate completion:

```bash
vs completion bash
```
