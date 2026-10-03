---
format: aep.planning-md/3
id: review-result:mcp-cli-intent-20261003
kind: review-result
status: active
title: Independent review of selected MCP CLI intent
relations:
- reviews: task:mcp-cli-selected-intent-contract
revision: 1
---
unit: task:mcp-cli-selected-intent-contract; frozen cb26j-cli working tree over 0f64816070123f789950ff404ce461cbf5e78ee0
verdict: nothing found
cases: executed 0→0 runtime cases, red 0; bounded document/source checks only
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none authored by this review
needs-coordinator: none for findings; integration and actual runtime work remain coordinator-owned

## 1. Scope and diff

`git --no-pager diff --stat` produced no output. `git status --short` showed only the author's three new untracked source documents:

```
?? adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json
?? adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md
?? docs/local-mcp-cli.md
```

Reviewed all three files, the task acceptance and brief, author report, generated binding and runtime, production local parser caller, current compatibility, shared CLI contract, and owning MCP auth/projection/invocation/local-binding contracts. No source, test, generated output, AEP or commit changes. Review scratch is only `.local/mcp-two-waves/cli-review`.

## 2. Cases added and red boundary

None. This is the explicitly scoped documentation-only pass; no mirrored-prose Rust test, fabricated runtime fixture, Cargo build or runtime suite was appropriate. The author reported document checks, not runtime execution. A static comparison does not establish command execution, OAuth custody, streaming process behavior or conformance.

## 3. Checks and observations

Independent jq comparison against `apps/connectors-cli-contract/binding.json` required exactly one existing command for each of the eight generic paths, exact complete command-specific flag sets, matching process globals, and unresolved MCP disposition. It also required unique documentation IDs, absence of the selected server path from the actual binding, compatibility deferred, stdio-only selected launch/no default, protocol-only stdout/no credential stdin, unresolved local operation/profile/OAuth codec bindings, and the parent journey open. Exit 0. All eight rows and seven document/source boundary observations were true; full output: `static-check.json`.

Independent jq extraction from `ess/domains/cli.yaml`, `ess/domains/service_wire.yaml` and generated `src/runtime.rs` compared the complete three distinct error vocabularies, checked the open integer peer domain and absence of uncertain-effect retry authority. Exit 0; output:

```
{"cli":30,"service":30,"presentation":11,"matches":true,"open_peer_domain":true,"no_uncertain_retry":true}
```

`git diff --exit-code 0f64816070123f789950ff404ce461cbf5e78ee0 -- apps/connectors apps/connectors-cli-contract ess contracts/cli adapters/mcp/spec` exited 0, no output. Neither actual parser/handlers nor shared/native models or compatibility changed.

Source reading checked protected file/stdin/terminal capture against shared CLI semantics; separate business and credential channels; no OAuth implementation inferred from static capture; local client ownership of the inbound child versus the open outbound child decision; unresolved caller assignment; preserved mutation-advertisement withholding; RPC versus resolved tool execution failures; complete safe service envelopes; and finite CLI envelope/exit semantics versus proposed long-lived framing. No contradictory source claim found.

One harmless read-only lookup initially used nonexistent `apps/connectors-cli-contract/cli-binding.json`; it was corrected to actual `binding.json` before comparison. No result was inferred from that failed lookup. Publication polling is a separately assigned task, not review work.

## 4. Frozen source identities and limits

```
3bd5d1bfb4bff269992981370be0208433b520910ef7a9051e9dc27b52b3ebbc  docs/local-mcp-cli.md
444ee5b732deae68142c7af1b49c1cb1c1ed7841bcb01723df823f1c47cee4b3  adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md
f81f3cb36a1b95524853e3dddd2da97c078205083db03ae635ec64b13aa131d1  adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json
```

These match the frozen author handoff. This agent review does not approve the unit or supply independently verified runtime evidence. The three documents can select future spelling and describe source-grounded grammar/error boundaries; they cannot complete the parent working journey. Native launch values, shared/native composition, pinned ESS presentation expressibility, configuration and protected codecs, actual framing and runtime conformance remain explicitly outstanding.

## 5. Findings

Nothing found in the bounded documentation/source attack.

```findings
[]
```
