---
format: aep.planning-md/3
id: review-result:adversary-catalog-selection-excludes-parameters-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: catalog-selection-excludes-parameters'
relations:
- reviews: story:catalog-selection-excludes-parameters
revision: 1
---
unit: story:catalog-selection-excludes-parameters, unit commit 96171aa76 plus one untracked test file, worktree wave1002a-params
verdict: CONFIRMED (2 note-level doc/compatibility findings; no red case)
cases: executed 324→332, red 0
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (<scratch>/suite.log)
needs-coordinator: the CHANGELOG lines in finding 2, because the unit could not write them

Cases (adapters/catalog/tests/selection_withhold_adversary.rs), 8, all green: no spelling or value of the keyset
parameters reaches commits.list (13 key spellings, 7 values; injected `&pagination=keyset` in ref_name stays one
value); the published descriptor (real binary, --print-local-bootstrap) lists neither; a keyset input to the adapter
process is invalid_input; a name declared as query and header is withheld from both; a withheld repeated parameter is
refused in every form; load refuses a withheld name shared with a path parameter, case variants and `query:` prefix;
every guard reference to a withheld name is refused at load; a withheld write parameter reaches neither preflight nor
write.

Suite: `cargo test -p connectors-catalog-provider --no-fail-fast` EXIT=0, 332 passed, 10 ignored; gateway_prefix 7
passed (re-pin is the observed value); clippy and fmt clean.

Not broken: path-parameter withhold refused at load; withhold with required or bounds refused; no template defaults;
names and values encoded (crates/connectors-host/src/http.rs:252-277); api_base with a query refused; configuration and
descriptor revisions both change (adapters/catalog/src/local.rs:738-750); empty withhold serialises unchanged.

```findings
[{"file": "docs/local-catalog-provider.md", "line": 67, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the paging paragraph still promises every declared query parameter is accepted by name, which commits.list's withheld pagination and page_token now contradict"},
 {"file": "adapters/catalog/providers/gitlab/operations.json", "line": 47, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "a binary older than this change rejects the whole GitLab operations_file on the unknown withhold field, and the refusal of pagination=offset is a breaking change with no CHANGELOG line yet"}]
```
