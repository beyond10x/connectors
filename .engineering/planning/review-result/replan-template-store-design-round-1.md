---
format: aep.planning-md/3
id: review-result:replan-template-store-design-round-1
kind: review-result
status: active
title: Design critic, round 1, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision. Read at commit 198fc306b. S9d is the registry draft, S9e the MySQL
fixture draft, N6 the question story, N5 the question story of the tests under 5 s.

## Findings

1. S9e, "Established by reading" and acceptance 2. Blocking. The draft says nine binaries of
   `connectors-sql` include the fixture, 83 test functions. There is a tenth, in another crate:
   `crates/connectors-conformance/tests/incident_call_join.rs` lines 19 to 22 include
   `adapters/sql/tests/mysql/calls.rs` and `fixture.rs` through `#[path = …]`. The pre-push run
   of acceptance 2 would not cover it. Settles it: name the binary in the established list and
   in the pre-push run, and correct the count.
2. N6, acceptance 4 and item 11. Blocking. N6 reads state that S9d produces and records no edge
   to it. Items 1 to 8 are measured "on a head that holds" S9d. A `depends_on` edge to S9d
   should be added; it closes no cycle. Item 10 reads S9e's outcome conditionally: say that it
   waits and needs no edge.
3. N6 item 11 against N5. Blocking. Two stories own one question: the share of REG and PARSE2
   in the tests under 5 s. Settles it: one story reads S9d's gate job, not both.
4. N6, items 2 and 6. Note. Parts are answerable now by reading. Item 2: `execute_batch`
   returns `AppendOutcome` (`entity-eventlog/src/facade.rs` line 383); the readback in
   `persist_batch` is the post-commit verification (`er.rs` lines 4470 to 4540), a check this
   repository performs. Item 6: "by reading, which tests hold what a migrate without a step must
   still do".
5. S9d, design (a) to (d). No finding on the core. No caller mutates the registry; no `cfg`,
   feature flag, environment variable or store dependence in `registry()` or
   `definition_bundle()`; no test expects `registry()` to fail. `Failure` is `Copy` and carries
   only `MetadataUnavailable` (`local/mod.rs` lines 48 to 58), so a stored error reproduces
   exactly. The build is a pure function of the compiled-in bytes. The draft's "five callers"
   undercounts: eight more are in the test module (lines 6127, 6467, 6547, 6876, 7084, 7562,
   7598, 8299); all borrow, so the kept signature is safe. State this in (c).
6. S9d, acceptance 1. Note. The test covers only the registry half. A `persist_batch` that kept
   its second parse of the bundle would still pass "at most 1", and the time bound would not
   catch it. Settles it: put the seam in `definition_bundle()` as well. `COSTS` is thread-local
   (`er.rs` line 3170 onward).
7. S9d, clone cost. Note. Settled by arithmetic: 28 definitions and 73 commands in 469,676
   bytes; a deep clone allocates no more than the parse that builds the same structures, 3.04 ms
   per call. 436 clones are at most about 1.3 s, 1.1 % of 117.7 s. The clone is forced:
   `start_with_read_policy` takes `registry: Registry` by value (`facade.rs` line 185),
   `Registry`'s field is private, there is no `Arc` form.
8. S9d, `simulate_process` and a warm cache. No weakening found. `simulate_process` sets a
   thread-local process id that keys the idle-handle pool. The registry is built before
   `note_fresh_open` and `started` (`er.rs` lines 2484 to 2486), so the open duration recorded
   for `cutoff(*took)` (line 654) does not include it. `checkpoint_tests.rs` lines 385 to 440
   assert only upper bounds. Two installs: the bundle is compiled in. After a fork the static is
   copied whole.
9. S9d, concurrent tests. Note, unproven. Tests 2, 6, 8, 10 and 11 run several threads. A
   persist 13 ms faster shrinks the windows they race in; a test could stay green while
   exercising the race less. Settles it: read the five for barrier or hook synchronisation
   (`BEFORE_BATCH` exists) against timing, and name the result in the draft.
10. S9e, source. No defect. `start` uses `tokio::net::TcpListener`; the stream at line 211 is a
    tokio `TcpStream` with `set_nodelay(bool)` before it is boxed. The TLS path wraps the same
    socket (lines 420 to 425). The fallback holds under TLS. No second MySQL accept loop.
11. Stale ownership in sibling stories. Note.
    `eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing` lines 40 and 189 say
    `every_page_is_one_the_model_admits` and `ess_model.rs` belong to S9a; they belong to S9e.
    `five-host-unit-tests-shorten-the-waits-they-chose` line 91,
    `four-host-tests-no-longer-wait-out-the-30-s-lock-wait` line 118,
    `one-debug-only-override-helper-…` line 126 and
    `six-cli-tests-take-jail-binaries-built-once-per-run` line 139 name S9a's edits.

## Checked and found correct

- No `depends_on` cycle; nothing points at the archived story but the two `supersedes` edges;
  those follow existing practice and `validate` prints `valid`.
- Removing the parent's edge to S9a is right: neither S9d nor S9e writes `gate.rs`.
- No file named by S9d or S9e changed between d5cf22480 and f9f263a2.
- The call sites in `er.rs`, the order of checks in `registry()`, and the borrow-only use in
  `persist_batch` match the draft. `Registry` is `Send` and `Sync`.
- The parent's sums recompute.

## Not checked

- Whether the Nagle hold is the cause; the real cost of a clone (no run allowed).
- The synchronisation of the tests with several threads (finding 9).
- Items 1 to 8 and 12 of N6 against the dependency's internals.

## For another critic

- S9d's file is shared with `mcp-outbound-stdio-runtime`, `spec-models-operation-admission` and
  `connections-list-answers-through-the-owner` (active).
