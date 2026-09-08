---
format: aep.planning-md/1
id: verification-report:independent-repository-followup-20260908
kind: verification-report
status: draft
title: Independent repository review reconciliation and reproductions
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: review-result:independent-repository-a-20260908
- informed_by: review-result:independent-repository-b-20260908
- informed_by: review-result:concept-stack-integration-20260908
revision: 2
---
## Result and baseline

Both independent major findings reproduce against local main `4b44916488a47c6a5d699552ee1d9c2ed3904938`. Fix SQL cancellation and the intermittent toolchain fixture failure before the next semantic hardening wave. This investigation records the remaining findings and qualifications; it does not claim runtime remediation or approve a new implementation wave.

The operator asked to investigate the two reports after committing and cleaning up the auth wave, then approved showing the content. This is an interactive investigation. The original reports reviewed `8903166`, before the F04/F05 wave. The SQL adapter, toolchain fixtures, client error decoder, federation schema and public import implementation are unchanged since that baseline. The gate changed to include the new ESS scenarios, but its MSRV target override remains.

The reports are preserved verbatim as [review A](../review-result/independent-repository-a-20260908.md) and [review B](../review-result/independent-repository-b-20260908.md). Their 24 source rows retain separate IDs below. Prefixes distinguish them from the earlier contract intake's F01–F15 and E01–E33; overlapping rows are not independent defects.

The historical [concept and stack integration review](../review-result/concept-stack-integration-20260908.md), cited by both reviewers, is now preserved too. Its G1–G10 recommendations, mutable sibling-checkout observations and proposed wire version remain historical review content. Preservation supplies no new organization authority, versioning decision, Atlas integration approval or consumer migration scope. The two proposal documents now link to that tracked source.

## Major findings reproduced

### B-F01: toolchain fixture race — P1, open

`crates/connectors-spec/src/toolchain.rs:136` writes executable shell fixtures and then probes them from three concurrently running tests. An executable may still be open for writing in a sibling child process during the fork/exec window. The observed failure is `Text file busy (os error 26)`, not a wrong ESS pin. The configured pin remains 0.20.0.

- Twenty invocations of `cargo test --locked --offline -p connectors-spec --lib` passed with default test threads.
- One hundred subsequent invocations of the resulting test binary, also with default threads, failed twice: runs 24 and 59. Both failures were ETXTBSY. The aggregate is 2 failing test-suite runs out of 120; the direct-binary cohort is 2/100.
- Reviewer B independently observed 5/11 parallel failures and 0/3 serial failures. These are separate experiments with different scheduling; neither pass rate is a reliability guarantee.

The retained [stress summary](../../../docs/evidence/independent-review-followup-20260908/toolchain-stress-direct.json) and [full direct-run log](../../../docs/evidence/independent-review-followup-20260908/toolchain-stress-direct.log) identify the failures. The [Cargo-run summary](../../../docs/evidence/independent-review-followup-20260908/toolchain-stress.json) and [log](../../../docs/evidence/independent-review-followup-20260908/toolchain-stress.log) preserve the preceding passes.

Remediation should eliminate concurrent fixture writes and process launches within this test harness, or use immutable executable fixtures. Synchronization must cover the complete write/probe interval of all three affected tests. Retain the resolver precedence and wrong-pin refusals. Do not hide the race with production resolver retries or a globally single-threaded gate. Acceptance needs repeated default-thread runs and the ordinary gate, with no retry-until-green policy.

### B-F02: SQL deadline bypass leaves a running backend — P1, open

`adapters/sql/src/lib.rs:119` installs `statement_timeout` once; the row loop issues separate portal executions; `ConnectionTask` aborts the socket driver on drop without sending a PostgreSQL cancellation request. The outer timeout bounds the adapter's wait, not the remaining server work. The implemented service contract's mandatory deadline and cancellation claims are therefore too strong.

