# vs-registry

`vs-registry` stores searchable plugin metadata and locally added plugin entries.

## Responsibilities

- persist the available registry index
- persist plugins explicitly added to the local home
- resolve a plugin by name or alias
- search the index by query

## Storage

The crate stores registry data under the active `vs` home:

- `registry/index.json`
- `plugins/entries.json`
