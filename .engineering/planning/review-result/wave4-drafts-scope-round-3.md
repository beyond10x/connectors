---
format: aep.planning-md/3
id: review-result:wave4-drafts-scope-round-3
kind: review-result
status: active
title: Scope critic, round 3, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdicts: the MCP draft `approve`, the ess_model draft `needs-revision`, the provider-budget draft
`approve`. Read at commit 304f24f16. No build or test was run.

## Round-2 findings

All 5 are resolved: the MCP draft names `paused`, `spawn` and `Served::server`, says that `served`
takes the budget and `Peer::start` the variables, and describes the four tests of the seven-tests
story as they are; the ess_model draft lists 12 other test files that start `ess generate`, which
a search over `crates`, `apps` and `adapters` confirms; the provider-budget draft says the two
tower crates carry their version in the crate's manifest.

## New findings

ess_model draft:

1. warning: the "Not in this story" line on
   `the_model_admits_exactly_the_read_profiles_the_executable_admits` names a test whose time
   stays and gives neither a destination nor "no story". Change: end it with "no story".
2. warning: the line on the 12 other files says their calls belong to
   story:where-do-the-1298-s-of-the-tests-under-5-s-go, whose first question reads four other
   binaries and names neither `ess generate` nor these files. Change: say "no story", or say that
   the question is extended to them.

## Checked, no finding

- MCP draft: 4 of 4 promises of the operator's decision are claimed; the comparison is at
  `apps/connectors/src/mcp/projection.rs:311`; scope and frontmatter name three files.
- ess_model draft: 3 of 3 promises of item 9 of the open-question story are claimed; 16 of the 17
  tests call `schema`; one file is in scope.
- Provider-budget draft: the one finding handed over by the independent read of the five-tests
  story is claimed; scope and frontmatter name the same four files.

Out of lane: the provider-budget draft has no test for the read POST, for a pooled connection, or
for the loss with less than the lead left; story:mcp-outbound-stdio-runtime (draft) carries
`Cargo.lock` as an inferred scope.
