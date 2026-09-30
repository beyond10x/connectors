---
format: aep.planning-md/3
id: review-result:adversary-legacy-commands-pass-1
kind: review-result
status: active
title: Adversary pass 1 on legacy command parsing
relations:
- reviews: story:legacy-commands-parse-like-the-contract
revision: 1
---
unit: story:legacy-commands-parse-like-the-contract, uncommitted tree wave0930b-legacy on 62aec7648
verdict: CONFIRMED
cases: executed 35→39, red 1
origin: introduced 0 / pre-existing 1 / undecided 0
wrote-outside-worktree: scratch/adversary probe files
needs-coordinator: the story text promised `-o`

Cases in apps/connectors/tests/adversary_legacy_edges.rs: root help after a global --output carries the service
block (red), --output after -- does not select JSON (green), non-UTF-8 and long argv (green), routing edges (green).
Could not break: argv echo in any form, --version/help after a legacy word, separators, stdout empty on errors, exit
codes, serve --config parsing with and without the prefix, contract commands' parsing.

Coordinator routing: finding 1 fixed in the unit (root help detected after leading global options, local.rs:20);
finding 2 is covered by the adversary's separator case; finding 3 corrected in the story text.

```findings
[
  {"file": "apps/connectors/src/local.rs", "line": 20, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "Root help omits the Explicit service commands block whenever a global option precedes --help, contradicting the unit's new semantics.md claim that root --help prints it."},
  {"file": "apps/connectors/src/legacy.rs", "line": 66, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The unit's tests do not cover the -- stop in parse_refusal; the added separator case now covers it."},
  {"file": "contracts/cli/v1alpha1/semantics.md", "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Story Decided text promises -o before the legacy word; the implementation and contract accept only --output."}
]
```
