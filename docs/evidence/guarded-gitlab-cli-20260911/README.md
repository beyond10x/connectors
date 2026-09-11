# Integrated guarded GitLab CLI checkpoint

Implementation under active `story:guarded-gitlab-merge`, based on
`98a91c8f3c9e87efb8429506bbbb67cb540d5950`. This is an integrated development
checkpoint, not a release, completed provider batch or dedicated sandbox result.
The implementation and this receipt belong to the same source commit; the input
manifest records its implementation, contracts, generated outputs and pins.

## Implemented path

GitLab selects native specification v3, reuses its existing ValidationInput and
MR observation model, and generates a separate private descriptor and consuming
PUT binding. Public/version-one descriptors and generated read-runtime bytes
are unchanged. Native preflight runs the existing generated validation read;
commit sends one exact SHA-guarded PUT. The finish binding verifies captured
numeric MR/project identity, IID, SHA and merged state, including path selectors.
Documented 403/405/409/422 replies are Refused; ambiguous/lost/malformed/wrong-target
replies are Unknown. Projection failure after known application preserves Applied.

The private owner/2 exchange carries protected proof bytes separately from
business input and a Linux monotonic deadline captured before CLI parsing.
The owner coordinates current proof/policy/key checks, exact credential capture,
audit acknowledgement before provider preflight, renewed clock/current admission,
audit confirmation, attempt/key preparation, approval spend, connection dispatch
and the original live attempt gate. Native preparations are consumed or cancelled
once; neither protocol layer retries a write. The child uses the earlier of its
wire deadline and the original owner budget.

`operations invoke` adds optional `--approval-file` and `--idempotency-key`.
The protected file is bounded and owner-private. Exact-key observation precedes
file/custody/clock/provider access and process startup. Native results carry
request, mutation and audit observations; safe errors preserve effect knowledge.
Read requests reject both write-only options and retain their existing protocol.
Current result-access admission is required even on retained-key replay.

## Verification

Linux x86_64; task-owned `TMPDIR=.local/tmp/gitlab-runtime-20260910`,
`CARGO_TARGET_DIR=$TMPDIR/target`, `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`.
Cargo runs were serialized. CLI fixtures explicitly selected the built
`$CARGO_TARGET_DIR/debug/connectors`; no installed configuration was migrated.

| Command | Result |
|---|---|
| Pinned `ess specify validate/compile --path adapters/gitlab/spec/ess` | Native root valid; four files. The unchanged native values are reused by the new declaration. |
| `cargo run --locked --offline -p connectors-spec -- --generate --specification adapters/gitlab/spec/adapter.json --output adapters/gitlab/generated` | Generated the v3 bundle from the existing pinned upstream; no vendor refresh. |
| `cargo run --locked --offline -p connectors-build -- cli` | Ten generated artifacts; 81 structural cases and existing consistency cases pass after the UUID projection correction below. |
| `cargo test --locked --offline -p connectors-gitlab --test merge_requests` | Fourteen tests pass, including fresh-preflight refusal and exact native outcome classification without retry. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Full gate passes: shared/native ESS, CLI and adapter generation/drift, workspace tests, Clippy, boundaries, Rust 1.88 and conformance. Host unit suite: 124 passed, 19 explicitly ignored. Conformance: 315 scenarios, 34 authored, 22 existing synthesis refusals. |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | All seven production CLI journeys pass in 290.91 seconds, including the new three-outcome merge journey and all six previous read/lifecycle journeys. |
| `connectors-build docs` and `connectors-build docs --check` using the freshly built project binary | 46 selected contracts, 99 total reference pages; generation/check pass. |
| `npm run typecheck`, `npm run build`, `npm run test:examples`, `npm run test:browser`, `npm run test:ui` | All pass. Fifteen Rust example tests pass; both browser suites pass against the task-owned production server on loopback port 3193. Build audits 487 public files and indexes 120 pages. Logs are retained alongside this receipt. |

The new CLI journey runs Applied, Refused and lost-response Unknown cases in
separate disposable authorities. It counts exactly one PUT per case, and zero
business effects for the native 409 refusal. Reusing the proof under a new key
returns `approval_replayed`/NotAttempted with no second PUT. After owner shutdown,
proof-file deletion and stopping the keyring daemon, invoking the original key
returns the same attempt and original request with fresh delivery correlation.
It makes no provider or clock call and starts no owner. Conflicting input returns
`idempotency_conflict` without original-attempt disclosure.

This is CLI process restart plus passive observation after owner shutdown. It
does not prove a newly started owner's replay path or a crash between individual
storage acknowledgements. The exact closed recovery payload projection is present,
but automatic owner-crash recovery is not newly qualified by this fixture.

Generation tests reproduce the selected bundle in isolated output directories
and retain explicit v2 parser/import compatibility coverage after GitLab selects
v3. They prove generated-byte reproducibility, not distributable/image digests.
The lock adds only the generated write-type package and adapter references to
already pinned test cryptography/base64 dependencies; registry versions do not
change. Cargo's generated type path dependencies appear as implicit workspace
members despite the excluded generated workspace directories; the gate checks
their bytes and formatting, and no generated source was edited manually.

## Corrections and remaining scope

The first CLI generation refused:

```text
error: unsupported CLI primitive `Uuid`
```

The CLI now uses a presentation projection whose attempt ID is a canonical UUID
string; the owner validates the UUID, correlation and outcome combination. This
does not change the stored attempt identity or add a persistent entity.

The first native compilation tried to deserialize an ESS-generated output type;
those generated values intentionally have no Serde codec. Native finish now
constructs the typed output explicitly. The first combined CLI run passed Applied
and lost-response cases, then failed because the test expected `service_failure`
for the native refusal. Existing safe mapping returns `forbidden`; the assertion
was corrected without changing production classification or effect counts.
Clippy also required removing an unnecessary `drop` of a borrowed admission view;
the owned key/policy leases are still explicitly released before a winner recheck.

Owner-crash acknowledgement cases, final-admission failure/audit completion,
concurrent repair/revoke/dispatch cases across this joined coordinator and the
wider required failure matrix remain open. Dedicated GitLab sandbox access,
C14 create/update atomic-head semantics, other selected GitLab workflows and
reproducible distribution also remain open. The story and parent initiative are
not closed. Kubernetes including Helm, then PostgreSQL, MCP and remaining
providers retain their agreed order.

Only primary `main` is in the Connectors checkout inventory; root is the sole
implementation/planning writer. Atlas bot authority was checked against clean
local and remote main at `1e9ea6546fcecbc87335d9f407f17790c296cc4e`; no Atlas
registration, release tag, source push, deployment or managed-tree cleanup is
part of this checkpoint.
