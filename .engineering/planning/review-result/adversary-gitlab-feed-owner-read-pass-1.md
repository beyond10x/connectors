---
format: aep.planning-md/3
id: review-result:adversary-gitlab-feed-owner-read-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the GitLab feed binding and owner-read JSON answers
relations:
- reviews: story:gitlab-feed-binding
- reviews: story:owner-read-answers-json-value
revision: 1
---
unit: U2+U3 story:gitlab-feed-binding, story:owner-read-answers-json-value
verdict: red
cases: executed 383→386, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: ~/.cache/conn-adv-u23 (journey TMPDIR, deleted)
needs-coordinator: yes

Findings cover tree `cab270ebe` plus my test edits. Only test files changed: `git diff --stat` shows `adapters/catalog/tests/local_runtime.rs` +9 (a stand-in route, `fixture-deep-<n>`) and `local_runtime/cli_journey.rs` +106, plus the new file `adapters/catalog/tests/gitlab_feed_u23_adversary.rs`.

| # | file:line | what breaks | story | origin | failing test |
|---|---|---|---|---|---|
| F1 | `adapters/catalog/src/feed.rs:575`, `contracts/feed/v1alpha1/gitlab.md:117` | GitLab shows the project (200) but refuses its merge request list (403). `feed.items` then answers `forbidden`. The family's error table says a container the connection cannot read is `not_found`. | gitlab-feed-binding | introduced | `gitlab_feed_u23_adversary.rs:75` (red) |
| F2 | `crates/connectors-host/src/local/owner/supervisor.rs:924`, `owner/transport.rs:549` | The adapter child admits a read result up to 64 levels deep. The owner now nests that result inside its answer (one level more), and the CLI limits the whole answer to 64 levels. So a 64-level result fails with `service_failure` / `upstream_protocol` and the advice `retry_explicitly`. A 63-level result passes. Before this unit the result travelled as a string, one level deep. | owner-read-answers-json-value | introduced | `cli_journey.rs:1262`, ignored journey (red); `cli_journey.rs:1226` (green) shows the child admits 64 levels and refuses 65 |
| F3 | `contracts/feed/v1alpha1/gitlab.md:9`, CHANGELOG | Acceptance item 6 is not met. The sandbox is running (`docker ps`: `connectors-gitlab-20260912 Up 3 weeks (healthy)`; `curl -sk https://localhost:8929/api/v4/version` returns 401). Yet the unit says "not yet read from a running GitLab" and records no transcript. | gitlab-feed-binding | introduced | none (acceptance) |

**What reaches each one:**
- **F1:** `gitlab.md` says `feed.items` reads any project the token can see, member or not. A listed member project with merge requests disabled also reaches it. That GitLab answers 403 there is an assumption from GitLab's `authorize! :read_merge_request`, not something I observed. The sandbox could settle it.
  - Possible fix: after a successful lookup, treat a 403 from the item list (one that is not a quota refusal) as `not_found`, and correct `gitlab.md:117`.
- **F2:** generic reads pass the provider body through (`body: {}`). I found no provider that answers 63 levels deep, so the verdict is INFEASIBLE.
  - Possible fix: check the depth of `result` alone, or allow the owner's answer one extra level.

**Red output captured before any suite run:**
- F1: `assertion left == right failed: a container the connection cannot read answered Forbidden: provider refused the request  left: Forbidden right: NotFound`
- F2: `a 64-level result the child admits is refused: {"error":{"code":"failure","data":{"code":"service_failure","kind":"operational","next_action":"retry_explicitly","service_code":"upstream_protocol","stage":"dispatch"}},"ok":false}`

**Suite runs, all after the cases existed:**

| command | passed | failed | ignored | exit |
|---|---|---|---|---|
| `cargo test -p connectors-catalog-provider --no-fail-fast` | 385 | 1 (F1) | 25 | 101 |
| same, `-- --skip adversary_u23` (my three cases left out) | 383 | 0 | 24 | 0 |
| `-p connectors-build -- feed` (GitLab conformance runs 26 of 28 scenarios) | 27 | 0 | 0 | 0 |
| `-p connectors-host` | 383 | 0 | 31 | 0 |
| `-p connectors` | 115 | 0 | 6 | 0 |
| the unit's two ignored journeys (feed journey, custody restart) | 2 | 0 | n/a | 0 |

The known flake did not trip.

**Coordinator needed:**
- Running F1 against the real sandbox and producing F3's transcript both need sandbox credentials, which are outside my bound.
- My new ignored journey is not registered in `crates/connectors-build/src/ignored.rs`. It belongs under Disposable with the Secret Service and CLI prerequisites.

**Tried and could not break:**
- **Last-key paging:** a full page with no last key is `unavailable`. `id_after` cannot be injected: the cursor is a sealed token and path segments are percent-encoded.
- **Dropped query filters:** removing `membership`, `state=all` or `scope=all` fails the exact-query asserts in `gitlab_feed.rs` and the journey.
- **Ties across a page boundary:** merge requests sharing one millisecond, reordered by GitLab on the next request, lose and repeat nothing (my probe at `gitlab_feed_u23_adversary.rs:132` is green).
- **Kind declarations:** `kind` and `kind_word` together or both missing, an empty word, a word with a space, and one over 128 bytes are all refused.
- **Visibility and revision pins:** without a rule every project is private; the moved pins and the unit's journeys pass.
- **U3 result values:** duplicate keys and non-JSON are `unavailable`. `arbitrary_precision` is on in the unified build (`cargo tree`), so large numbers survive. A string result where the schema names an object is refused.
- **Not tested:** a cursor that does not advance would loop, but only if GitLab ignored `id_after`, and the pinned bundle declares it.

```findings
[
 {"file":"adapters/catalog/src/feed.rs","line":575,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F1 gitlab-feed-binding: after a successful project lookup, a 403 on the merge request list answers forbidden where the family's error table requires not_found for a container the connection cannot read, and gitlab.md:117 states the same contradiction."},
 {"file":"crates/connectors-host/src/local/owner/transport.rs","line":549,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"F2 owner-read-answers-json-value: the owner now nests the read result one level deeper (supervisor.rs:924) and the CLI's 64-level depth bound on the whole answer refuses a 64-level result the child admitted, answering upstream_protocol with retry_explicitly; no real provider shown to answer 63 levels deep."},
 {"file":"adapters/catalog/contracts/feed/v1alpha1/gitlab.md","line":9,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"F3 gitlab-feed-binding: acceptance item 6 is unmet; the sandbox connectors-gitlab-20260912 is running (healthy, :8929 answers 401) yet no real-CLI transcript was recorded and the unit states it was not read from a running GitLab."}
]
```
