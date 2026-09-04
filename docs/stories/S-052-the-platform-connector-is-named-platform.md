---
id: S-052
title: "The platform connector is named platform"
pillar: Platform
status: done
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
- 2026-08-24 — implemented on impl/S-052: crate `integration-b10x` → `integration-platform`;
  `B10x{Connection,Integration}Config`/`B10xBackend`/`B10xIntegrationError` →
  `Platform*`; `ConnectionInitiator::B10x` → `::Platform` (wire id stays `b10x` via
  serde rename, pinned by the contract vectors); `InitiationConfig::B10x` → `::Platform`
  (serializes `platform`, old spelling alias-parsed); config field `b10x` → `platform`
  behind `#[serde(alias = "b10x")]` in personal and hosted configs, proven by
  `an_existing_{personal,hosted}_config_with_the_old_b10x_section_parses_unchanged`;
  `b10x_only()` → `platform_only()` and companions; consent pages and design/guide prose say
  platform; `docs/design/09-b10x-modules.md` → `09-platform-modules.md` with a dated retitle
  note; check-brand.sh D5 class shrunk to published identity + serialized state and now carries a
  negative self-test that runs on every gate.
- 2026-08-24 — merged to main after independent review (PASS, 0 blocking, 2 minor: the fence's
  whole-line allowlist filter can swallow a violation co-located with an allowed token —
  pre-existing class, surface shrank with this diff; and the initiator wire token's serialize
  direction rides on serde rename bidirectionality plus the bundles round-trip test).

## Superseded by

`story:the-platform-connector-is-named-platform` in the AEP planning store, at
`.engineering/planning/story/the-platform-connector-is-named-platform.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
