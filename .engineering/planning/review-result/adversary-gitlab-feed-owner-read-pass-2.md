---
format: aep.planning-md/3
id: review-result:adversary-gitlab-feed-owner-read-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the GitLab feed binding and owner-read JSON answers
relations:
- reviews: story:gitlab-feed-binding
- reviews: story:owner-read-answers-json-value
revision: 1
---
unit: U2+U3 story:gitlab-feed-binding, story:owner-read-answers-json-value
verdict: red
cases: executed 387→390, red 1
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: no

Pass 2 found one defect: a note-level gap in the 403→`not_found` correction. It does not need to hold the unit. The U3 depth correction held under every angle you listed. Findings cover tree `3d50a0740` plus one new test file.

**1. Diff.** `git diff --stat` is empty. The only change is the new untracked test file `adapters/catalog/tests/gitlab_feed_u23_adversary_pass2.rs`. No implementation file was touched.

**2. Cases added**

| case | asserts | now |
|---|---|---|
| `…refused_project_and_an_unknown_one_answer_alike_whatever_the_watermark` | with a foreign or malformed watermark, a project GitLab shows but whose merge requests it refuses (200, then 403) answers exactly as an unknown project (404) | red |
| `…a_403_on_the_project_itself_stays_forbidden` | `gitlab.md`: a 403 on the project lookup stays `forbidden`, and the merge request list is never read | green (probe) |
| `…a_refusal_on_a_resumed_page_is_not_found_and_a_401_stays_unauthorized` | a 403 on a resumed page is `not_found`; a 401 there stays `unauthorized` | green (probe) |

Red output, from running that case alone before any suite run:
`left: (StaleCursor, "watermark or cursor was not issued for this read") right: (NotFound, "container not found")`. The recorded provider reads were `[["projects","1001"]]`, so the merge request list was never reached.

**3. Suite runs (both after the cases existed)**

| command | passed | failed | ignored | exit |
|---|---|---|---|---|
| `cargo test -p connectors-catalog-provider --no-fail-fast` | 389 | 1 (F4) | 25 | 101 |
| same, `-- --skip adversary_u23_pass2` (my 3 cases left out) | 387 | 0 | 25 | 0 |
| `cargo test -p connectors-host --lib -- read_answer depth` | 3 | 0 | 0 | 0 |

**4. Findings**

| # | file:line | what breaks | story | origin | failing test |
|---|---|---|---|---|---|
| F4 | `adapters/catalog/src/feed.rs:785`, `gitlab.md:120` | The watermark is checked after the project lookup and before the merge request list. So with a foreign or malformed watermark, a refused project answers `stale_cursor` while an unknown one answers `not_found`. The family table says these two are "indistinguishable", and `gitlab.md` says the refused project answers "exactly as" an unknown one. Both statements are false. | gitlab-feed-binding | introduced (by 3d50a0740) | `gitlab_feed_u23_adversary_pass2.rs:142` |

- **What reaches it:** any caller that sends a stale, foreign or malformed watermark. What leaks is that the project exists, which the same token can already learn from GitLab's `GET /projects/:id`. So it is contract drift, not a real exposure.
- **Fix:**
  - **Option 1, move the watermark check:** check the watermark before the project lookup. Then all three cases answer `stale_cursor` alike, and a stale watermark costs no provider request.
  - **Option 2, correct the wording:** narrow "exactly as" in `gitlab.md` and §3.3.

**5. Attacked, not broken**
- **403 on the project lookup:** stays `forbidden`, because the mapping only covers the merge request list.
- **403 on a later page:** every page re-runs the lookup first, so it is `not_found`; a 401 there stays `unauthorized`.
- **Refusal raised by the host:** the host never marks a `forbidden` as the provider's own answer (`http.rs` marks only timeouts), so it keeps its code.
- **403 that is a rate limit:** the feed engine has no rate-limit reasons to recognise one (`feed.rs:578`), so no 403 is ever `rate_limited` there. GitLab rate-limits with 429, and only GitLab declares a feed.
- **`read_answer()`:** refuses a deep member other than `result`, a 65-level result, and an answer that is not an object; other owner answers still stop at 64 levels. Duplicate `result` keys are refused when the JSON is read. The `error` answer path is unchanged.
- **Depth helper:** brackets inside strings, and escaped quotes and backslashes, are skipped correctly.
- **Write path:** unchanged. Write answers are still held to 64 levels, and the CLI's result check skips any answer that carries `mutation`.
- **`.` and `..` as container ids:** the host refuses them as `invalid_input` before any request.

**6. Written outside the worktree:** none. Scratch was `<worktree>/.local/tmp/u23-adv2`, now deleted. No keyring journey ran, so no `~/.cache` directory was made. Session lease `adv2-u23-20261007` was taken and released with `worktree hook`.

```findings
[
 {"file":"adapters/catalog/src/feed.rs","line":785,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"F4 gitlab-feed-binding: the watermark is opened after the lookup and before the item listing, so a refused project answers stale_cursor where an unknown one answers not_found for the same foreign or malformed watermark, contradicting the family's 'indistinguishably' and gitlab.md:120 'exactly as'."}
]
```
