# Independent review remediation — 2026-09-08

This response answers all 24 rows from the two independent reviews under
[story:independent-review-remediation](../.engineering/planning/story/independent-review-remediation.md).
The original reports remain unchanged: [review A](../.engineering/planning/review-result/independent-repository-a-20260908.md),
[review B](../.engineering/planning/review-result/independent-repository-b-20260908.md).
The [investigation and failing reproductions](../.engineering/planning/verification-report/independent-repository-followup-20260908.md)
precede these fixes, at source baseline `492c20c`.

## Runtime corrections

SQL now owns an execution supervisor after connection establishment. It notices
when the invocation is dropped and sends a PostgreSQL CancelRequest using the
actual connected address, backend key and captured TLS configuration. It continues
driving the original connection during bounded cleanup. The execution deadline is
independent of caller-controlled session settings. Connection establishment has a
5-second bound, execution at most 10 seconds, and up to 2 seconds of cleanup are
reserved within the 15-second request budget. A slow connection reduces the
execution time available. The supervisor does not survive process/runtime loss.

The service contract now distinguishes an attempted cancellation from confirmed
remote termination. PostgreSQL does not acknowledge cancellation on its separate
connection, and a partition or process crash can prevent delivery. The tests prove
backend termination against the reachable disposable database; they do not claim
termination under those failures. The protocol behavior is documented by
[PostgreSQL 17](https://www.postgresql.org/docs/17/protocol-flow.html#PROTOCOL-FLOW-CANCELING-REQUESTS).

The three toolchain tests serialize their complete fixture-write/probe intervals,
eliminating the inherited writable-file-descriptor race while the normal test
harness continues to use default threads. Production resolution and wrong-version
refusals are unchanged. A fixed cohort of 100 runs passed, versus 2/100 failures in
the matching pre-fix direct-binary experiment.

The gate respects `CARGO_TARGET_DIR` and puts MSRV metadata beneath that base.
Federation advertises a schema derived from the actual Rust configuration owner,
including closed objects and downstream count bounds; the schema contributes to
the descriptor revision. Schema derivation is now always available in the host,
including builds without default features; the old `schema` feature remains an
empty compatibility feature. Public `Spec` import returns typed refusals for
missing operations or mapped inputs instead of panicking.

## Verification

The full gate passed on its first run, exit 0:

```sh
TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 \
  CARGO_TARGET_DIR="$PWD/.local/review-fixes-20260908/target" \
  cargo run --locked --offline -p connectors-build -- \
  --ess "$PWD/.local/toolchains/ess/0.20.0/bin/ess" gate --msrv
```

| Check | Result |
|---|---|
| Workspace tests, including doctests | 50 passed, 0 failed, 0 ignored |
| Formatting, descriptor/bundle drift, Clippy `-D warnings`, library/CLI boundaries | Passed |
| Minimum Rust | 1.88.0, all targets passed |
| ESS 0.20.0 | 7 files valid, 92 declarations, 169 compiled scenarios (34 authored), 0 refusals |
| AEP | 60 artifacts valid; 18 warnings about unrecognized/missing findings blocks, retained verbatim |
| MSRV target isolation | 1,991 pre-existing default-target files unchanged; 1,997 files under the caller-selected private MSRV target |
| Toolchain fixture stress | 100/100 default-thread runs passed |

See the [full gate log](evidence/independent-review-fixes-20260908/full-gate.log),
[summary](evidence/independent-review-fixes-20260908/gate-summary.json),
[isolation check](evidence/independent-review-fixes-20260908/target-isolation.json),
[stress evidence](evidence/independent-review-fixes-20260908/toolchain-stress.json),
[live SQL log](evidence/independent-review-fixes-20260908/sql-live.log),
[inventory audit](evidence/independent-review-fixes-20260908/inventory.json), and
[verified source digests](evidence/independent-review-fixes-20260908/verified-sources.json).
ESS proves model structure and authored traces here; runtime SQL proof comes from
the protocol and live database checks.

The [Rust live SQL harness](../adapters/sql/examples/live_deadlines.rs) is explicit
local tooling and is not run by the offline suite. It refuses nonlocal Docker
endpoints, uses an already-cached image without pulling, creates one disposable
database with a non-superuser reader, and cleans up its container and temporary
TLS material. Connectors operation discovery returned no admitted Docker operation;
that gap was reported before using Docker.

```sh
TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 \
  cargo run --locked --offline -p connectors-sql --example live_deadlines
```

Observed PostgreSQL 17.11 image:
`sha256:1bea307dfb3ee30541a7acf7de14b58bcd6948da98e5d31a04c627c4d35ec64b`.

| Live case | Result |
|---|---|
| `schema.list` | Returns the granted `review_sample.id` column |
| Normal 30-second sleep | Timeout at 10.030 s; zero active test backends |
| First row disables `statement_timeout`, next row sleeps | Timeout at 10.005 s; zero active test backends |
| Caller drops an executing invocation | Cancellation completes; zero active test backends |
| SQL TLS connection | `pg_stat_ssl` confirms encryption; the untrusted self-signed certificate is refused |
| TLS timeout bypass after replacing the CA file during execution | Timeout at 10.007 s; zero active test backends, proving cancellation reused captured trust |

The automated wire fixture separately checks CancelRequest framing and backend
keys, deadline cancellation, caller drop, cleanup against an unresponsive server,
transaction setup, the bounded-row wrapper and exact text/null/empty parameter
encoding. Host tests exercise real in-flight capacity, deadline expiry, oversized
results, configuration size, environment credentials, federation duplicates,
reserved route names and cycles. Kubernetes tests cover disabled host discovery
and the 4,096-endpoint expansion boundary. The response round-trip now asserts
the outcome payload as well as correlation.

Two new test expectations were corrected during development: the disabled
Kubernetes library operation returns `Forbidden` (and is absent from the
descriptor), and PostgreSQL's explicit boolean-to-text cast returns `"true"`.
The initial live harness's boolean assertion failed after the plaintext cases
passed; its failure and cleanup are retained beside the successful complete run.
Neither correction relaxed a production assertion or skipped a regression.

## Every source finding

“Fixed” means implementation or documentation changed to resolve the finding.
“Documented limit” closes an overbroad or missing claim without adding the optional
capability. “Existing behavior verified” preserves a qualified observation and
does not pretend a defect was repaired. Duplicate rows retain their original IDs.

| ID | Disposition | Correction or evidence |
|---|---|---|
| A-M1 | Fixed | Current GitLab design text uses ESS 0.20.0; historical observations retain their original versions. |
| A-M2 | Fixed | The upstream README identifies the retained ESS 0.20.0 refusal. |
| A-M3 | Fixed | All 17 semantic documents are indexed; 22 rows are counted separately; configuration is deferred explicitly. Broader adapter-matrix reconciliation retains its existing story owner. |
| A-M4 | Fixed | The mutation/idempotency stories name commits `34f298a`/`8903166` instead of saying uncommitted. |
| A-M5 | Fixed clarification | The design artifact separates its historical handoff from dated later delivery; no lifecycle approval was invented. |
| A-M6 | Fixed previously | `492c20c` preserved the G1–G10 source verbatim and repaired both proposal links. |
| A-M7 | Fixed | MSRV output follows the caller's target base; see isolation evidence. Same correction as B-F03. |
| A-M8 | Fixed | The CLI story name is explicitly proposed and uncreated; its decisions remain unassigned. |
| A-N1 | Fixed coverage gaps | Added host, environment, federation, Kubernetes and SQL regression coverage; live SQL verifies TLS and schema discovery. |
| A-N2 | Documented limit | README states that federation downstreams use built-in public roots and have no private-CA setting. That optional capability is not implemented. |
| A-N3 | Fixed | The mutation invocation sample is explicitly incomplete/proposed and omits the undecided version field. |
| A-N4 | Fixed | The service proposal distinguishes preserved response structure from proposed new fields/codes. |
| B-F01 | Fixed | Fixture write/probe serialization; 100/100 default-thread runs pass. |
| B-F02 | Fixed | Independent execution deadline, backend cancellation, captured TLS trust and bounded cleanup; live and wire regressions pass. |
| B-F03 | Fixed | Same MSRV target-isolation correction as A-M7. |
| B-F04 | Fixed | Current ESS references and the design status header are current; same version corrections as A-M1/A-M2. |
| B-F05 | Fixed | Same inventory correction as A-M3, with explicit row-versus-document counts. |
| B-F06 | Fixed previously | Same preserved G-source repair as A-M6. |
| B-F07 | Fixed | Federation configuration schema derives from its Rust owner and is validated in the service tests. |
| B-F08 | Documented limit | Contract/README place operation bounds after admission/body reading; public ingress requires separate connection/read limits. |
| B-F09 | Fixed | SQL setup, wrapper and Bind bytes are asserted; wire round-trip checks the outcome; named branch gaps have focused coverage. |
| B-F10 | Existing behavior verified | The unchanged closed `Error` decoder already rejects duplicate/unknown fields; the investigation's actual-client probes remain applicable. No parser bypass was demonstrated. |
| B-F11 | Fixed clarification | `Secret` documentation distinguishes prevention of accidental formatting from trusted holders' deliberate access to plaintext. |
| B-F12 | Fixed | Public import returns typed refusals for missing operations and fields, covered by malformed-after-validation tests. |

This is 21 fixed rows (including two source-portability rows fixed previously),
two documented limits and one verified existing behavior. These are source-row
counts, not counts of independent code defects.

The overlapping original E13 is also closed in the earlier 48-item ledger. That
ledger now has seven fixed findings and 41 still assigned; the broader
`contracts-documentation-index` story remains draft for E22 and its prerequisites.
No wire-version choice, private-CA federation feature, public-ingress implementation,
new adapter or Atlas/consumer integration was introduced by these fixes.
