---
format: aep.planning-md/3
id: story:google-export-limit-fixture-reliability
kind: story
status: implemented
title: Complete TLS fixture delivery at the Google export size boundary
relations:
- informed_by: release-plan:connectors-v0251-provider-acceptance
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/tests/google_reads_adversary.rs
- confidence: cited
  path: docs/evidence/google-export-ci-20261002
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T19:24:56Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
- {from: "proposed", to: "active", at: "2026-10-02T19:24:56Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-02T20:40:01Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":4,"review_outcome":1}}}
---
## Outcome and observed failure

Unblock the provider0.25.1 release by diagnosing and correcting the existing Google
Drive export boundary test's intermittent Unavailable at exactly4MiB, preserving
its exact-limit success and limit+1 capacity assertions. PR79 repositorygate run
37051606475 failed at google_reads_adversary.rs405 on candidateb0b42f5a3a85dc3621a5a263b717c9d748bb2932.
This file was not changed by the provider wave. Other PRchecks passed; do not merge.

## Red-capable loop

The unchanged integrated binary google_reads_adversary-3ffb38827e46a92d, run with
private TMPDIR .local/tmp/google-export-loop and exacttest
files_export_at_and_over_the_response_limit, repeated up to100times, reproduced
exactly Unavailable on iteration27 after26passes (0.18s failingrun). Retain baseline
binary/source hashes and all outputs. The initial cargo single-test invocation
without the gate's privateTMPDIR instead failed at fixture setup InvalidConfiguration;
it is a separate precondition error, not the reported symptom.

## Ranked falsifiable hypotheses

1. The TLS fixture drops buffered final response records because write_all does
not guarantee flush. A flush-only correction should eliminate the boundary failure;
a controlled real-TLS backpressure case should be red without that completion.
2. A server write/early peer-close error truncates the response. Bounded transport
observations should show error/incomplete delivery; flushing alone would not cure
an independently failing write. Retain over-limit peer-close behavior distinctly.
3. Child framing rejects the boundary-sized native result. A fixture known to have
fully delivered the response would still produce Unavailable; exact framing
observations would separate this from transport loss. No frame/response/deadline
limit may be increased to make the test pass.

## Scope and acceptance

Primary source scope is adapters/catalog/tests/google_reads_adversary.rs only:
fixture completion and a deciding regression at the actual transport seam if
needed. Root owns docs/evidence/google-export-ci-20261002, AEP and release notes.
No production/dependency/limit/timeout/guard changes without reproduced evidence
and explicit scoped coordinator extension. All running code Rust; CLIclap derive;
no Python. Managedisolatedworker tree, own lease/defaulttarget,2jobs,sccache,
privateTMPDIR,8GiBdiskreserve. No source commits or publication by worker.

Loop first, one variable per probe. Retain original CI/localred, separately label
other failures, prove the chosen cause, retain decidingred→green, rerun original
boundaryloop and complete affected testbinary, Clippy/fmt, and adversarial review.
Root repeats affected source gates on the updatedcandidate and requires allCIchecks.
No blind CI rerun, skipped assertion, inflated limit or unconditional retry in SUT.
One bounded release-fix story, not a multi-storydecomposition; no new noun/model.

## Implemented candidate — 2026-10-02

The missing flush hypothesis is confirmed at the fixture's actual shared write
path. A real verified rustls connection over a64-byte duplex stream loses response
bytes without flush (UnexpectedEof) and completes after adding only flush. The
initial handshake/ticket timeout is a distinct retained fixture setup observation.
The original production-child boundary test passed100times with flush alone and
100times with the final shared helper/regression. Whole affectedbinary6passed,
scoped Clippy and packagefmt passed. All existing assertions and production inputs
remain unchanged. Source SHA256
5f81c2f93c5168a40209f1aefa7bc8006bef6c03b1ddda77d6e7f84fe4608b0a.

Public evidence: docs/evidence/google-export-ci-20261002/README.md. Original raw
logs, red/green patches and binary identities remain privately retained. Independent
review and integrated gate are in progress; this observation does not merge PR79
or establish the source release. Broad cargo fmt --all follows excluded generated
CLI dependencies; docs/development.md instructs authored-package formatting, which
passed. No generated source is edited to satisfy that unrelated broad command.

## Corrected candidate integration — 2026-10-02

The actual TLS fixture writer now flushes buffered ciphertext before dropping its
stream. The new regression was red without flush and green with it; original
boundary loops passed100/100 twice. Separate bounded adversary review found no
concrete defect; its immutable record is review-result:google-export-fixture-20261002.
No production, dependency, limit, deadline or existing assertion changed.

The full local gate --msrv passed:1208ordinary tests,0failed,65ignored, Clippy,
authored formatting, source/CLI/descriptor/model checks and Rust1.88library plus
Rust1.91workspace/all-targets checks. Contract synthesis retains21refusals and
498emittedscenarios; metadata execution289passed retains Inconclusive due to
undeclaredcoverage. Website references and build/publicaudit passed (482files).
Unchanged website presentation/examples/browser inputs retain prior passing checks.
Source/evidence: docs/evidence/google-export-ci-20261002/README.md.

This completes the scoped local fixture correction. Fresh required CI on the
updated PR79 head, App merge, exact immutabletag and hostedrelease checks remain
required. The earlier failed CI and initial candidate's checks remain historical;
no version release is claimed by this observation.
