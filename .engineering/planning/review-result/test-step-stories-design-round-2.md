---
format: aep.planning-md/3
id: review-result:test-step-stories-design-round-2
kind: review-result
status: active
title: Design critic, round 2, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision.

Round 2 of the design critic on the 18 drafts of
`story:the-gates-test-step-ends-within-120-s-on-the-forge`, read on 2026-11-25 on the branch
`plan/test-step-stories`. Read-only: source files and the graph were read, no build and no test was
run. Each claim is labelled verified (read in source) or inferred.

## Round-1 findings

1. Nobody owned the test step in `slow.yml` and the `ignored.rs` table. Resolved:
   `story:the-lane-slow-runs-the-ignored-timing-family` owns both, and the stories that move a test
   to the lane carry a `depends_on` edge to it. Verified: `slow.yml` still has one check and narrow
   paths. New problems with that lane: findings 1 and 2 below.
2. The "release build ignores it" acceptance was unowned. Resolved: the helper story holds it once
   and each override story points to it. Verified: `Cargo.toml` sets `debug-assertions = true` only
   for `[profile.dev]` and `[profile.test]`. Inferred: `cargo test --release` resolves to a profile
   with debug assertions off.
3. About ten copied overrides and a hidden owner-environment edit. Mostly resolved: one helper and
   one forwarding rule by the `CONNECTORS_TEST_OVERRIDE_` prefix; the SQL timeout is a constructor
   parameter. Leftover: finding 5.
4. The connection-lease seam changed what is proved, and its target was unknown. Resolved in shape:
   the question went to `story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`;
   the eleven-tests story has an edge to it and cannot be approved before the answer.
5. Catalog hidden dependency. Resolved: the crate story writes no production code and has an edge
   to the provider story; both keep the lowering-twice test lowering.
6. Two 15 s literals. Resolved: the story names line 239 and leaves line 79 out.
7. Two clock stories and item 7. Resolved: the edge from seven-cli to five-host exists;
   `query_timeout()` keeps one reader; item 7 moved to the question story; acceptance 3 asks for 20
   runs.
8. The eighteen story was a split abstraction. Resolved: the jail has its own story with an edge to
   it, the `ess generate` item moved into it, the stress test is its own story, and only it writes
   `.config/nextest.toml`.
9. The `policy_child` test asserts a report, not the wait. Resolved: the real 30 s run of
   `WAIT_BOUND` stays in the lane.
10. Small items. Resolved.

## Shape of the set

- 22 `depends_on` edges inside the set, no cycle, no edge entering from outside. The longest chain
  is lane, helper, idle-exit, slow-owner, seven-cli, eleven.
- The five writers of `crates/connectors-host/src/local/owner/transport.rs`: the order is total
  (helper, idle-exit, slow-owner, seven-cli, eleven), all four edges exist, the regions are disjoint
  (verified by line ranges).
- `story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run` uses an accepted form: a
  nextest setup step and a cache under `target/test-fixtures/<digest>/`, no `OnceLock`.
- `story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process`: only its id is
  stale; the body reads a committed lowering and adds no cache.
- `story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`: digest
  check in the gate, byte comparison from sources in the lane. It complies.

## Findings, most severe first

1. `story:the-lane-slow-runs-the-ignored-timing-family` (blocker): the lane job has no step that
   permits unprivileged user namespaces, and the lane tests other stories move need them. Verified:
   `slow.yml` has none; the gate job and the journeys job in `rust-gate.yml` have one; the jail
   fixture (`crates/connectors-custody/src/mounted/fixture.rs`) needs them; `connection_lease.rs`
   runs through that jail. Change: add the step to `slow.yml` in the lane story and say which lane
   tests need it.
2. Same story (warning): the second lane step can pass with zero tests run. The release test is
   compiled only without `debug_assertions`; if a profile resolves wrongly the filter matches
   nothing and the step exits 0. Change: the step fails on zero tests run.
3. `story:six-cli-tests-take-jail-binaries-built-once-per-run` (blocker): the product "built once"
   has no producer and no digest of sources. Verified: `Container::new` takes the CLI from the
   compile-time `env!("CARGO_BIN_EXE_connectors")`; `Jail::install` is a plain `fs::copy`. The story
   claims no saving (50.7 s to 50.6 s). Change: drop the setup product and hard-link
   `CARGO_BIN_EXE_connectors` into the jail inside `Jail::install`, with a copy fallback; a cached
   hash of the test binary is keyed by content, not mtime; the edge to the eighteen story may go.
4. `story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing` (warning): nobody owns the
   run that lets `no_suite_sees_the_installation_another_suite_sets` fail; this story and the lane
   story each defer to the other. Change: decide here: a third lane step that runs the whole `main`
   binary in one process, or the test is removed and the removal is stated.
5. `story:one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`
   (warning): acceptance 3 tests "the function that picks the owner's environment"; the forwarding
   is an inline loop in the spawn. Change: say that the story extracts a function and tests it.
6. Same story (warning): nothing checks that every override goes through the helper; the grep covers
   three files while later call sites are in `http.rs`, `config.rs`, `clock.rs` and
   `maintenance.rs`. Change: a gate test that finds no direct read of `CONNECTORS_TEST_OVERRIDE_`
   outside the helper.
7. Four stories that shorten a production wait (warning each) say how a release build is kept from
   the override but not which property of the product the wait protects:
   `story:seven-cli-tests-shorten-their-waits-or-a-production-wait` (print deadline, clock query
   timeout, client drain bound),
   `story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait` (the 10 s
   connect wait), `story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps` (the 5 s
   recovery sweep), `story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts` (15 s
   request and 5 s connect timeouts). Change: one sentence each naming the property and why the
   shorter test value is safe; the eleven-tests story the same for the 30 s admission bound.
8. `story:where-do-the-10-s-of-the-identification-retry-loop-test-go` (warning): question 2 reads
   the connect-wait override the slow-owner story creates, and no edge records the order. Change:
   add the edge or reword the question as an inference.
9. `story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run` (warning): the
   template-store digest covers `metadata.rs`, `metadata/` and `Cargo.lock`; verified:
   `Metadata::initialize` creates the store through the `filesystem` module, which is not an input.
   Change: put `filesystem` into the digest inputs, or digest all of `connectors-host/src/local/`.

## Approve as they stand

- `story:five-host-unit-tests-shorten-the-waits-they-chose`
- `story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait`
- `story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds`
- `story:the-100-run-capacity-stress-keeps-10-runs-in-the-gate`
- `story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`
- `story:the-catalog-crate-tests-ingest-the-released-definition-once-per-process`
- `story:where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`

Shape-sound with the one sentence of finding 7: the idle-exit, slow-owner, seven-cli and http
stories. Not as they stand: the lane story, the helper story, the six-cli story, the eight-tests
story, the eighteen-tests story, the identification question, and the eleven-tests story (waits on
its question).

## Not established

- Cargo's profile resolution for `cargo test --release` was not confirmed by a run.
- Whether nextest 0.9.143 runs setup scripts without an experimental switch.
- Whether the lane needs more setup than the user-namespace step.

## Outside this critic's role

- The parent's figures say the stories leave a test step of about 408 s against 120 s; no item
  closes that gap.
- Eight draft stories outside the set write the digest inputs of the committed lowering; each turns
  the fast check red until it regenerates.
- Several stories share `local_cli.rs`, `mounted.rs` and `refresh/tests.rs` with no ordering edge.
- `story:test-binary-per-package-rest` renames binaries that acceptance commands name.
- The parent's "seven waves" is stale: the store shows eleven.
