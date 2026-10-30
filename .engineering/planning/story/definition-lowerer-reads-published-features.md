---
format: aep.planning-md/3
id: story:definition-lowerer-reads-published-features
kind: story
status: implemented
title: The definition lowerer reads rate_limit_headers, query_fixed, query_folds and embedded_text
summary: A published GitHub definition lowers, and install admits exactly what the lowerer reads (D247)
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:14:34Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "proposed", to: "active", at: "2026-10-30T03:14:34Z", actor: "human:timo", revision: 3, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "active", to: "implemented", at: "2026-10-30T10:11:27Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
---
## Outcome

The definition lowerer reads the four fields a published connectors-definitions release uses and
the lowerer refused until now: `rate_limit_headers` on `defaults`, and `query_fixed`,
`query_folds` and `embedded_text` on an operation. connectors-definitions 0.5.2's
`connectors/github.yaml` lowers and loads. `connectors definitions install` admits exactly the
features the lowerer reads (D247): the two lists are held equal by a test, not by prose.

Wave D243-C1, unit C0. Design: the credential-free agent pod design of 2026-10-30, story D1 of
its section 6 needs this to load at all.

## Scope

- `crates/connectors-catalog/src/definition.rs`: `Defaults` (about line 485) and `Operation`
  (about line 712) gain the four fields; each reaches the engine's selection as written, and an
  operation's own value replaces the default's.
- The engine already reads all four in a selection (`adapters/catalog/src/lib.rs`,
  `query_fixed`, `rate_limit_headers`, `embedded_text`, `query_folds`); nothing changes there.
- The FEATURES list of `crates/connectors-definitions-install` (wave
  20261029-definitions-runtime) and its doc comment, CHANGELOG line and
  `contracts/cli/v1alpha1/semantics.md` sentence that say the lowerer does not read the four yet.

## Acceptance (named tests)

In `cargo test -p connectors-catalog`:
- `defaults_rate_limit_headers_reach_every_operation`
- `an_operation_rate_limit_headers_replace_the_default`
- `query_fixed_reaches_the_selection`
- `query_folds_reach_the_selection`
- `embedded_text_reaches_the_selection`
- `the_released_github_definition_lowers`: connectors-definitions 0.5.2 `connectors/github.yaml`,
  byte for byte, lowers against its pinned OpenAPI source, and every one of its 10 operations is
  in the lowered selection.

D247, in a test target that links both crates:
- `install_features_are_the_features_the_lowerer_reads`: the install FEATURES list equals the
  lowerer's list of read features, and a definition using each listed feature lowers.

Evidence, not a test: `probe.sh` of the D243-D1 scratch directory against the released
`github.yaml` answers an operation-lookup refusal, not a definition refusal.
