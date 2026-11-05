---
format: aep.planning-md/3
id: review-result:connectors-wave-2026-11-04-u1-adversary
kind: review-result
status: active
title: Adversary pass on the MCP revision rule (wave 2026-11-04)
relations:
- reviews: story:claude-connects-to-the-mcp-server
revision: 1
---
## Adversary pass: unit u1-mcp-revision (claude-connects-to-the-mcp-server, revision rule)

Verdict: holds. Added 6 cases in `apps/connectors/tests/local_mcp_server_revision_adversary.rs`, all
passing on b3adde687; committed as a3f00d6ff. Pre-existing findings: none.

Could not break: what counts as absent (a non-object `_meta` is `-32602`; only the exact member name is a
revision; duplicate keys are `-32700`), binding before and after listing and after a tool is hidden, both
protocol revisions, the refusal precedence (an absent revision meets every refusal a current one meets),
and the checker (a `revision_current: null` row with a wrong outcome is caught).

Findings (all introduced, notes):

1. `adapters/mcp/contracts/server/v1alpha1/mutations.md:23-25`: still says a mutating tool request carries
   its exact current revision, while `projection.md` now lets a tool call bind with none. Mutating tools
   are not advertised. Not resolved in this wave: which rule a mutating tool follows is a design choice
   the operator's option A did not make; filed as a draft story tagged from-finding.
2. `docs/local-mcp-stdio.md:85-86`: did not say the revision is optional or what omitting it costs.
   Closed by 10ce2d4ca.
3. `adapters/mcp/contracts/server/v1alpha1/projection-cases.json:463-477`: one `revision_current: null`
   row; none for an absent revision on a hidden tool (`not_found` versus `stale_description`). Held at
   runtime by `an_absent_revision_binds_to_the_current_advertisement_not_the_listed_one`; no case row
   added.
4. `adapters/mcp/contracts/server/v1alpha1/projection.md:127-131`: said nothing about a `_meta` that is not
   an object. Closed by 10ce2d4ca (code refuses `-32602`, `projection.rs:731`).

Checks: local_mcp_server_revision_adversary 7 passed; clippy on that test target `-D warnings` exit 0;
fmt exit 0.
