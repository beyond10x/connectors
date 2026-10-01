---
format: aep.planning-md/3
id: review-result:adversary-gitlab-commit-reads-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: catalog-gitlab-commit-reads'
relations:
- reviews: story:catalog-gitlab-commit-reads
revision: 1
---
unit: story:catalog-gitlab-commit-reads, commits fcc47d696 + 496becbc7 plus my one uncommitted test file
verdict: CONFIRMED (1 warning, 1 note; neither one is a red case)
cases: executed 306→310, red 0
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 3 paths (adv2-case.log, adv2-suite.log, adv2-clippy.log under wave-20261001b/gitlab/scratch)
needs-coordinator: none

Cases (adapters/catalog/tests/gitlab_commit_reads_adversary_pass2.rs), all green: untried parameters declared with the
pinned types (path/author strings; all/with_stats/follow/trailers booleans; from_project_id integer; unidiff boolean);
untried parameters sent verbatim and 8 mistyped values refused with nothing sent; project id reaches the wire as one
encoded segment (org/sub/project → org%2Fsub%2Fproject; ref_name=release%2F1.0; pre-encoded org%2Fproject →
org%252Fproject; . and .. refused); a compare body of exactly 4 MiB is read with compare_timeout visible, 4 MiB + 1
byte gives ErrorCode::Capacity.

Suite: `cargo test -p connectors-catalog-provider --no-fail-fast` EXIT=0, 310 passed, 0 failed, 10 ignored; clippy
EXIT=0; fmt EXIT=0.

Correction 496becbc7 checked: all five re-pinned cases assert the full sorted query and name an existing story; none
weaker than pass 1's version; guide row accurate.

```findings
- file: docs/local-catalog-provider.md
  line: 81
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the compare row tells callers to read body.compare_timeout, but a compare body over the 4 MiB response limit reaches the caller as a capacity error with no body, which the row does not mention
- file: crates/connectors-host/src/http.rs
  line: 252
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: a project id pre-encoded as the pinned document describes it is encoded a second time (org%252Fproject), but no exposed description or guide text tells a caller to pre-encode
```
