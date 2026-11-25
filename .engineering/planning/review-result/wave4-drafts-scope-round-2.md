---
format: aep.planning-md/3
id: review-result:wave4-drafts-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdicts: the MCP draft `needs-revision`, the ess_model draft `needs-revision`, the
provider-budget draft `needs-revision`. Read at commit 7ac4c2086. No build or test was run.

## Round-1 findings

All 8 are resolved but one, which is resolved in part: the ess_model draft's list of other
callers of `ess generate` now names six files and a destination and is still not complete
(finding 4 below).

## New findings

MCP draft:

1. warning: the control and the on-time test of `apps/connectors/tests/security_mcp_deadline.rs`
   share `paused` (line 758), `spawn` (line 618) and `Served::server` (line 426). Neither the
   Change nor the Scope names them, so the way the on-time case alone gets the variables is not
   stated. Change: name the three in Scope; `paused` takes the variables as a parameter and the
   other callers of `server` get none.
2. warning: Change item 4 sets the variables for every test that uses `served()`, and
   acceptance 5 starts a server at 19,999 ms with no variable, but `Peer::start`
   (`apps/connectors/tests/mcp_tool_call_shapes_adversary.rs:317`) takes no parameter and
   `served()` writes `REQUEST_MS` (line 299). Change: `served` takes the budget and `Peer::start`
   takes the variables.
3. warning: "The four tests … shortened" is wrong for the long-read test (7.1 s, then 6.7 s, not
   shortened) and leaves it and the control (6.3 s), both over 5 s, with no destination. Change:
   name both as left, with "no story".

ess_model draft:

4. warning: the list of other tests that call `ess generate` omits
   `crates/connectors-build/tests/catalog_{guard,walk,binary,amendment,identity}_model.rs` and
   `fresh_checkout_regeneration.rs`. Change: add them with the same destination.

Provider-budget draft:

5. warning: item 7 does not say whether `tower-layer` and `tower-service` are workspace
   dependencies or carry a version in `crates/connectors-host/Cargo.toml`, while acceptance 7
   allows four files. Change: the local form, as `libc` there; the root `Cargo.toml` does not
   change.

## Checked, no finding

- MCP draft: every place that writes the budget or starts the server is in a file in scope;
  `WALK_REPLY_RESERVE` and `walk_deadline` are used in `projection.rs` alone;
  `apps/connectors/Cargo.toml` depends on `connectors-sdk` already.
- ess_model draft: one file is enough; the four dev-dependencies exist; 4 source files.
- Provider-budget draft: the six callers of `send` go through the one `send`; the read POST is
  the only other place that sets the timeout from the budget, with `upgrade`; one production
  `Client::builder()`; the three call sites of `with_provider_budget` need no change.
