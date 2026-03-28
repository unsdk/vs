# Migration from vfox

`vs` keeps its public branding, while its home and config layout now stay intentionally close to `vfox` so plugin and config compatibility remain practical.

## What `vs migrate` copies

The current migration command copies these roots when they exist:

- `config.yaml`
- `global/`
- `registry/`
- `plugins/`
- `cache/`

## Basic migration

```bash
vs migrate --source ~/.vfox
```

If no explicit source is passed, `vs` uses the first detected legacy home candidate.

## After migration

Check migrated global tools:

```bash
vs current
```

Inspect copied plugins:

```bash
vs available
vs list
```

## Notes

- Migration copies data into the active `vs` home.
- It does not rename your legacy directories.
- You can run it again after cleaning the target home if you want a fresh import.