Rebuilt `connectors-sql` and `connectors` from the current checkout with `cargo build --locked --offline -p connectors-sql -p connectors`. A disposable PostgreSQL 17.11 container used cached image `sha256:1bea307dfb3ee30541a7acf7de14b58bcd6948da98e5d31a04c627c4d35ec64b`, a fresh non-superuser `reader` role and loopback-only plaintext transport. Both requests used `query.read` with `limit: 10` through the real CLI and service.

```sql
-- Control
SELECT pg_sleep(30)::text AS value

-- First portal row disables the timeout used by the following execution.
SELECT set_config('statement_timeout','0',true) AS value
UNION ALL SELECT pg_sleep(40)::text
```

| Observation | Control | Bypass |
|---|---|---|
| CLI exit | 1 | 1 |
| Response | `timeout`, database could not complete the requested read | `timeout`, database request timed out |
| Response elapsed | 10.08 s | 15.03 s |
| Active backend immediately after response | none | PID 107, active at 15 s |
| Three seconds later | not applicable | PID 107, active at 18 s |
| After graceful adapter exit | not applicable | PID 107 still active at 18 s |

The [live transcript](../../../docs/evidence/independent-review-followup-20260908/sql-live.jsonl) records all observations and cleanup. The surviving backend was explicitly terminated in this disposable database; the adapter exited 0 and the container was stopped and removed. Repeated-request connection exhaustion was not exercised; it remains an impact inference from the independently surviving backend. TLS cancellation and network-loss behavior were not tested here.

