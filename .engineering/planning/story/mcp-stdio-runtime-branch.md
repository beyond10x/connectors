---
format: aep.planning-md/3
id: story:mcp-stdio-runtime-branch
kind: story
status: active
title: Re-land the read-only MCP stdio server kept on branch unit/mcp-local-runtime-20261003-rebased
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .gitignore
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: adapters/README.md
- confidence: cited
  path: adapters/catalog/src/local.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime
- confidence: cited
  path: adapters/mcp
- confidence: cited
  path: apps/connectors/Cargo.toml
- confidence: cited
  path: apps/connectors/spec/compatibility.json
- confidence: cited
  path: apps/connectors/src
- confidence: cited
  path: apps/connectors/tests
- confidence: cited
  path: contracts/cli/v1alpha1
- confidence: cited
  path: contracts/service/local-operation-curation.md
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-build/src/main.rs
- confidence: cited
  path: crates/connectors-build/src/mcp.rs
- confidence: cited
  path: crates/connectors-build/src/service_metadata.rs
- confidence: cited
  path: crates/connectors-build/tests
- confidence: cited
  path: crates/connectors-core
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/src/local
- confidence: cited
  path: crates/connectors-host/tests
- confidence: cited
  path: docs/development.md
- confidence: cited
  path: docs/local-mcp-cli.md
- confidence: cited
  path: docs/local-mcp-stdio.md
- confidence: cited
  path: docs/local-runtime-foundation.md
- confidence: cited
  path: ess/domains
- confidence: cited
  path: website/docs/reference/cli.md
- confidence: cited
  path: website/docs/reference/crates.md
revision: 40
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T09:48:11Z", actor: "human:timo", revision: 39, decided_on: {"recorded":{"review_outcome":5}}, executor: "agent:connectors-session", correlation: "wave-20261022"}
- {from: "proposed", to: "active", at: "2026-10-22T09:48:11Z", actor: "human:timo", revision: 40, decided_on: {"recorded":{"review_outcome":5}}, executor: "agent:connectors-session", correlation: "wave-20261022"}
---
## Outcome

The production `connectors server --transport stdio` command lists and invokes admitted read-only
tools through the protected local owner, so an MCP client can use Connectors operations directly.

## Where the work is

Branch `unit/mcp-local-runtime-20261003-rebased` on `origin` (head `5140636d9`), 14 commits,
195 files, about 19,000 inserted lines, last rebased onto `main` on 2026-10-05. It was proposed as
pull request https://github.com/beyond10x/connectors/pull/93, closed on 2026-10-08 unmerged
because `main` has moved by many releases since; the branch is kept.

What the branch adds:

- native MCP framing and lease supervision (`adapters/mcp/runtime`, generated launch CLI, launch
  and configuration types, supervision plan);
- audited lossless reads for governed bindings in the host;
- complete operation metadata, modeled and validated, with local operation curation bound to
  executable and bootstrap pins;
- a shared provider deadline across HTTP capabilities, carried through native catalog execution
  and owner admission with the original deadline preserved;
- the stdio server: one adapter and Connection per process, inline schemas, one active owner
  request, protocol revisions `2026-07-28` and `2025-11-25`; disabled, hidden, approval-required
  and ungranted operations omitted or refused before provider work;
- planning records it carried: stories `mcp-local-stdio-runtime`, `mcp-outbound-stdio-runtime`,
  `metadata-sidecar-fixture-isolation`, decision-blockers `helm-execution-family` and
  `mcp-outbound-stdio-process-ownership`, a delivery specification and four plan reviews.

## Acceptance

Each scenario below runs in `cargo test` in the `Rust gate` workflow, not `#[ignore]`d: the
server runs under the `mounted_secret` binding in the jail of `apps/connectors/tests/support/mounted.rs`
(as `apps/connectors/tests/revalidate_transient.rs` does), and the provider is a loopback listener
the test serves, so "no provider request" is the listener's request count staying 0.

`apps/connectors/tests/local_mcp_server.rs`:
- `stdio_lists_and_calls_an_admitted_read_on_2026_07_28`: `initialize`, `tools/list` holds the
  admitted read, `tools/call` returns the listener's answer with `isError:false`.
- `stdio_lists_and_calls_an_admitted_read_on_2025_11_25`: the same on the older revision.
- `a_disabled_operation_is_not_listed_and_its_call_makes_no_request`
- `a_hidden_operation_is_not_listed_and_its_call_makes_no_request`
- `an_approval_required_operation_is_refused_with_no_request`
- `an_ungranted_operation_is_refused_with_no_request`
- `stdout_carries_only_json_rpc_frames`: every stdout line of every scenario above parses as one
  JSON-RPC frame; diagnostics go to stderr.
- `owner_refusal_and_invalid_launch_emit_no_protocol_bytes` (branch `:21`), kept.
- `production_stdio_withdrawal_framing_and_cancellation` (branch `:123`) and
  `production_stdio_stopped_session_cannot_renew_after_expiry` (branch `:176`): the lease
  supervision, moved from the Secret Service to the jail and no longer ignored.

The rest of the branch:
- The branch adds or changes 28 test files (`git diff --stat main...5140636d9` over `*/tests/*`).
  Each is re-landed and passes, or the unit's pull request lists it as dropped with the reason.
  This covers the shared provider deadline and the audited lossless reads, whose tests are among
  them.
- The planning records it carried are re-created through the planning CLI, not copied.

Specification first:
- The branch's changes to `ess/domains/cli.yaml`, `ess/domains/service_wire.yaml` and
  `ess/domains/transport.yaml` land before the code that implements them, re-applied to the
  current files.
- The repository gate (`cargo run --locked -p connectors-build -- gate`) validates them. The unit's report pastes the gate's ESS conformance lines (passed, failed, skipped per suite) at its base and at its head, and no suite passes fewer scenarios at the head.
- `apps/connectors/spec/compatibility.json` keeps `"mcp": "deferred"`; story:claude-connects-to-the-mcp-server
  flips it.

## Order

- The branch's `README.md` change (an MCP claim inside the status block) is not re-landed: the
  status block goes in story:docs-first-run-path, and the MCP support claim is
  story:claude-connects-to-the-mcp-server's. This story does not edit `README.md`.
- No dependency on story:docs-first-run-path; this story can run beside it.
- story:claude-connects-to-the-mcp-server depends on this story and cites the scenarios above.
