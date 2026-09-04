---
format: aep.planning-md/1
id: story:the-os-keyring-becomes-the-local-store
kind: story
status: active
title: The OS keyring becomes the local credential store
refs:
- provider: legacy
  reference: S-036
relations:
- derived_from: epic:local-product
scope:
- confidence: cited
  path: crates/connector-secrets
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-036-the-os-keyring-becomes-the-local-store.md:61`. **read**

- [x] Values in the OS keyring on Linux, in preference to the file store.
- [x] The bound backend is published in readiness and named by `doctor`.
- [x] A live round trip against this machine's keyring, including trailing-whitespace preservation
      and the `NotFound`-versus-`Unreachable` distinction.
- [x] `connectors auth status` reports connectedness without reading a value.
- [x] Design 07 amended with the accepted topology and its exact scope.
- [ ] The prepared two-phase store, which Slack uses, still writes the unencrypted file.
- [ ] macOS Keychain and Windows Credential Manager, behind the same port.

## Context

A credential on a workstation should be sealed by the desktop session, not by file permissions
alone.

Source frontmatter: pillar Platform · areas [connector-secrets, runtime, cli] · priority 2 · design `../design/07-credential-custody-topologies.md`. **read**

Source `note:` field, quoted: “values are in the OS keyring on Linux and `connectors auth status` reports connectedness without reading one; the prepared two-phase store and macOS/Windows remain on the file backend”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-036-the-os-keyring-becomes-the-local-store.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-036-the-os-keyring-becomes-the-local-store.md`, which is not deleted and now names this artifact.

- First written 2026-08-20 · last touched 2026-08-20 · 1 revision(s)
- Legacy id `S-036`, recorded as the reference `legacy:S-036`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
