---
format: aep.planning-md/3
id: release-plan:connectors-v0251-provider-acceptance
kind: release-plan
status: active
title: 'Release 0.25.1: verified provider acceptance and precise SQL refusals'
relations:
- informed_by: specification:wave-20261002d-provider-acceptance
- serves: vision:independent-contract-adapters
revision: 7
transitions:
- {from: "draft", to: "active", at: "2026-10-02T16:50:11Z", actor: "human:timo", revision: 2}
---
## Outcome and standing authorization

Prepare patch release0.25.1 for the verified provider-acceptance handoff under
approval-record:milestones-delivery-20261002. Remote main was verified at
b1f432766057fb8b7db60736363926cc44fe0db8 and the0.25tag namespace contains only
v0.25.0 on2026-10-02. Recheck before tagging. Patch scope reflects the exact0A000
SQL refusal correction and acceptance/tooling/documentation changes; it introduces
no new provider capability or protocol version.

## Candidate scope

Reviewed PostgreSQL sourcec3e20e1df82ccd4bc33b25c2943d3059ae0b51e1 and Kubernetes
source8ce90cb6d0f765a78f4273ca0ccade1ed7253a35 are integrated locally. Include current
GitLab18-read evidence and explicit initial revalidation refusal, exact ignored
runner classifications/prerequisites, and catalog lifecycle/mutation acceptance
only after its remaining helper/recovery/fault assertions and adversary review
complete. The rejected bounded clock experiment remains a recorded decision;
production clock source is unchanged and Entity Runtime51/M1 remains open.

Catalog is still executing. This plan is not release readiness. Do not describe
unexecuted catalog variants or failed fault injection as passing. If a requested
obligation proves infeasible, record its specific blocker and narrow the released
scope explicitly while retaining the unmet goal; never silently close it.

## Owned delivery surfaces and checks

Root owns Cargo.toml workspace version, generated Cargo.lock workspace entries,
README.md, CHANGELOG.md, docs/development.md, website/docs/adapters/{gitlab,kubernetes,sql}.mdx,
public evidence, generated owning projections/reference data as required, and AEP.
Provider workers retain their test/native guide scope until frozen. No source
version changes until the release candidate's final scope is established.

Required verification: pinned ESS/AEP validation and projection/conformance checks,
repository gate with MSRV, selected provider evidence with unchanged-input reuse
explicit, compiled ignored inventory with no unknown entries, affected website
checks after npm ci. Test/docs-only changes do not justify unrelated live reruns.
Capture full AEP output including historical warnings. Preserve exact candidate
source/tree identity and bot author+committer for every direct commit.

Publish branch through b10x-gates bot; create/merge PR through bot App API, required
checks green; verify exact remote main merge/tree before immutable annotated tag.
Verify tag workflow, GitHub release author/latest identity and required source
archives. Report queued until all source-release requirements succeed. Docs
publication remains asynchronous; no Atlas/Website delivery/consumer pin work.
Managed worker commits must be published before finish; cleanup uses reviewed
exact worktree ids and recovery evidence. No force/manual removal.

## Completion boundary

This source release cannot close the parent initiative, remaining Kubernetes/Helm
semantics, sustained ER workload, the unexplained isolated GitLab revalidation
observation or MCP delivery. The three product decisions stay open. This is one
release artifact over already-reviewed work, not a new multi-story decomposition;
a new decomposition critic panel is not applicable.

## Release documentation reconciliation — 2026-10-02

Root also owns website/docs/introduction/status.md for this handoff. The source
audit found stale fixture-only GitLab claims and future-sounding PG/K8S acceptance;
these are corrected to the actual reviewed evidence. README and the GitLab guide
now list all eighteen already-shipped read operations. The PG evidence table names
five CLI cases and one direct native cancellation case. Catalog remains explicitly
partial until all required variants and review finish. Versions are still unchanged.
Audit report SHA2566e103fcb1fbe4c5b8828deddc20914b21b2c836e87d4015fb4741886134d0704.

