# GitLab owner restart, crash and revocation evidence

Local development increment under active `story:guarded-gitlab-merge`, based on
`bb9fda9fdb4bb22fc45a7224a2ae18c57859896f`. This receipt and implementation belong
to the same source commit. It is not a completed GitLab batch or source release.

## Change and runtime evidence

The coordinator previously propagated a failed current-admission recheck before
finishing an already acknowledged execution audit. Both affected pre-dispatch
failure paths now complete that audit before returning the refusal. Native
preparations are still destroyed before the final admission failure is returned.
Current disclosure admission is preserved; no original-result authority is added.

The production CLI fixture now exercises four outcomes: applied, native refusal,
lost provider response, and owner death after the provider receives/applies a PUT
but before it supplies a response. Every case proves exactly one PUT. The native
refusal causes zero business effects; the other three cause exactly one.

For owner death, a concurrent owner/2 observer first sees the pending attempt
without changing it. The fixture obtains the exact owner's kernel pidfd through
its private socket using SO_PEERPIDFD, kills that owner, and polls held handles
to verify both the owner and its one native child exit. It never signals a
numeric PID. The original CLI receives outcome_unknown. After a new owner starts,
the same attempt and original request remain observable without another PUT.
The original approval is refused under a different business key after restart.

For all four outcomes, a separate admitted read starts a new production owner and
the fixture verifies its incarnation differs. Direct owner/2 observation then
returns the original mutation with fresh request/audit correlation, a deleted
proof file, no supplied proof and custody stopped. Provider and clock call counts
do not change during this observation. The crash fixture retains a protected
in-memory copy solely for the subsequent proof-spend persistence check.

A separate deterministic case holds the first native preflight response, checks
that the execution audit is admitted, then revokes the connection through the
production CLI. It proves the invocation refuses as revoked, sends zero PUTs and
finishes the exact original audit with a refused observation and revoked code.
It reproduced the missing final observation before the coordinator fix.

## Verification

Linux x86_64, kernel 6.18.49-1-MANJARO. The crash test requires SO_PEERPIDFD.
Task-owned TMPDIR is `.local/tmp/gitlab-runtime-20260910`; CARGO_TARGET_DIR is its
`target` subdirectory. Cargo jobs and ordinary Rust test threads are bounded at
two; runtime journeys are serial. CLI fixtures explicitly select the built
`target/debug/connectors` through CONNECTORS_TEST_CLI. Cargo runs were serialized.

| Command | Result |
|---|---|
| `cargo build --locked --offline -p connectors` | Updated production CLI built before acceptance. |
| `cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | All eight disposable production CLI journeys pass in 421.43 seconds, including the stronger restart/crash case, revocation regression and all previous read/lifecycle journeys. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Full gate passes: shared and six native ESS roots, generation/drift, workspace tests, Clippy, boundaries, Rust 1.88 and conformance. Host unit tests: 124 passed, 19 explicitly ignored. Conformance: 315 scenarios, 34 authored, 22 existing synthesis refusals. |
| `npm run typecheck`, `npm run build` | Pass. Build regenerates selected references/examples, checks links/public output and indexes 120 pages; 487 public files pass the private-path audit. CSS minimization emits missing-font-size warnings, retained in the log. |
| Built `connectors-build docs --check` | Pass; 46 selected contracts and 99 total reference pages are current. |
| `git diff --exit-code bb9fda9 -- adapters/gitlab/generated apps/connectors-cli-contract` | Native and CLI generated outputs remain byte-identical. |

The first revocation fixture timed out because its barrier compared the entire
request path with a route while the generated request included a query suffix.
The barrier now compares the route. The corrected fixture then failed on the
actual missing final audit observation. Both failures and the passing final run
are retained, without changing provider behavior to make the regression pass.

The gate rebuilds workspace executables with a different Cargo selection from
the package-specific acceptance commands. `gate-artifacts.sha256` records those
intermediate bytes separately. Re-running the exact package build and native
test compilation (`--no-run`) restored both originally tested executable hashes;
the final checksum verification passes. No runtime evidence is attributed to
different binaries. This is not proof of two isolated distributable builds.

`source-inputs.sha256.gz` records 188 selected source/dependency inputs, with the
base commit retaining unchanged surrounding inputs. `runtime-artifacts.sha256`
records tested CLI/adapter bytes and dependency pins. Test-only libc/rusqlite
references use versions already in Cargo.lock; no registry version changes.
Website presentation and browser/example implementation inputs did not change;
their previous checks remain in the preceding guarded CLI receipt.

## Remaining work

The owner-crash attempt remains pending and conservatively unknown. This increment
does not automatically recover abandoned attempts into terminal/quarantined
records, establish crashes across every storage acknowledgement, or close the
broader concurrent admission/final-audit failure matrix. Dedicated GitLab sandbox
access, C14 create/update atomic-head semantics, other required GitLab acceptance
and reproducible distribution remain open. Root requested the missing sandbox
URL, project and protected credential-file path while continuing independent work.

The story and parent initiative remain active. Full GitLab still precedes
Kubernetes including Helm, PostgreSQL, MCP and remaining providers. Root is the
sole implementation/planning writer on primary main. Only the primary checkout
is present; no linked tree, release tag, push, deployment or Atlas registration
is created by this increment. Atlas bot authority was rechecked against clean
local and remote main at `1e9ea6546fcecbc87335d9f407f17790c296cc4e`.
