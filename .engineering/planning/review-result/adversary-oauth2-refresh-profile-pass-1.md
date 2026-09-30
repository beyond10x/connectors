---
format: aep.planning-md/3
id: review-result:adversary-oauth2-refresh-profile-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the OAuth refresh profile
relations:
- reviews: story:catalog-oauth2-refresh-profile
revision: 1
---
unit: story:catalog-oauth2-refresh-profile at 70f734718 (impl/catalog-oauth2-refresh-profile), working tree ~/.local/state/worktree/trees/b10x/connectors/wave0930c-refresh
verdict: NEEDS-CHANGE
cases: executed 36→42, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 (~/.cache/w0930ra, empty TMPDIR)
needs-coordinator: none

**1. Diff stat** (tracked changes only; the new file is untracked)
```
 adapters/catalog/tests/local_runtime.rs | 2 ++
?? adapters/catalog/tests/local_runtime/oauth2_refresh_adversary.rs
```
Both are test files. I added `mod oauth2_refresh_adversary` to `adapters/catalog/tests/local_runtime.rs`. No implementation file was touched.

**2. Cases added** in `adapters/catalog/tests/local_runtime/oauth2_refresh_adversary.rs`, with their own scripted TLS fixture:

| case | asserts | now |
|---|---|---|
| `adversary_write_401_evicts_cached_access_token` | after a write commit refused with 401, the next read exchanges again | **red** |
| `adversary_expired_id_token_refused` | an `id_token` with `exp: 2` fails validation | **red** |
| `adversary_distinct_entries_never_share_a_cached_token` | a second refresh token gets its own exchange and its own access token | green |
| `adversary_token_redirect_is_not_followed` | a token response of 301/302/307/308 gives `Protocol`, with one request and no follow | green |
| `adversary_expires_in_extremes` | `u64::MAX` is capped and cached; `0`, `-1`, `"3599"`, `3599.5` and `null` give `Protocol` | green |
| `adversary_malformed_id_token_refused` | 2 or 4 parts, non-JSON payload, non-base64 payload, `aud` array, no `sub`, `aud` prefix and `iss` case all give `Protocol`, and nothing is cached | green |

Red output from the first run of my cases, before the suite ran:
```
test oauth2_refresh_adversary::adversary_write_401_evicts_cached_access_token ... FAILED
panicked at adapters/catalog/tests/local_runtime/oauth2_refresh_adversary.rs:376:5:
assertion `left == right` failed: a token refused with 401 on a write was served again from the cache
  left: 1
 right: 2
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 36 filtered out
```
```
test oauth2_refresh_adversary::adversary_expired_id_token_refused ... FAILED
panicked at adapters/catalog/tests/local_runtime/oauth2_refresh_adversary.rs:541:5:
an id_token whose exp is in 1970 validated a connection
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 41 filtered out
```

**3. Suite run** (I rebuilt `target/debug/connectors` first, because the binary on disk predated the build of this commit)
`CONNECTORS_TEST_CLI=$PWD/target/debug/connectors cargo test -p connectors-catalog-provider --test local_runtime -- --include-ignored`
```
test oauth2_refresh_adversary::adversary_expired_id_token_refused ... FAILED
test oauth2_refresh_adversary::adversary_write_401_evicts_cached_access_token ... FAILED
test oauth2_refresh::oauth_invalid_grant_reports_repair ... ok
test result: FAILED. 40 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.43s
EXIT=101
```
The before-count of 36 is `--list --include-ignored` with my file's cases excluded (42 total).

**4. Findings** (covering 70f734718 plus my working-tree test files)

| # | file:line | measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| F1 | `adapters/catalog/src/local.rs:1074-1076` (`Write::execute`) | Test `:376`: after a committed write gets a 401 (effect `Refused`), the next read uses the cached `ACCESS1` and makes no new token request (1, expected 2). Only `invoke`, `prepare_write` and `validate` call `evict_refused`. | Production write path: `crates/connectors-host/src/local/owner/mutation/execution.rs:235` calls `prepare_write_until`, then commit. The shipped GitLab selection has three writes (`providers/gitlab/operations.json:44,51,59`). A refused token is reused for every write until a read happens to evict it, or for up to about 59 min. | NEEDS-CHANGE (warning) | introduced |
| F2 | `adapters/catalog/src/local.rs:303-311` (`OAuth::identity`) | Test `:541`: an `id_token` with `exp: 2` validates a connection. The code comment cites OIDC Core 3.1.3.7 to justify skipping the signature, but that section also requires the current time to be before `exp`. | Only an `id_token` the TLS-verified token endpoint itself returns expired. I found no provider that does this; the case builds that state itself. | INFEASIBLE (note) | introduced |
| F3 | `apps/connectors/src/local.rs:114-117` | The Unauthorized arm checks neither origin nor stage. So a first `connections connect` with a wrong credential now reports `next_action = repair_connection`, when no connection exists to repair. The unit's own `oauth_invalid_grant_reports_repair` asserts this for connect. This also covers the Kubernetes and SQL connect refusals (`adapters/kubernetes/tests/local_runtime/cli_journey.rs:729`, `adapters/sql/tests/local_runtime/cli_journey.rs:294`); before this change they were `retry_explicitly`. `contracts/cli/v1alpha1/semantics.md:483-491` names only "a stored credential" and says probe answers are "classified separately and not by this rule". | Every connect whose provider refuses the credential (401, `invalid_grant`, `invalid_client`), on any adapter. | CONFIRMED (warning) | introduced |

For F1, the likely fix (not applied) is to evict on a `Refused` write whose error code is `Unauthorized`. The key is available from `authenticated` inside `prepare_write`, so the `Write` struct would need to carry the key and a reference to the cache.

**5. Attacked and could not break**
- No secret reaches a log, an error or a `Debug` impl: no logging or `Debug` derive in the touched code, `provider_error` drops reqwest text, and the CLI journey's material checks are green.
- Cache isolation between entries: green case above.
- Redirects: the client is built with `Policy::none()` (`http.rs:124`), and the green case above confirms it.
- Expiry arithmetic: it saturates and is capped at 24 h; `Instant::checked_add` is used.
- Rotated `refresh_token`: refused, and nothing is stored (unit test plus the code at `local.rs:275-281`).
- A 401 on read, prepare or validate evicts the cached token.
- Revisions of existing configurations: `auth` serializes the same bytes as `b3a287d02:adapters/catalog/src/local.rs:30-81,277-289`, and `acquisition` and `token_ca_digest` are only present for OAuth.
- `token_url` and `authorize_url` must be https in canonical form; the unit's cases cover this and I reviewed `https_url`.

**6. Paths written outside the worktree:** `~/.cache/w0930ra` (mode 700, used as TMPDIR, 4.0K, empty). I rebuilt the `target/` binaries inside the worktree.

```findings
- file: adapters/catalog/src/local.rs
  line: 1074
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A write commit the provider refuses with 401 leaves the refused access token in the cache, so later writes and reads reuse it until a read evicts it or it expires."
- file: adapters/catalog/src/local.rs
  line: 303
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "The id_token check cites OIDC Core 3.1.3.7 but skips the exp check that section requires, so an id_token with exp in 1970 validates; only a misbehaving TLS-verified token endpoint could supply one."
- file: apps/connectors/src/local.rs
  line: 114
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "The Unauthorized arm ignores stage, so a first connections connect with a refused credential on any adapter now reports repair_connection for a connection that does not exist, while the contract text covers only stored credentials and excludes probe answers."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
