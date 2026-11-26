---
format: aep.planning-md/3
id: review-result:wave4-drafts-acceptance-round-3
kind: review-result
status: active
title: Acceptance critic, round 3, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdicts: the ess_model draft `needs-revision`, the provider-budget draft `approve`; the approval
of the MCP draft from round 2 still holds. Read at commit 304f24f16. No build or test was run.

## Round-2 findings

- ess_model draft, finding 1 (nothing observes that the 16 tests read the shared directory):
  resolved in part. Acceptance 6 holds the diff property, and its sentence can fail on the pull
  request's own new tests (finding 2 below).
- ess_model draft, finding 2 (the child test and its bound): resolved.

## New findings

ess_model draft:

1. warning: acceptance 5, "removes no line that contains `assert`", fails on a correct pull
   request: the helper's `assert!(status.success(), …)` (`adapters/sql/tests/ess_model.rs:36`)
   moves under the lock and shows as a removed line. Change: limit it to lines of `#[test]` and
   `#[tokio::test]` functions.
2. warning: acceptance 6, "no call of `schema` passes a temporary directory", is false for the two
   new tests, which call the helper with a `tempfile::tempdir_in` parent directory. The story does
   not say whether the function the new tests call is `schema` or a second function. Change: name
   the function.

Provider-budget draft: no finding. Acceptance 1 turns on `upstream_answer`; the bound of 2 s guards
against a connect that is not cut. Acceptances 2 and 3 would go red for a layer that bounded more
than the connect.

MCP draft: its acceptance statements did not change between 7ac4c2086 and 304f24f16; the edited
item 4 and Scope match the two test files.

## Verified in source

- reqwest 0.13.5: `connector_layer` is not feature-gated (`src/async_impl/client.rs:2475-2481`);
  `connect_timeout` is the outermost layer (`connect.rs:141-155`); `is_timeout` accepts an
  `io::Error` of kind `TimedOut` (`error.rs:140-141`); `PendingRequest::poll` polls the total and
  read timers before the request and returns a timeout that is no connect error
  (`client.rs:3096-3113`).
- hyper-util 0.1.21: a connector error becomes a connect error (`client.rs:517`); the connector is
  called inline on the caller's task (`client.rs:216-238`, `416-448`).
- `Failure::from_provider` maps a timeout with `upstream_answer` false to `Failure::Timeout`
  (`crates/connectors-host/src/local/runtime.rs:524`).
- ess_model: 17 tests, 16 call `schema` at 18 call sites; 4 files under `adapters/sql/spec/ess`.

## Not established

- ess_model acceptance 3 does not say which byte it changes; a byte that makes a source invalid
  makes `ess generate` fail. A byte inside a comment is safe: the three domain files hold comment
  lines.
- Out of lane: the provider-budget draft still says "Inferred, not read in reqwest's source" of a
  fact that is read at `client.rs:3096-3113`.
