---
format: aep.planning-md/3
id: review-result:test-step-stories-design-round-1
kind: review-result
status: active
title: Design critic, round 1, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Recorded by the session that ran the panel, 2026-11-24. The critic read the first draft of twelve
stories that decompose story:the-gates-test-step-ends-within-120-s-on-the-forge. The set was revised once after this
report (eighteen stories); the report below is the critic's text about the first draft, with
the path of the worktree shortened to `<tree>`.

verdict: needs-revision

**needs-revision**

Findings, most severe first. The edge to add is `depends_on` ("Needs something else first") in every case.

1. `story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait` and 7 other stories — none owns the step that runs tests in `.github/workflows/slow.yml`.
   - Eight stories each list `slow.yml` as an inferred scope entry ("Seven other stories of this set add a step to the same file").
   - The workflow has one check today (`metadata-conformance emissions --check`) and runs no test.
   - Each story's "lane slow" acceptance cannot pass until a test step exists.
   - Each story also says "how a test is kept out of the gate's test step … is not stated". The repository already has that mechanism: `#[ignore]` plus the classification in `crates/connectors-build/src/ignored.rs`, which has a `Family::Timing`. `rust-gate.yml:109-111` already runs `ignored`. `ignored.rs` is in no story's scope.
   - Smallest change: add a ninth story, "the lane slow runs the ignored timing family". It owns the `slow.yml` step, the union of trigger paths and the `ignored.rs` entries. The eight stories drop `slow.yml` from their scope and take a `depends_on` edge to it.
   - Unchanged: each story still names its own slow-lane test.

2. `four-idle-exit`, `seven-cli-tests-of-a-slow-owner-start`, `eleven-connection-lease`, `five-tests-…-http-and-sql`, `seven-cli-tests-shorten-…` — the "a build without `debug_assertions` ignores it" acceptance is unowned.
   - Five stories carry that acceptance, each ending "which job builds … is not in the plan".
   - A debug-built test binary cannot test a `cfg(not(debug_assertions))` body.
   - A precedent exists: `apps/connectors/tests/cold_owner_start.rs:318` is `#[ignore]` with "run explicitly on a release build".
   - Smallest change: resolve this together with finding 3. One shared helper gets one release-build test. The five stories drop their own copies.

3. About ten new debug overrides, no shared mechanism, and a hidden owner-environment edit.
   - The existing mechanism is not one helper. It is a copied pattern: `const NAME`, a `#[cfg(debug_assertions)] fn` reading the env var, and a `#[cfg(not(...))] fn` returning the constant.
   - It appears in `transport.rs:1969-1979` (`idle_bound`), `apps/connectors/src/local/owner_stop.rs:18-29`, `registry/lifecycle.rs:682-700` and `transport.rs:1590-1597`.
   - Every story points at `idle_bound` as the model. Each will make another 12-line copy for these overrides: connect wait, sweep interval, clock offset, admission bound, HTTP request and connect timeouts, SQL query timeout, print deadline, clock query timeout, drain bound.
   - The CLI starts the owner with `.env_clear()` and forwards only a fixed list (`transport.rs:1670` and `1685-1695`).
   - So the overrides read in the owner process only work for CLI-spawned owners if added to that list. These are the clock offset and admission bound in `eleven-connection-lease` (its tests drive the CLI through `mounted.rs`), and the drain bound in `accept` for `seven-cli-tests-shorten-…`.
   - No story names that list. `eleven-connection-lease` does not list `transport.rs` at all.
   - Smallest change: one small story adds a `debug_override(name, production)` helper, the forwarding-list entry point, and the one release-build test from finding 2. The override stories take `depends_on` to it.
   - Each of those stories then adds only its call site.
   - `five-tests-…-http-and-sql` should use a `Config` or constructor parameter for the SQL timeout instead of an env var in the shipped adapter crate. `Config` is already imported in `mysql.rs`.

4. `story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping` — the seam changes what is proved, and its target is unknown.
   - The offset has to move `connectors_sdk::now_ms()` (`crates/connectors-sdk/src/lib.rs:356`) in three processes, one of them the adapter fixture. That file is not in scope.
   - The same crate serves shipped adapters.
   - Deadlines elsewhere are `Instant`-based (`transport.rs`) and would not move with the offset.
   - The admission bound is `now_ms() + 30_000` at two sites: `connections.rs` in the CLI and `supervisor.rs:1355` in the owner. The story says which one the tests prove is "inferred, not run".
   - Risk: with a shifted clock and a shorter bound, the four tests exercise the same code with different numbers. They no longer prove the production 30 s bound, and an offset applied in one process but not the other can pass without the two clocks agreeing.
   - Smallest change: first fix the real site and process, then name `connectors-sdk` in scope. Keep the two slow-lane cases.

