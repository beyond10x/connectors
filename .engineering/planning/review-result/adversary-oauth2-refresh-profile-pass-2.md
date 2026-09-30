---
format: aep.planning-md/3
id: review-result:adversary-oauth2-refresh-profile-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the OAuth refresh profile
relations:
- reviews: story:catalog-oauth2-refresh-profile
revision: 1
---
unit: story:catalog-oauth2-refresh-profile at 6b4d5a4d8 (impl/catalog-oauth2-refresh-profile), plus my uncommitted test files, in ~/.local/state/worktree/trees/b10x/connectors/wave0930c-refresh
verdict: NEEDS-CHANGE
cases: executed 43→52, red 4
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 (~/.cache/w0930ra/adv2, used as TMPDIR, empty now)
needs-coordinator: none

**1. Diff stat**
```
 adapters/catalog/tests/local_runtime.rs | 2 ++
?? adapters/catalog/tests/local_runtime/oauth2_refresh_adversary_pass2.rs
```
Both are test files; I touched no implementation file. The tracked change is the new `mod oauth2_refresh_adversary_pass2` line in `adapters/catalog/tests/local_runtime.rs`. rustfmt ran on my file only, and clippy with `-D warnings` is clean.

**2. Cases added** in `adapters/catalog/tests/local_runtime/oauth2_refresh_adversary_pass2.rs`. I wrote them first and ran them alone before the suite. Red output (line numbers are from after rustfmt):

| case | asserts | now |
|---|---|---|
| `adversary2_write_credential_refusal_names_repair_like_a_read` | a write refused for its stored credential reports `repair_connection` | **red** |
| `adversary2_unsatisfiable_minimum_scope_is_refused_at_load` | a `minimum_scopes` entry that can never be granted is refused at load | **red** |
| `adversary2_token_ca_path_is_not_part_of_the_revision` | moving `token_ca_file` keeps the revision, as moving `ca_file` does | **red** |
| `adversary2_host_bootstrap_check_bounds_acquisition` | `Bootstrap::validate` refuses an unchecked `acquisition` | **red** |
| `adversary2_token_ca_bytes_are_bound_and_enforced` | changed token CA bytes give `ReadinessMismatch`; roots that do not trust the host send no HTTP | green |
| `adversary2_oversized_token_answers_cache_nothing` | a body of 64 KiB+1 or 4 MiB+1 fails, reaches no API and caches nothing | green |
| `adversary2_id_token_time_claims_are_enforced` | `iat` one hour ahead is refused, 60 s ahead is accepted, missing `exp` is refused | green |
| `adversary2_scope_whitespace_forms` | extra spaces are accepted, NBSP gives `Protocol`, an empty scope gives `InsufficientScope` | green |
| `adversary2_oauth_cached_bootstrap_round_trips_across_owner_restart` (ignored, CLI) | after owner shutdown, `owner::cached` equals the printed profile including `acquisition`; `adapters describe` answers `cached`; a new owner invokes | green |

```
panicked at .../oauth2_refresh_adversary_pass2.rs:436:5:
a write whose stored credential the provider refused reports ("retry_status", "retry_status") (preflight stage admission), where the contract promises repair_connection for any operations invoke of an existing connection
panicked at .../oauth2_refresh_adversary_pass2.rs:471:5:
unsatisfiable minimum_scopes were accepted: ["\"openid https://www.googleapis.com/auth/drive.readonly\" loaded; validate granting every requested scope gave Err(InsufficientScope)", "\"https://www.googleapis.com/auth/drive.readonly \" loaded; ... Err(InsufficientScope)"]
panicked at .../oauth2_refresh_adversary_pass2.rs:523:5:
the same token trust roots at another path changed the revision
  left: String("1800872e…")  right: String("9320f9f5…")
panicked at .../oauth2_refresh_adversary_pass2.rs:636:5:
an acquisition with a plaintext consent URL, an empty token URL and 1000 scopes validated
test result: FAILED. 5 passed; 4 failed; 0 ignored; 0 measured; 43 filtered out
```

**3. Suite run.** First I rebuilt `target/debug/connectors`: the binary was from 17:01 and HEAD was committed at 17:26.
`CONNECTORS_TEST_CLI=$PWD/target/debug/connectors cargo test -p connectors-catalog-provider --test local_runtime -- --include-ignored`
```
test result: FAILED. 48 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.05s
EXIT=101
```
The four failures are exactly my red cases. The before-count of 43 comes from `--list --include-ignored` with my module left out (52 in total).

**4. Findings** (covering 6b4d5a4d8 plus my test files)

