---
format: aep.planning-md/2
id: story:resolve-spec-markers
kind: story
status: implemented
title: Every open spec marker has a recorded outcome
relations:
- serves: vision:independent-contract-adapters
revision: 5
---
## Acceptance

Every `UNMAPPED:` marker in this repository's ESS sources has one of four recorded outcomes, and
`ess specify validate` exits 0:

1. **Declared** — the semantics are read from code, OpenAPI or docs and are now modelled, citing
   the source line; where the code can run it, a conformance scenario covers it.
2. **Decided** — a design question with no source; the coordinator's default is modelled and the
   spec comment names it as a decision dated 2026-09-27.
3. **Deferred** — out of scope of the shipped system; the marker becomes a `DEFERRED:` note naming
   an existing story or artifact id that owns it (a draft story is filed when none exists).
4. **ESS limit** — the semantics are known but ESS 0.36 cannot express them; the marker becomes an
   `ESS-LIMIT:` note naming the missing construct.

No `UNMAPPED:` marker remains. The repository's full check exits 0.

## Source

Operator approval 2026-09-27 to resolve the open spec markers after wave 2.


## Result, wave 3 (2026-09-27)

99 markers at the start (the brief's pathspec missed 56 adapter markers). 60 resolved: 11 declared,
9 decided, 4 deferred, 36 ESS-LIMIT. `connectors-build gate --msrv` exit 0.

**Coordinator decision:** the 39 markers in `adapters/mcp/spec/ess` stay `UNMAPPED:`.
`story:mcp-domain-model` (implemented) requires them in its acceptance, four tests enforce them
(`mcp_domain_model_census.rs`, `mcp_domain_model_adversary_pass2.rs`,
`mcp_profile_selection_matrix.rs`, `mcp_inbound_local_binding.rs`) and `selection.md` states them.
Changing that contract is outside this story.

Follow-ups filed as drafts: `story:multi-tenant-principal-assignment`,
`story:session-connection-binding`, `story:audit-anchor-co-presence`,
`story:identity-views-for-witnessing` (views so the two new wrong-state refusals and 17 type
invariants get scenarios; today they give ESS-SYNTH-001 and ESS-SYNTH-013).
