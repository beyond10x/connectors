---
format: aep.planning-md/3
id: review-result:adversary-gitlab-commit-reads-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: catalog-gitlab-commit-reads'
relations:
- reviews: story:catalog-gitlab-commit-reads
revision: 1
---
unit: story:catalog-gitlab-commit-reads, unit commit fcc47d696 plus my uncommitted test file
verdict: red
cases: executed 296→306, red 5
origin: introduced 1 / pre-existing 2 / undecided 0
wrote-outside-worktree: <wave-scratch>/gitlab/scratch/{adv-red.log,adv-suite.log,adv-clippy.log}
needs-coordinator: yes. Refusing `pagination=keyset` needs a way to exclude parameters per selection, and `adapters/catalog/src` has none.

Cases (adapters/catalog/tests/gitlab_commit_reads_adversary.rs):

| line | case | now | red output |
|---|---|---|---|
| 140 | `pagination=keyset` is refused | red | `commits.list {"id":"org/project","page":2,"pagination":"keyset","per_page":100} was accepted and sent [... ("pagination", "keyset")]` |
| 153 | `order` outside its enum (`default`, `topo`) is refused | red | `commits.list {"id":"org/project","order":"newest"} was accepted and sent` |
| 167 | `since` that is not a date-time is refused | red | `commits.list {"id":"org/project","since":"yesterday"} was accepted and sent [... ("since", "yesterday")]` |
| 253 | same check on `projects.list` `order_by` (shipped at base) | red | `projects.list {"order_by":"newest"} was accepted and sent` |
| 262 | same check on `projects.list` `last_activity_after` (shipped at base) | red | `projects.list {"last_activity_after":"yesterday"} was accepted and sent` |
| 107, 182, 198, 211, 278 | declared types, `from`/`to` missing or null, compare has no `page`/`per_page`, `per_page` bounds, re-pinned digest | green | — |

Suite: `cargo test -p connectors-catalog-provider --no-fail-fast` EXIT=101; 301 passed, 5 failed, 10 ignored.

Attacked and not broken: declared types and required fields match the pinned document; per_page bounds (1, 100, -0, +5, 1.5, true, null, 1e2, 2^64 string); missing or null from/to refused before any request; page/per_page on compare refused; compare_timeout reaches the caller unchanged; no write operation added; re-pinned digest explained by the two new operations (case 278 restores the base pin when they are removed); `+` in since percent-encoded.

```findings
- file: adapters/catalog/tests/gitlab_commit_reads_adversary.rs
  line: 140
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: commits.list sends pagination=keyset unchanged, which makes GitLab page by a response header the provider drops and breaks the documented short-page walk
- file: adapters/catalog/tests/gitlab_commit_reads_adversary.rs
  line: 153
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: values outside an enum in the pinned document (commits.list order, projects.list order_by) are sent to GitLab instead of being refused as invalid_input
- file: adapters/catalog/tests/gitlab_commit_reads_adversary.rs
  line: 167
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: since/until (and projects.list last_activity_after) are not checked as date-times despite format date-time in the pinned document and the guide's date-times label
```