PostgreSQL restarts statement timing across extended-protocol command boundaries, and zero disables the configured timeout. See [PostgreSQL 17 timeout semantics](https://www.postgresql.org/docs/17/runtime-config-client.html#GUC-STATEMENT-TIMEOUT). Cancellation uses a separate connection and matching backend key; sending it is not proof that it took effect. See [PostgreSQL 17 cancellation protocol](https://www.postgresql.org/docs/17/protocol-flow.html#PROTOCOL-FLOW-CANCELING-REQUESTS).

Remediation must combine an adapter-owned deadline with bounded server cancellation and connection cleanup, including when the invocation future is dropped. Preserve the admitted endpoint and TLS trust for cancellation. Decide and document which database-enforced limits can actually survive caller-controlled session settings; another mutable timeout alone is insufficient evidence. Test the bypass against PostgreSQL, assert eventual backend termination when cancellation is reachable, and keep claims about unreachable databases or crashed processes explicit. Expand the wire fixture to check timeout/setup statements, parameters and cancellation framing. No SQL function-name blacklist or version bump is implied by this fix.

## Disposition of every source finding

“Open” means no remediation has been made in this investigation. “Qualified” retains the observation but limits the claimed defect or scope. Priorities below are investigation recommendations; they do not silently change existing story priorities.

| Source | Review severity | Disposition and next action |
|---|---|---|
| A-M1 | minor | Confirmed open, P3. Current GitLab description in `docs/design.md:1247` still says ESS 0.9.2; the pin and bundle use 0.20.0. Shares the version-drift work with A-M2/B-F04. Leave explicitly historical baseline versions intact. |
| A-M2 | minor | Confirmed open, P3. `adapters/gitlab/upstream/README.md:28` calls the retained refusal 0.9.2; it is the 0.20.0 import refusal. Same documentation work as A-M1/B-F04. |
| A-M3 | minor | Confirmed open, P3. Reconcile index coverage, deferred configuration and row/document counts under existing `story:contracts-documentation-index`; include the proposed service document without endorsing its version choice. Overlaps B-F05. |
| A-M4 | minor | Confirmed open, P3. The implemented mutation and idempotency story bodies still say uncommitted. Replace current-state claims with commit references `34f298a` and `8903166` through AEP, preserving the historical checkpoints. |
| A-M5 | minor | Qualified documentation clarification, P3. The design artifact explicitly records the original design-only handoff, so later implementation is not evidence that its original scope was false. Add a dated delivery-status pointer to the later stories and design §27–30; do not rewrite historical evidence or infer lifecycle approval. |
| A-M6 | minor | Addressed here. Preserve the exact G1–G10 review and replace both ignored-source dependencies with tracked links. Same source-portability finding as B-F06; no G recommendation is implemented or approved by preservation. |
| A-M7 | minor | Confirmed open, P2. `crates/connectors-build/src/gate.rs:129` still overrides caller `CARGO_TARGET_DIR` for MSRV. Resolve the caller-selected target base and isolate MSRV beneath it; verify the private-target gate leaves default `target/msrv` untouched. Duplicate of B-F03. |
| A-M8 | minor | Confirmed open, P3. The proposed CLI owner is not an AEP artifact. Mark the owner proposed/unassigned until the governed work is actually created; do not invent an approved story or settle its seven decisions. |
| A-N1 | nit | Coverage follow-up, P2 for regressions tied to confirmed defects, otherwise P3. The cited tests omit several credential, capacity, federation, discovery and SQL TLS branches. `schema.list` does have a live conformance path, so “no coverage” would overstate the gap. Overlaps B-F09; no new runtime defect follows from missing tests alone. |
| A-N2 | nit | Confirmed capability gap, scope decision deferred. The downstream client has no private-CA configuration. This does not break documented loopback or publicly trusted TLS ingress. Record the limitation; a private-CA federation feature needs its own bounded contract/configuration decision. |
| A-N3 | nit | Confirmed documentation ambiguity, P3. The mutation invocation example labels new fields `v1alpha1`, which the implemented strict decoder refuses. Label it illustrative/proposed and refer wire encoding to existing `story:contracts-wire-compatibility`; do not choose v1 extension versus v2 as an incidental edit. |
| A-N4 | nit | Confirmed documentation ambiguity, P3. The proposed service summary says unchanged error-envelope shape while adding response data and error codes. Clarify which structure is preserved and which proposed bytes change; retain the unresolved implementation/version boundary. |
| B-F01 | major | Reproduced, P1 open. Fix the fixture race; stress and ordinary-gate acceptance described above. |
| B-F02 | major | Reproduced, P1 open. Fix SQL cancellation and align the service guarantee with enforceable behavior; live-backend acceptance described above. |
| B-F03 | minor | Confirmed open, P2. Same MSRV target override as A-M7; one fix and one verification account for both rows. |
| B-F04 | minor | Confirmed open, P3. Same current ESS drift as A-M1/A-M2, plus design status header stops at §27–28 although §29–30 exist. Update the current summary, not dated observations. |
| B-F05 | minor | Confirmed open, P3. Same index issue as A-M3. Reviewer wording conflates rows and documents; the exact inventory below corrects the counting unit. |
| B-F06 | minor | Addressed here. Same tracked-source preservation and proposal-link repair as A-M6. |
| B-F07 | minor | Confirmed by source, P2 open. Federation advertises only `{"type":"object"}` although its config is typed and closed. Derive the descriptor schema from the actual configuration owner and verify invalid configs are rejected by the advertised schema. No live federation reproduction was needed to establish the placeholder. |
| B-F08 | minor | Qualified limitation, P3 documentation; ingress work deferred. The semaphore and timeout cover adapter execution after the body has been read. They are not connection-count or complete request-read bounds. The contract says “operations,” so that phrase itself is not a false connection-cap claim. Do not advertise this local profile as bounded public ingress. |
| B-F09 | minor | Confirmed test-assertion gap. The SQL fixture ignores Parse and Bind bodies and only partly checks setup; the wire round-trip asserts request ID but not outcome. Add meaningful assertions alongside the corresponding fixes. Keep broader branch coverage distinct from a demonstrated defect; overlaps A-N1. |
| B-F10 | nit | Qualified; duplicate acceptance not reproduced. `Error` is a closed derived struct with scalar fields: duplicate code, message and retry fields and an unknown nested object all produce the fallback error, not an accepted ambiguous value. Using shared `read_json` would be consistency cleanup, not repair of a proven parsing bypass. See the probe evidence below. |
| B-F11 | nit | Qualified trusted-boundary observation. No `Debug`/`Serialize` implementation prevents accidental formatting/serialization of `Secret`; it never prevents a holder from deliberately exposing plaintext needed by transports. A private field plus a bytes accessor would not provide that stronger guarantee. No exfiltration path was demonstrated. |
| B-F12 | nit | Reproduced public-library panic, P2 open. Parse a valid GitLab spec, clear its public `operations`, and call public `v2::import`; it panics at the lookup unwrap. Return a typed refusal or enforce a validated input type. The normal parse-before-import CLI path remains guarded. |

## Additional checks and qualifications

The index has **17 `semantics.md` documents**: one implemented service document and 16 proposed documents. Its **16 proposed rows point to 15 distinct proposed documents**, because catalog is indexed twice. `contracts/service/v1alpha2/semantics.md` is the unindexed document. Five implemented rows share `service/v1alpha1`. The deferred list contains three names and omits configuration. Counting rows as files is the reason several review counts appear inconsistent.

The [error-decoding probes](../../../docs/evidence/independent-review-followup-20260908/error-decoding.json) served HTTP 403 bodies to the current CLI's `describe` command over a disposable loopback fixture. The valid control preserved its message. Duplicate `code`, `message`, `retry_after_seconds` and an unknown nested-object case all returned the generic invalid-response fallback. This tests the actual `parse_error` path, not just a separately chosen deserializer.

The [public import probe](../../../docs/evidence/independent-review-followup-20260908/public-import.log) linked the current `connectors-spec` Rust library and called:

```rust
let path = std::path::Path::new("adapters/gitlab/spec/adapter.json");
let bytes = std::fs::read(path).unwrap();
let mut spec = connectors_spec::v2::Spec::parse(&bytes).unwrap();
spec.operations.clear();
let result = std::panic::catch_unwind(|| connectors_spec::v2::import(&spec, path));
assert!(result.is_err());
```

This establishes the public API misuse case without changing the compiler or generated bundle.

## Recommended sequence and boundaries

1. Repair SQL deadline/cancellation behavior with live PostgreSQL regression evidence, and remove the toolchain fixture race. Both are P1 and should precede another semantic wave. They affect separate implementation files, but shared gate/evidence/planning edits still require coordination if later dispatched concurrently.
2. Repair MSRV target isolation and the federation configuration schema; make public import misuse return a refusal. Add tests tied to those behaviors.
3. Reconcile current documentation and planning statements, folding A-M3/B-F05 into the existing index story and keeping wire-compatibility questions with their existing owner. Address broader test gaps by affected behavior, not by adding a single blanket coverage story.

No new implementation stories were decomposed in this investigation, so no planning-critic panel or implementor dispatch ran. The existing contract intake still owns its original 48 source findings; these 24 review rows are separately traceable here. No ESS domain or runtime entity was added. The auth wave's structural ESS proof remains distinct from executable SQL or future auth-runtime proof.

## Workspace and verification scope

The three completed auth-wave worktrees were finished and removed through exact-ID managed GC after establishing the local bare recovery remote. Cleanup is committed as `4b44916`; its [receipts](../../../docs/waves/auth-hardening-20260908/integration/managed-cleanup.json) record the IDs. The two unrelated linked worktrees remain untouched.

Installed Connectors 0.7.0 reported a healthy local placement but no admitted operation for `docker container`. This capability gap was reported before using Docker locally. The test used one new container, no existing database, no external provider calls and no package download. All created test services, the container and temporary fixture credential files were cleaned up. No publication or Atlas registration occurred.

This pass ran targeted reproduction and source checks. It did not rerun the entire gate or claim a clean release gate; the reproduced intermittent failure remains open. Source byte preservation, evidence paths, disposition ID coverage and AEP validity are checked when recording this intake. The imported reviews retain their original prose format; any AEP warnings about missing machine-readable findings blocks are disclosed rather than repaired by rewriting the reviews.

## Subsequent remediation — 2026-09-08

The operator subsequently requested “good, solve them”. Corrections and passing regression evidence are recorded by [story:independent-review-remediation](../story/independent-review-remediation.md) and [the final response](../../../docs/independent-review-response-2026-09-08.md). The observations and open dispositions above describe the investigated 492c20c baseline; this follow-up supplies their later outcome without rewriting that evidence.