## Candidate freeze preparation — 2026-10-02

Catalog now has passing evidence for all10logical/15historical variants, with its
introduced test-cleanup safety defect corrected by a red/green boundary control.
The focused correction review and integrated gate remain pending. The version
input is now0.25.1; Cargo regenerated exactly13workspace lock entries with no
upstream dependency change. CHANGELOG, README and website status describe this
candidate and its explicit evidence reuse, CLI/native seam and remaining limits.
These source labels are release preparation, not a claim of publication.

Website npm ci and typecheck passed; installation reported29dependency audit
advisories (1low/25moderate/3high). No dependency versions were changed and audit
remediation is outside this patch's accepted source scope. Full website and Rust
MSRV gate checks remain required. Remote main is stillb1f432766057fb8b7db60736363926cc44fe0db8;
v0.25.1 does not yet exist. The catalog source correction is frozen under manifest
b545c005ffa28b7dbd19605909d388dde3fd843a02dcd805509ab5137b625c8f.

## Integrated release candidate ready — 2026-10-02

All required local checks passed. The full gate --msrv exited0:1207ordinary
passed/0failed/65ignored, Clippy/fmt/production/independent-model/source/CLI checks,
Rust1.88 libraries and1.91workspace. Metadata conformance289passed/0failed retains
Inconclusive coverage-undeclared status; synthesis498scenarios/43authored/21refusals
is not498executions. Compiledignoredinventory65cases/134binaries/unknown0 executed0,
selected41/missing38/skipped65 in the intentionally unconfigured inventory.
Selectedprovideracceptance remains its separately reviewed evidence.

Website npmci/typecheck/build/referencecheck/examples15/browser/UI all exited0;
482publicfiles audited withoutprivatepaths. No dependency upgrades or deployment.
Detailed commands/counts/limits: docs/evidence/provider-release-20261002/README.md.
The author/reviewer/public reports preserve all earlier reds and scoped evidence reuse.
Final release prose review found no concrete issue. Catalog, PG real acceptance,
exact SQLSTATE correction and Kubernetes acceptance stories are implemented.

Remote publication is still pending. Next: bot commit/push, App PR, requiredchecks,
App merge, verifyexactmain/tree, immutableannotatedv0.25.1tag, successfultagchecks,
App-authoredLatestrelease and sourcearchives. Release plan staysactive until verified.
MCP and the sustained-load/upstreamclock and decision-gated provider remainder stayopen.

## CI refusal and scoped correction — 2026-10-02

PR79 candidateb0b42f5a3a85dc3621a5a263b717c9d748bb2932 failed its repository gate
(run37051606475) in existing files_export_at_and_over_the_response_limit,
google_reads_adversary.rs405: exactly4MiB returned Unavailable. OtherPRchecks passed.
The complete localgate had passed; neither that green nor a blind rerun discharges
this observed CI failure. Merge/tag/release remain held.

The unchanged test reproduced the same symptom locally on iteration27 of a
100-iteration bounded loop after26passes, with a private gate-style TMPDIR.
Loop logSHA7fce34011d0bf245ae7018067338a572c10b6e6ff0bd8b1c49750de78ba1c0ec;
CIlogSHA894320c580b62b002e7295e984c9a479d4f03c01fada4c9bb5993d48d6cbcb66.
An earlier no-private-TMPDIR single invocation failed fixture setup instead;
it is retained separately, not counted as reproduction of this symptom.

Active story:google-export-limit-fixture-reliability owns the bounded correction.
Worker cb26e-export at exactpublishedcandidate may change only the existing Google
adversary fixture/regression, starting with ranked one-variable probes. Production,
size/deadline limits, guards/dependencies stay unchanged absent a separately proven
and scoped defect. Root owns store/evidence/release notes and integration.
Fullscope/rankedpredictions are recorded in that story. Independentreview and an
updated requiredCI run must precede release. The existing four provider stories'
reviewed unit evidence remains valid; the source release is not yet complete.

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