5. `story:the-catalog-provider-tests-read-a-committed-lowering-…` and `story:the-catalog-crate-tests-ingest-…-once-per-process` — a hidden dependency.
   - The committed lowering's digest covers "the lowering sources".
   - The catalog-crate story edits `crates/connectors-catalog/src/definition.rs` (`lower` and the ingest).
   - Whichever lands second invalidates the committed file and its digest check, and there is no edge.
   - The catalog-crate story caches the ingest for a test that calls `lower` several times, then reuses it. That weakens `lowering_the_released_definition_twice_writes_the_same_bytes`, because the second call no longer re-ingests.
   - A process-global cache in production `lower` must also be keyed by content if `lower` serves user definitions.
   - Smallest change: add `depends_on` from the provider story to the catalog-crate story, so the digest is computed on the final `lower`. Add an acceptance line that the twice-lowers test still re-ingests.

6. `story:five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts` — two 15 s literals.
   - `http.rs:79` (`READ_POST_DEFAULT_TIMEOUT`) and `http.rs:239` (`.timeout(Duration::from_secs(15))`) are both 15 s.
   - The story overrides only the builder's timeout and lists `READ_POST_DEFAULT_TIMEOUT` as "not in this story".
   - The test `provider_budget_generic_30_seconds_is_not_clipped_to_legacy_15` proves that a per-request budget beats the client-wide timeout. That holds only if the right 15 s is shortened.
   - Smallest change: name which literal "legacy 15" is. Keep the relation (override less than budget less than fixture delay) in the acceptance.

7. `story:seven-cli-tests-shorten-their-waits-or-a-production-wait` — two defects.
   - `clock_check_below_quorum_refusal` gets a debug override of the 2 s clock query timeout.
     - The implemented `story:clock-quorum-test-tolerates-a-slow-loopback-source` exists because a source answering late on a loaded runner failed that test.
     - Shrinking the timeout and the 2.5 s answer removes its margin. Risk: flake on loaded runners.
   - `story:five-host-unit-tests-shorten-the-waits-they-chose` also shortens the same 2 s via a `cfg(test)` thread-local in `clock.rs`. The two mechanisms sit in the one function `query_timeout()`, `clock.rs:60-76`.
   - Also in this story: item 7 (`adversary2_the_identification_retry_loop…`) is an investigation. It stays at 10.2 s with no change, but counts in the acceptance sum.
   - Smallest change: record an edge between the two clock stories (or merge both clock changes). Drop item 7 from the set into a spike.

8. `story:eighteen-tests-take-a-template-store-or-jail-built-once-per-run` and `story:eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing` — a split abstraction and a contested file.
   - The eighteen story is two stories. The template store (nine unit tests and two integration tests) and the template jail (CLI tests, `mounted.rs`) share only a nextest setup script. The 100-to-10 stress move is a third change, and three of its tests stay unchanged.
   - Item 5 of the eight story (`ess generate` once under `target/`) is the same "built once, keyed by digest" idea. The eight story itself notes both stories would edit `.config/nextest.toml`.
   - Smallest change: move item 5 of `eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing` into the template story. Split that story into store and jail, or give both a `depends_on` to one setup-script story.

9. `four-host-tests-no-longer-wait-out-the-30-s-lock-wait` — the new `policy_child` action reports its effective lock wait, and the test asserts that report. The test no longer exercises the wait itself. The risk is small, because the seams are `cfg(test)` and nothing production changes. It is acceptable if acceptance 2 keeps the real 30 s run in the lane `slow`.

10. Small items.
    - The digest of the template store covers `metadata.rs`, which `four-host-tests-no-longer-wait-out-the-30-s-lock-wait` edits for test-only helpers, so the key churns.
    - `seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait` names `Client::connect` only. `WriteClient::connect` (`transport.rs:1294`) has its own 10 s literal.
    - Three stories edit different regions of `transport.rs`; the stories already say so. This is a merge hazard, not a design defect.

Walked: 3,902 edges in the whole graph (full `graph` output; the grep that listed the set's edges was filtered to this set). The set's edges are only `decomposes`, `informed_by` and `serves`, all to the parent, the test plan and the vision. There are no edges between the twelve, so there are no cycles and no chain. Edges from the twelve stories to artifacts outside the set were not followed. Acceptance reads traced to a producer in the set: 4 (items 1, 2, 3, 5), with 0 edges recorded.

Out of my lane:
- Three acceptance bounds are not 1.5 times the estimate (eighteen: the estimate itself, 107.7 s).
- `five-host-unit-tests-shorten-the-waits-they-chose` and `four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps` name no slow-lane run.
- Two stories (`eleven-connection-lease`, `eighteen-tests-take-a-template-store-or-jail-built-once-per-run`) touch `mounted.rs`; that is the parallel-safety critic's call.

`aep plan artifact validate` printed `valid`; its findings are not mine.
