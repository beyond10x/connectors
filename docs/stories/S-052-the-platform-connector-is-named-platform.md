---
id: S-052
title: "The platform connector is named platform"
pillar: Platform
status: ready
priority: 1
areas: [integrations, config, server]
---

# The platform connector is named platform

## Goal

D5 is resolved: the platform's name is "platform" (extraction plan resolution note, 2026-08-23).
The brand sweep left ~154 code-level occurrences blocked on that decision. Rename the code-level
family: crate `integration-b10x` becomes `integration-platform`, and
`B10xConnectionConfig`, `B10xIntegrationConfig`, `B10xBackend`,
`ConnectionInitiator::B10x`, `b10x_only()` and companions follow, with prose
("B10x Identity/Integration/Provider/Session Authority", "Authorize B10x" consent
pages) moving to platform naming. Explicitly kept, each on the brand-fence allowlist: provider id
`b10x` and authority `io.b10x` (public identity, irreversible), `urn:b10x:*`,
`x-b10x-*` headers, `b10x.*.vN` contract ids, hash domains, and serialized state
(keyring service attribute). The serde-visible `[b10x]` config section moves only behind a
serde alias or a documented migration — an existing personal config must keep parsing.

## Acceptance

- No non-allowlisted `b10x` remains in crate names, type/function/variant names, or prose;
  `scripts/check-brand.sh`'s allowlist shrinks accordingly and stays negative-tested.
- An existing personal config carrying the old section name parses unchanged, proven by a test.
- `bash scripts/gate.sh` exits 0.

## Progress

- 2026-08-23 — filed on D5's resolution; the D5-blocked inventory is in the brand-sweep report.
