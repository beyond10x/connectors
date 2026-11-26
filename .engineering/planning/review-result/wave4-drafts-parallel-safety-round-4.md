---
format: aep.planning-md/3
id: review-result:wave4-drafts-parallel-safety-round-4
kind: review-result
status: active
title: Parallel-safety critic, round 4, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at a2a515f39. Nothing was built or run. Two drafts were judged, with the proposed story
on the two MCP tests as their neighbour.

- Sixteen ess_model tests: approve. No finding.
- Provider budget that ends in an unanswered TLS handshake: approve. No finding.

## The findings of round 2

1. ess_model, the two new tests named no parent directory of their own: resolved. Change
   item 7 gives each new test `tempfile::tempdir_in(CARGO_TARGET_TMPDIR)`, and the children
   get that one. No new test writes the default parent directory.
2. ess_model, the lock described as released when the process ends: resolved. The helper
   drops the file, and with it the lock, when it returns.
3. Provider budget, no line for story:test-binary-per-package-rest: resolved. The draft says
   that story would move the test files and change the binary names of acceptance 5, and
   that it is not planned beside this story.

## Can the three run at once

Yes, in three worktrees, merged in any order. No collision found.

- ess_model writes `adapters/sql/tests/ess_model.rs` alone. Its frontmatter scope and its
  Scope section agree. `sha2`, `hex`, `tempfile` and `connectors-spec` are dev-dependencies
  of `adapters/sql/Cargo.toml` already.
- The provider budget writes `crates/connectors-host/src/http.rs`,
  `crates/connectors-host/tests/provider_timeout_adversary.rs`,
  `crates/connectors-host/Cargo.toml` and `Cargo.lock`. `Cargo.lock` holds one `tower-layer`
  and one `tower-service` entry, both 0.3.3, and the block of connectors-host lists neither
  yet. `http_budget.rs` is not written; its tests with 200 ms and 150 ms left stay in the
  band under 500 ms.
- The MCP story writes `apps/connectors/src/mcp/projection.rs` and two test files of
  `apps/connectors`.
- The three sets share no path.

Surfaces at run time:

- `target/tmp`: the ess_model draft alone writes it, under its own parent name and under
  `tempdir_in` names. No cargo configuration sets a shared target directory, so each
  worktree has its own lock files and its own `generations.log`.
- Environment variables: the MCP story sets two on child processes after `env_clear`. The
  provider budget sets none. The ess_model child variable has no name in the draft.
- Ports: the provider-budget tests bind `127.0.0.1:0`. No fixed port in the set.
- `crates/connectors-build/src/ignored.rs` and `.config/nextest.toml`: no draft writes
  either, and no draft adds an ignored test.

`aep plan artifact waves` puts ess_model in wave 2, the MCP story in wave 38 and the provider
budget in wave 43. None of the three is listed as unassessed, and no collision line pairs two
of them. Four reads go to a producer outside the set, and a `depends_on` edge records each.

## Not established

- Inferred, not measured: the provider-budget tests with a budget of 1,000 ms and a lead of
  250 ms could fail when the test task is not polled for more than 250 ms on a loaded host.
  Left to the acceptance lane.
- Not checked: whether `ess generate --out` writes outside its `--out`. Today's 16 calls at
  once share that risk, so the change adds none.

## Notes outside this lane

- The child test of the ess_model draft reads its parent directory from an environment
  variable that the draft does not name. Not counted as a finding: no harm is shown.
- The provider budget names one of three contract texts that cite lines of `http.rs` by
  number. The other two are `contracts/auth/connection/v1alpha1/semantics.md` (lines 30 to
  36) and `adapters/prometheus/contracts/series/v1alpha1/semantics.md` (lines 11 to 25).
