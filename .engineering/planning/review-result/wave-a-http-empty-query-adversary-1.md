---
format: aep.planning-md/3
id: review-result:wave-a-http-empty-query-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on http-empty-query-no-trailing-question-mark (wave A)
relations:
- reviews: story:http-empty-query-no-trailing-question-mark
revision: 1
---
```
unit: story:http-empty-query-no-trailing-question-mark
verdict: green
cases: executed 125→125, red 0
origin: introduced 0, pre-existing 1, undecided 0
wrote-outside-worktree: <scratch>/units/http-empty-query-no-trailing-question-mark/scratch/adv-{host,catalog,catalog2}.log
needs-coordinator: yes
```

The fix holds and both Acceptance points are met. The coordinator needs to check the branch base before merging.

**Branch base (coordinator).** The brief names base `881139ac3`, but that commit is not an ancestor of HEAD. HEAD's parent is `9069c43a4`, and the two branches fork there. `git diff 881139ac3..HEAD` shows 851 deleted lines this unit never touched: `.github/workflows/macos-host.yml`, `crates/connectors-custody/src/keychain.rs`, `artifact/macos.rs`, `grafana/tests/local_runtime/cli_journey.rs` and `Cargo.lock`. I reviewed `HEAD~1..HEAD` (33 files) as the unit. Merge it, or rebase it onto 881139ac3, so the macOS round-1 work is not reverted.

**What I checked**
- **The fix:** `request_url` in `crates/connectors-host/src/http.rs:583` now adds a query only when there are pairs.
  - It is the only place a request URL gets a query. `get`, `get_prefix`, `send_json`, the trailing-slash `send_get` and the POST at line 932 all go through it.
  - `post_form` and `post_assertion` never added a query, so they are unchanged.
  - The configured base cannot carry a query (rejected at line 192), so skipping the call loses nothing.
- **Mutants:**
  - Removing the guard turns `empty_query_sends_no_question_mark` red. The base's own lib test asserted `POST /search?`, which confirms that base behaviour.
  - Dropping pairs turns red on the `GET /api/list?page=a+b` line.
  - The test covers read, read prefix, a read with a query, and a write.
- **Other callers:** the OAuth authorize URLs (`connectors-oauth2/src/client.rs:151`, `local/oauth.rs:218`) always add pairs, so they are not affected.
- **Changed tests:** every catalog, grafana, kubernetes and tavily change only drops the `?`. I found no weakened assertion.
- **Leftovers:** no `.md`, contract or non-vendor fixture still expects a trailing `?`. `local_runtime.rs:443` (`/issues?`) only matches requests that carry a query.
- **Acceptance bullet 2:** `adapters/catalog/tests/google_drive.rs` is updated, plus 28 other targets.

**Runs** (all exit 0)
- `cargo test --locked -p connectors-host --test http --test http_write --test fixed_header_adversary --test http_service_account_token`: 7, 4, 18 and 5 passed, 0 failed.
- `cargo test --locked -p connectors-host --lib -- http::`: `ok. 17 passed; 0 failed`.
- `cargo test --locked -p connectors-catalog-provider --test google_drive --test slack`: `ok. 21 passed` and `ok. 53 passed`.

**Finding F1 (minor, does not block).** Two checks in `adapters/catalog/tests/slack.rs` (lines 1183 and 2435) still strip a trailing `?` before comparing. They now accept both forms, so they would not catch the `?` coming back. The `/files-pri/` request is a read through ScopedHttp. The check was already there, and the unit left it alone.

I added no test cases and the worktree has no uncommitted changes. The lease is released.

```findings
[{"file": "adapters/catalog/tests/slack.rs", "line": 1183, "category": "test-coverage", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "slack.rs:1183 and :2435 still strip a trailing ? before comparing (trim_end_matches), so they accept both forms and would not catch the ? returning; should be exact equality"}]
```