| # | file:line | measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| F1 | `contracts/cli/v1alpha1/semantics.md:483` | Two refusals were run through the real V2 child: a committed write refused with 401, and a preflight refused with `invalid_grant`. Both give `InvalidCredential`. The host maps that to `Origin::Host` (`owner.rs:163-168`), so `mutation::Failure::next_action` (`mutation.rs:60-79`) returns `retry_status` for both, and the preflight also reports stage `admission`. The unit's new sentence, and `docs/local-catalog-provider.md:152-154`, promise `repair_connection` for any `operations invoke` of an existing connection. | Any guarded write through `operations invoke`: owner path `execution.rs:235-242` and `mutation.rs:281-288`, rendered at `operations.rs:209`. This also covers the shipped GitLab writes under a PAT. I did not run it through the CLI end to end, because no write journey with approvals exists in the repository. | NEEDS-CHANGE (warning) | introduced |
| F2 | `adapters/catalog/src/local.rs:564-569` | For an `id_token` profile, load accepts a `minimum_scopes` entry containing a space. `scope_set` splits on whitespace (`:479`), so that entry can never be granted, and every validation returns `InsufficientScope` even when every requested scope is granted. `Bootstrap::validate` (`runtime.rs:417`) lets spaces through. | Operator configuration only, for example pasting Google's space-separated scope string as one entry. Nothing shipped does this. The API profile has the same gap. | CONFIRMED (note) | introduced |
| F3 | `adapters/catalog/src/local.rs:711` | `token_ca_file`'s path goes into the revision through the serialized `auth`. `ca_file` goes in by digest only (`:710`). The same roots at another path produce a new revision. | An operator moving the token CA file; the cost is a `readiness_mismatch` until the configured revision is updated. | CONFIRMED (note) | introduced |
| F4 | `crates/connectors-host/src/local/runtime.rs:329` | `Bootstrap::validate` bounds every profile field except `acquisition`. A plaintext `authorize_url`, an empty `token_url` and 1000 scopes all validate, and the owner caches that record (`er.rs:785-797`). | Only a child that skips the provider's own load checks, and the executable is pinned by SHA. I built this state myself. | INFEASIBLE (note) | introduced |
| F5 | `adapters/catalog/src/local.rs:429` | Before this pass, the only `iat` values anywhere in the suite were `1` (`oauth2_refresh.rs:60`, `oauth2_refresh_adversary.rs:551`), so removing the `iat` check left the suite green. My green case `adversary2_id_token_time_claims_are_enforced` now pins it. I did not run a mutated copy. | Any `id_token` validation. | CONFIRMED (note) | introduced |

For F1 there are two ways to fix it; neither is applied. Either narrow the contract and guide sentence to reads, or have `mutation::Failure::next_action` return `repair_connection` for `ServiceFailure` with `Unauthorized` when the classification is `NotAttempted` or `Refused`. The second also needs `InvalidCredential` to carry the provider origin, otherwise the preflight stays at stage `admission`.

**5. Attacked and could not break**
- **Cache Mutex under interleaving:** it cannot interleave. The child serves one frame at a time on a current-thread runtime (`server.rs:98-140`), and a prepared write blocks the channel until commit or cancel (`writes.rs:167`). The lock is never held across an `await`.
- **Write-path eviction:** pass 1's case is green, and eviction happens only for `Refused` with `Unauthorized` (`local.rs:1102-1106`).
- **exp/iat:** expired, missing `exp`, and `iat` beyond the 300 s skew are all refused; `iat` within the skew is accepted.
- **Narrowed `invoke_failure`:** connect keeps `retry_explicitly` and invoke says `repair_connection`, both covered by the unit's journeys. The `stage == "dispatch"` guard always holds for `ServiceFailure` (`local.rs:132`), so it never changes the result.
- **Oversized token answers:** 64 KiB+1 gives `Protocol`, over 4 MiB gives capacity, and neither caches anything or reaches the API.
- **Scope whitespace:** extra ASCII spaces are accepted, NBSP is refused.
- **`Profile.acquisition` ESS round trip across an owner restart:** holds, cached record and CLI both.
- **`token_ca_file` bytes:** bound to the revision, captured at load, and enforced.

**6. Paths written outside the worktree:** `~/.cache/w0930ra/adv2` (mode 700, used as TMPDIR, 4.0K, empty). The builds went into the worktree's `target/`. My lease `wave0930c-refresh-adv2` is released.

```findings
- file: contracts/cli/v1alpha1/semantics.md
  line: 483
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The new rule promises repair_connection for any stored credential refused on operations invoke, but a guarded write refused with 401 at commit or invalid_grant at preflight reports retry_status (preflight at stage admission), because InvalidCredential maps to Origin::Host and mutation::Failure::next_action has no unauthorized arm."
- file: adapters/catalog/src/local.rs
  line: 564
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "An id_token profile loads with a minimum_scopes entry containing whitespace, which scope_set can never grant, so every validation is InsufficientScope even when all requested scopes are granted."
- file: adapters/catalog/src/local.rs
  line: 711
  category: property
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "token_ca_file enters the configuration revision by path as well as by digest, unlike ca_file, so the same trust roots moved to another path change the revision."
- file: crates/connectors-host/src/local/runtime.rs
  line: 329
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "Bootstrap::validate bounds every profile field except acquisition, so a plaintext consent URL, an empty token URL and 1000 scopes validate and are cached; only a child bypassing the provider's own load checks can supply one."
- file: adapters/catalog/src/local.rs
  line: 429
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "No case before this pass sent an iat ahead of now, so removing the iat skew check left the suite green; adversary2_id_token_time_claims_are_enforced now pins it."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
