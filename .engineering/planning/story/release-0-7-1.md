---
format: aep.planning-md/1
id: story:release-0-7-1
kind: story
status: active
title: Release binary inspection and current fixes as Connectors v0.7.1
relations:
- derived_from: runbook:small-wave-after-0-7-0
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: WHATS-NEW.md
- confidence: cited
  path: catalog
- confidence: cited
  path: connectors.lock
- confidence: cited
  path: crates
- confidence: cited
  path: docs/architecture/interfaces.md
revision: 4
---
## Acceptance

The approved Connectors v0.7.1 release contains the reviewed `inspect upgrade` command and already-merged fixes, passes all authoritative release checks, publishes four Unix archives with matching checksums and release notes, delivers the updated public documentation, and retires only safely published wave-owned worktrees.

## Scope

Release identity across current internal Cargo manifests/locks and generated catalog artifacts; CHANGELOG.md and WHATS-NEW.md; existing command documentation; source and tag delivery through Atlas; deterministic Website source lock and Atlas snapshot publication. External dependency versions, frozen contracts and release workflow semantics remain unchanged. Implementation belongs to story:the-binary-says-what-it-carries. Changes already on main include subscription OAuth recovery and generated-service remediation; retain and describe them as released fixes.
