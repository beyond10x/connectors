---
format: aep.planning-md/1
id: story:a-provider-can-hold-a-credential-it-cannot-spend
kind: story
status: implemented
title: A provider can hold a credential it cannot spend
refs:
- provider: legacy
  reference: S-070
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-spec
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-070-a-provider-can-hold-a-credential-it-cannot-spend.md:21`. **read**

- [x] `custody_only = true` is a provider-level declaration field, schema'd in
      `crates/connector-spec/schema/provider-toml.schema.json` and represented in the IR.
- [x] The loader **refuses**, with a named reason each, a `custody_only` provider that declares
      `[spec]`, `[[operations]]`, `[[services]]`, `base_url`, `verify`, `[[channels]]`,
      `[[events]]`, `[[discoveries]]`, `[[graphs]]`, `[patch]`, `const_headers`, `default_auth`, or
      an `auth.oauth2` block. Each is a refusal, not a silent allowance, so the kind cannot smuggle
      a half-declared ordinary provider past review. Asked of the **declared key**, so
      `base_url = ""` and `operations = []` are refused too.
- [x] The loader **requires** at least one `[[auth]]` on a `custody_only` provider — a provider that
      holds nothing has no reason to exist. `auth = []` is refused for the same reason.
- [x] The existing refusal at `crates/connector-spec/src/provider/validation.rs:31` is unchanged
      for every other provider, and its golden `tests/golden/nothing-to-generate.error` still
      matches byte for byte.
- [x] The invariant is parameterised over every provider setting the flag, in the one consolidated
      file `crates/catalog-build/tests/main/catalog_invariants.rs` —
      `a_custody_only_provider_publishes_no_surface_and_every_other_provider_does`, asserting both
      directions. **No per-provider test file** — AGENTS.md states the rule once and parameterises
      it.
- [x] `catalog build` emits a custody-only entry; `catalog diff` is clean twice; `catalog check`
      passes. The catalog document publishes `custody_only` and carries no service at all, rather
      than one implicit service with an empty base URL.

## Context

Let a declaration say "this provider owns a credential and no request surface", so a credential
whose *use* belongs to another component can still have an owner, an address and a lifecycle here —
without any path by which connectors could spend it.

Source frontmatter: pillar Catalog · areas [connector-spec, catalog-build] · design `../design/16-subscription-credential-custody.md`. **read**

Source `note:` field, quoted: “design 16: the custody_only declaration kind. Today the loader refuses a provider with neither [spec] nor [[operations]], so custody-without-use cannot be expressed. The two workarounds available are both dishonest; this is the schema consequence.”

## Status

`done` in the source. Quoted from `docs/stories/S-070-a-provider-can-hold-a-credential-it-cannot-spend.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-070-a-provider-can-hold-a-credential-it-cannot-spend.md`, which is not deleted and now names this artifact.

- First written 2026-08-25 · last touched 2026-08-25 · 3 revision(s)
- Legacy id `S-070`, recorded as the reference `legacy:S-070`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
