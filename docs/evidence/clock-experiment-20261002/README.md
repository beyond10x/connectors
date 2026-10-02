# Registry clock experiment — 2026-10-02

The single candidate is rejected. Removing the equal-value registry-clock action
from ordinary business batches breaks the same-millisecond revision fence and
still exceeds the accepted store-size cost limit. The candidate was reverted;
no production change is proposed by this evidence.

Baseline source: `26929027f85b4653d81bd9cb290a7f472cfe84e8`, with runtime source
identical to released `v0.25.0`. Entity Runtime remains 0.25.1 at
`72455539c0756290d03fd5ef28b30512a729c457`. Cargo.lock SHA-256:
`97865c27834d79a1c6eceabf20c3b4e12ab12996daa5050444f0f235d1eb72c0`.
The [rejected patch](rejected-candidate.patch) changes exactly one condition in
`metadata/er.rs`; its SHA-256 is
`230d9cb2bd04f37756e7d5a8cf683e2498c198dcaddcc45f904ed733401ecdcc`.

## Decision evidence

The existing same-millisecond test passed on baseline and failed on the candidate:
`Err(MetadataUnavailable)` replaced required `Err(ConcurrentRevision)`.
This loses the definite precommit conflict used by observation retry. A stale
successful response was **not** observed. The complete outputs and process exits
are retained in [baseline](baseline-same-millisecond.log) and
[candidate](candidate-same-millisecond.log) logs and corresponding `.exit` files.
Nine invariant cases executed on each: baseline 9 passed; candidate 8 passed,
1 failed. The other cases cover monotonic sampling, concurrent observations,
refused-action clock persistence, repeated observation races, reopen corruption
and three postcommit checks. Their original outputs remain in the retained task
archive. No invariant or production deadline was changed.

The cost probes ran once per profile, in release mode with Rust 1.98.1, two Cargo
jobs, and identical saved fixture bytes. Each read-cost row is one warmup followed
by five measured invokes. All six warmups and all thirty measured invokes passed;
the complete logs contain no caught-panic output.

| Actual events (requested) | Baseline wall median, ms | Candidate wall median, ms | Baseline CPU median, ms | Candidate CPU median, ms |
| --- | ---: | ---: | ---: | ---: |
| 55 (50) | 545 | 356 | 474 | 316 |
| 601 (600) | 13,805 | 4,410 | 13,559 | 4,174 |
| 1203 (1200) | 31,349 | 9,227 | 30,918 | 9,225 |

The candidate ratio is **25.9185×**, exceeding the maximum **2×**. Baseline is
57.5211×. Successful invokes append seven events on baseline and four on candidate.
The [baseline invoke log](baseline-invoke.log) includes fixture generation;
the [candidate invoke log](candidate-invoke.log) reuses saved fixtures. Their total
process durations must not be compared as invoke timings.

Both diagnostic batch probes completed all rows, three pairs per size:

| Actual events | Baseline runtime-record / clock batches, ms | Candidate runtime-record / clock batches, ms |
| --- | --- | --- |
| 55 | [47,24,29] / [66,67,59] | [35,23,29] / [70,79,63] |
| 601 | [50,35,44] / [2400,2596,2946] | [55,26,35] / [2224,2285,2601] |
| 1203 | [67,79,28] / [6303,7931,6318] | [37,26,25] / [5430,5402,5050] |

Full outputs: [baseline batch](baseline-batch.log), [candidate batch](candidate-batch.log).
All four cost-probe process exits were zero; adoption additionally considers
printed failures and invariants, rather than treating exit zero as acceptance.

## Reproduction and retained identities

Run from an isolated checkout with a private temporary directory, default target,
`CARGO_BUILD_JOBS=2` and the repository-pinned dependencies:

```console
CONNECTORS_STORE_COST_STORES=<private-fixture-directory> CONNECTORS_STORE_COST_EVENTS=50,600,1200 cargo test --locked --offline --release -p connectors-host --lib local::registry::store_cost_tests::read_invoke_cost_by_store_size -- --exact --ignored --nocapture --test-threads=1
CONNECTORS_STORE_COST_STORES=<same-private-fixture-directory> CONNECTORS_STORE_COST_EVENTS=50,600,1200 cargo test --locked --offline --release -p connectors-host --lib local::registry::store_cost_tests::batch_cost_by_subject -- --exact --ignored --nocapture --test-threads=1
cargo test --locked --offline --release -p connectors-host --lib local::registry::tests::prepared_same_millisecond_observation_refuses_stale_registry_state -- --exact --nocapture --test-threads=1
```

All three fixtures were generated once under baseline. Untouched baseline
measurement copies were byte-identical to saved databases. The saved set was
checked before and after both candidate probes and remained unchanged. There was
no complete fixture manifest before initial generation; per-fixture creation
hashes and later complete manifests establish the actual sequence.

| Actual events | Saved metadata.sqlite3 SHA-256 |
| --- | --- |
| 55 | `0ebbf8c6cd0e604bf967ccda3552b54d2193c220cec2b766751b5d035aaa3e8a` |
| 601 | `ef26e15b3140d43e979955724c44f1a6b665293dc3ac33c767b8dfaf8a2808c2` |
| 1203 | `9fd3ad20853b0cfcac4502da45b55819d38febb2c8d18c4982c7996b984bca55` |

The baseline test executable hashes to
`8eb0d97063caeaa336b6a4e8e9f7b2d62167fc9da5a9e289914d61767dc2a936`;
candidate to `2582730f823ae146c5b9008f2b6cd7f782cb734e298bac7f02b042f7896be0b2`.
Restored `er.rs` hashes to
`847e76669449d6df66c33ec8284d7a620f7b52c78a6a9a160155aa262f926415`, identical
to baseline. Tracked diff/status are empty; package formatting and diff checks
passed. Full package/Clippy was not rerun after restoring unchanged source.

## Limits and next work

Unrelated host builds occurred during measurement. CPU and wall figures are
retained; these are host observations, not a controlled speedup attribution.
The deterministic correctness failure and failed adoption threshold both reject
this candidate. The existing 30-second bridge bound and blob verification stayed
unchanged. There was no second candidate or favorable rerun.

[Entity Runtime #51](https://github.com/beyond10x/entity-runtime/issues/51) remains
the upstream dependency. This experiment does not complete the 600/6000-event
and approximately 700-invoke sustained workload milestone, and does not rule out
every possible future clock-isolation design.

Published log copies replace the local home-directory prefix with `$HOME`.
Original reports, logs, fixture hashes and saved databases are retained privately
with the task worktree recovery archive; those are not public provider data.
