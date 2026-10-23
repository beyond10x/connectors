---
format: aep.planning-md/3
id: review-result:claude-epic-panel-r5-20261022
kind: review-result
status: active
title: 'Plan-critic panel, round 5: connectors with Claude'
summary: 3 approve, acceptance needs-revision (1 blocker); addressed
relations:
- reviews: story:docs-first-run-path
- reviews: story:standing-write-approval-for-unattended-agents
- reviews: story:mcp-stdio-runtime-branch
- reviews: story:definition-reader-feed-extensions
revision: 1
---
# Plan-critic panel, round 5: connectors with Claude

**Verdicts: scope approve, parallel-safety approve, design approve, acceptance needs-revision (1 blocker, 7 notes). All are addressed in the revisions after round 5.**

| Lens | Story | Severity | Finding | Disposition |
|---|---|---|---|---|
| acceptance | docs-first-run-path | blocker | README cap 15,000 B while the kept support table (`README.md:284-304`) is 14,992 B alone | the `Implemented operations` cells become a count and a link to the onboarding page; `Connector` and `Binding` columns stay; cap `wc -c README.md` at most 20,000 (38,807 at `fc9d455e1`) |
| acceptance | docs-first-run-path | note | "practical" is at `website/docs/examples/contract-exercises.mdx:14`, not `follow-a-request.mdx:13` | named; scope swaps the two files |
| acceptance | docs-first-run-path | note | `docs/local-clock.md` already exists | after-state: `local-runtime.md` drops `:87-121` and links it; missing facts are added |
| acceptance | docs-first-run-path | note | `author-a-definition.md` line drift | re-cited `:259-271` at `fc9d455e1` |
| acceptance | standing-write | note | the ESS domain line named no file or scenario | `ess/domains/standing_approvals.yaml`, with scope, expiry, revocation and rate scenarios passing above 0 in the gate |
| acceptance | standing-write | note | the adversary pass was outside the Acceptance | acceptance line: an `aep:adversary` review-result relates `reviews:` the story before merge |
| acceptance | standing-write | note | the signature scenario joined two outcomes | split into `a_grant_with_a_changed_byte_…` and `a_grant_signed_by_a_key_the_owner_did_not_issue_…` |
| acceptance | mcp-stdio-runtime-branch | note | "conformance counts do not drop" had no base; `task check` does not exist here | the gate command `cargo run --locked -p connectors-build -- gate`; the report pastes per-suite conformance lines at base and head |
| design | standing-write | note | changed grant bytes vs `mounted_secret` rotation needing revalidate | the ADR decides how a changed or removed grant file is read; the revocation scenario holds it |
| design | standing-write | note | Order said "A, B and C"; the container guide was owned by nothing | Order rewritten; `docs/local-container.md` added to scope with a standing-grant section |
| parallel-safety, design, scope | several | note | scope gaps | docs-first-run-path drops `connectors/mcp.md` and `docs/catalog-slack.md`; mcp-stdio-runtime-branch adds `adapters/README.md`, `.gitignore`, `service_metadata.rs`, `compatibility.json`, `reference/{crates,cli}.md`; feed-extensions adds `connectors/index.md`, `status.json`, `status.md` |

Not changed: story:catalog-feed-engine-extensions stays `active` (outside the set; its 0.56.0 close kept it active). The epic's release cut stays with the repository's release process.
