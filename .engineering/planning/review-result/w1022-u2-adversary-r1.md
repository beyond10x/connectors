---
format: aep.planning-md/3
id: review-result:w1022-u2-adversary-r1
kind: review-result
status: active
title: 'Adversary pass 1, wave 20261022 U2: first-run docs path'
summary: 'break: 1 blocker, 4 warnings, 3 notes; all addressed in 558bf99b8'
relations:
- reviews: story:docs-first-run-path
revision: 1
---
Adversary pass 1 on the U2 working tree (base `6f44d34b1`). **Verdict: break.** 1 blocker, 4 warnings, 3 notes. 4 cases were added in `crates/connectors-docs/tests/adversary_first_run_path.rs`; 3 fail.

| severity | path:line | finding | disposition |
|---|---|---|---|
| blocker | website/docs/guides/set-up-the-local-cli.md:22-26 | The page describes the state default instead of giving `~/.local/state/connectors` literally. The literal is required by the acceptance and refused by the public-output audit (`crates/connectors-build/src/docs.rs:293-318`). | Narrow the audit to admit exactly that literal, with a unit test; the page gives the path. |
| warning | README.md:187, website/docs/connectors/zendesk.md:26,93 | The README says each page lists its operations, but the Zendesk page omits `ticket.update`, `ticket.note` and `ticket.draft_reply` and says it has no write. | The page lists every shipped operation. |
| warning | README.md:235-236 | Two MCP decision-blockers are named as open; the store shows both cleared. | README and `website/docs/connectors/mcp.md:72` state the store's state. |
| warning | crates/connectors-docs/src/vocabulary.rs:384 | The custody-binding test survives 3 of 4 README mutants. | The test fails when any platform's binding is missing. |
| warning | README.md | The cut lost four facts: services advertise no writes, the Windows build-from-source route, the macOS and Windows guarantee links, and the `local-runtime-foundation.md` link. | Restored once, on the README or a site page. |
| note | contracts/cli/v1alpha1/semantics.md:9 | The link to `README.md#where-the-project-stands` no longer reaches a slice description. | Repointed or reworded. |
| note | crates/connectors-docs/src/status.rs:150, docs/catalog-slack.md:874 | Both links reach the `#after-an-upgrade` stub. | The coordinator applies U2's anchor patch at integration. |
| note | website/docs/guides/author-a-definition.md:271 | The `auth_library` row misstates the rule. | Reworded to `definition.rs:1615-1645`. |

The operation counts (20 rows), the install block (run against 0.56.0), the bundle sha with `--amendments`, the six refusal names and the clock facts all held.
