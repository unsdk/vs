# vs-installer

`vs-installer` handles transactional runtime installs.

## Responsibilities

- stage installs in a temporary directory
- copy unpacked runtime trees into the staging area
- validate staged installs before promotion
- atomically rename the staged runtime into the final cache location
- persist install receipts
- uninstall cached versions

## Current scope

This build focuses on local filesystem sources so the install path is deterministic and easy to test.
