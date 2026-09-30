---
format: aep.planning-md/3
id: review-result:adversary-ess-pin-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the ESS 0.45.0 pin
relations:
- reviews: story:ess-pin-newest-release
revision: 1
---
unit: story:ess-pin-newest-release, uncommitted tree wave0929c-ess on 74f09e6d4
verdict: NEEDS-CHANGE
cases: executed 16→17, red 1
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary (about 45 MB); tag refs fetched in the local ess checkout
needs-coordinator: the ess on PATH is 0.44.0, so the gate refuses without CONNECTORS_ESS until the plugin is upgraded

Case crates/connectors-spec/tests/adversary_ess_limit_notes.rs the_observation_id_equality_note_holds_for_the_pinned_ess
(red: ESS 0.45.0 synthesizes the guard the note says it cannot; 0.40.0 refuses it). Could not break: the gate refuses
an unset or mismatched CONNECTORS_ESS; manifest/2 states nothing false; the lock resolves; refusals 24 → 20.

Coordinator routing: findings 1, 2, 3 and 6 to the implementor (notes rewritten to hold under 0.45.0, a host test for
RevisionExhausted with a note naming beyond10x/ess#251, a non-tautological manifest check); finding 5 kept as a note.

```findings
[
  {"file": "ess/domains/execution_audit.yaml", "line": 258, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "ESS 0.45.0 synthesizes the observation_id equality guard the note says it cannot"},
  {"file": "ess/domains/execution_audit.yaml", "line": 252, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "ESS 0.43 (#234) no longer refuses every branch over a stored Optional struct member guard, contradicting the note"},
  {"file": "ess/domains/cli.yaml", "line": 710, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Related-record guard notes give a reason 0.41 when_related removed; the limit now comes from ess/15 and RelatedGuardUnsupported"},
  {"file": "ess/domains/local_approval_policy.yaml", "line": 102, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The pin drops the only conformance scenario that checks RevisionExhausted, and nothing in the repository records or replaces it"},
  {"file": "crates/connectors-spec/src/toolchain.rs", "line": 184, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The pin test never compares the locked commit with the tag and skips .git, query-less and path-patched ESS sources"},
  {"file": "crates/connectors-build/src/metadata_conformance.rs", "line": 1033, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The final assertion of emitted_manifest_is_admitted_by_the_pinned_ess is tautological because emit_suite derives refusals from refused.len()"}
]
```
