---
id: S-036
title: "The OS keyring becomes the local credential store"
pillar: Platform
status: in-progress
priority: 2
design: ../design/07-credential-custody-topologies.md
epic: local-product
areas: [connector-secrets, runtime, cli]
note: "values are in the OS keyring on Linux and `connectors auth status` reports connectedness without reading one; the prepared two-phase store and macOS/Windows remain on the file backend"
---

# The OS keyring becomes the local credential store

## Goal

A credential on a workstation should be sealed by the desktop session, not by file permissions
alone.

## Why

`FileStore` protects a value with owner plus mode `0600`. That is a real guarantee against another
user on the machine and **none at all** against a copied backup, a synced home directory, or
anything running as you. It was the right middle rung when the alternative was a Vault an operator
had to run; it is the wrong default on a laptop that has a keyring already.

## What landed

- `connector_secrets::KeyringStore` over the freedesktop Secret Service.
- `connectors-runtime` composes it in preference to `FileStore` for the personal posture, publishes
  which backend it bound in readiness, and falls back with that fact visible rather than silently.
- `connectors doctor` names the store, and warns when it is the unencrypted file or when no keyring
  is available at all.
- `SecretStore::exists` — defaulted in terms of `get`, so "is this connected?" stops being written
  as `get(…)` and dropping the result at every call site.
- `connectors auth status` — which configured providers have their credential stored, answered from
  the declaration plus a presence probe.

## Two decisions worth the words

**`secret-tool`, not the `keyring` crate.** Measured before choosing, as the plan required:
`keyring` v4 with the zbus Secret Service backend resolves to 97 packages, **48 of which are not
already in this component's lockfile** — including `async-io`, `async-executor`, `async-task`,
`blocking`, `polling` and `futures-lite`, the whole smol executor stack, linked beside the Tokio the
binary already runs. A second async runtime and a 17% enlargement of a root workspace member's
dependency graph, to make a DBus call. `secret-tool` is libsecret's own interface, adds no Rust
dependency, and takes the credential on **stdin** so it never appears in `argv`. The cost is a
process spawn, paid when a Connection opens rather than per request.

**No `references` enumeration, deliberately.** `secret-tool search` prints a `secret = …` line for
every match and has no attribute-only mode, so implementing the port's enumeration through it would
mean reading every value in scope into memory to list their names. The store answers `Unsupported`,
and `connectors auth status` covers the operator's actual question — *is this provider connected?* —
from the configuration plus a presence probe.

That second decision was learned rather than reasoned: running `secret-tool search` against the
Connectors collection during implementation put a live GitLab token into a session transcript. It
was rotated. A tool whose safe path is "remember not to run the obvious command" does not have a
safe path, which is why `auth status` exists rather than a documentation warning.

## Acceptance

- [x] Values in the OS keyring on Linux, in preference to the file store.
- [x] The bound backend is published in readiness and named by `doctor`.
- [x] A live round trip against this machine's keyring, including trailing-whitespace preservation
      and the `NotFound`-versus-`Unreachable` distinction.
- [x] `connectors auth status` reports connectedness without reading a value.
- [x] Design 07 amended with the accepted topology and its exact scope.
- [ ] The prepared two-phase store, which Slack uses, still writes the unencrypted file.
- [ ] macOS Keychain and Windows Credential Manager, behind the same port.

## Evidence

- `connector-secrets` 46 passed plus 3 ignored; the live keyring test passes with
  `cargo test -p connector-secrets -- --ignored` on a workstation.
- `connectors-console` 36, `connectors-cli` 4, `connectors-runtime` 11.
- End to end: `credential_store: keyring` in readiness, the GitLab credential present in the
  keyring with its address as attributes, and `gitlab-user-get` returning live data with no
  plaintext credential file on disk.

## Superseded by

`story:the-os-keyring-becomes-the-local-store` in the AEP planning store, at
`.engineering/planning/story/the-os-keyring-becomes-the-local-store.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
