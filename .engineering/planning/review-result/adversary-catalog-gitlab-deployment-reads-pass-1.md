---
format: aep.planning-md/3
id: review-result:adversary-catalog-gitlab-deployment-reads-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: catalog-gitlab-deployment-reads'
relations:
- reviews: story:catalog-gitlab-deployment-reads
revision: 1
---
unit: story:catalog-gitlab-deployment-reads, worktree wave1001c-deploy at 73b643041 plus the untracked adversary file
verdict: nothing found
cases: executed 315→322, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 6 paths (<scratch>/adv-suite-{1..5}.log and the scratch TMPDIR)
needs-coordinator: none

Cases (adapters/catalog/tests/gitlab_deployment_reads_adversary.rs), all green: integer id alone sends the bare route
with no pinned defaults; missing id refused before any request; per_page decimal-text edges ("1", "100", "001", "0100"
sent; "0", "-0", "101", "+1", " 1", "1e2", null, true, [10], 100.0, u64::MAX, i64::MIN refused unsent); body returned
unchanged (deployable null, non-ASCII environment, id u64::MAX, unknown keys, empty page); unreadable project (401, 403
JSON/empty, 404 JSON/HTML) answers the same code as six other GitLab list reads; misspelt filters refused, not
dropped; declared once as a read with only id required.

Suite: `cargo test -p connectors-catalog-provider --no-fail-fast` run 5 times: EXIT=0 each, 322 passed; the earlier
one-off failure did not recur. clippy and fmt clean.

Not broken: parameters match the pinned document (yaml:63259); per_page bounds; paging stops; body unchanged;
refusal parity; re-pinned digest is the observed value (gateway_prefix.rs hashes the printed bootstrap); rewritten
base-pin case keeps exactly the base's 18 ids in order and the base digest (stricter than before).

```findings
[]
```
