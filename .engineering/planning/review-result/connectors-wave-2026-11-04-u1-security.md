---
format: aep.planning-md/3
id: review-result:connectors-wave-2026-11-04-u1-security
kind: review-result
status: active
title: Security review of the MCP revision rule (wave 2026-11-04)
relations:
- reviews: story:claude-connects-to-the-mcp-server
revision: 1
---
## Security review: unit u1-mcp-revision (claude-connects-to-the-mcp-server, revision rule)

Verdict: holds. No case added, nothing built; no invariant found unenforced.

- An absent revision reaches nothing a current one could not: `bind_revision`
  (`apps/connectors/src/mcp/projection.rs:724-741`) returns Ok or a refusal, and every later step uses the
  server's own revision; foreign instance (`:340`), grant (`:343`), limits (`:352-355`), exposure and
  disabled (`:357-361`), not advertised (`:362`) and the owner's re-check (`apps/connectors/src/mcp/mod.rs:67-90`)
  all still apply. Held at runtime by the adversary case
  `an_absent_revision_meets_every_refusal_a_current_one_meets`.
- A mutating tool is not callable by name, at three layers present on the base: `projection.rs:214-229`,
  `crates/connectors-host/src/local/owner/governed.rs:140`, `.../governed/bounded.rs:30-36`.
- The revision never authorizes: every comparison only refuses (`projection.rs:737`, `mod.rs:83`,
  `bounded.rs:21`).
- The revision is per owner and selection, recomputed per request (`mod.rs:34-41`, `projection.rs:711`).

Finding (pre-existing, note): no MCP `tools/call` conformance case calls by name an operation with a
`Write` requirement or non-empty `semantic_effects` while approval is not required; the read-only guard
(`projection.rs:214-229`, `governed.rs:140`) holds by reading only. Filed as a draft story tagged
from-finding.
