# Independent stable-core semantic review A

Verdict: **needs-revision**. One finding: **SCA-01, P2**. No P0/P1 finding.

Exact reviewed commit: `eb0815961e6ffee54c50f91f37eff2bbe11811c0`.
Managed checkout: `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-review-a-20260909`.
Reviewer/session: `specs-review-a-20260909`.

This is the initial stable-core pass, not final approval of all specifications.
Tracked sources and planning remained read-only. The coordinator accepted SCA-01
for a later integration correction; that correction is not present in this
reviewed commit and is not treated as verified here.

## SCA-01 — P2 — Catalog value freedom permits the credential locators it otherwise forbids

Primary citation: `contracts/catalog/v1alpha1/semantics.md:140`.
The rule says that bundle files, descriptors, the index and every catalog output
contain credential references rather than credential values. That explicitly
allows private runtime locators on the distribution/public surfaces. It conflicts
with the same contract's no-credential-address rule at
`contracts/catalog/v1alpha1/semantics.md:130`, the authored profile's exclusion of
secret references at `contracts/auth/profile/v1alpha1/semantics.md:79`, and the
safe descriptor projection at `contracts/service/compatibility.md:60`.

Concrete counterexample: the receiver runtime configuration example in
`adapters/catalog/design.md:101` selects
`{"kind":"environment","name":"ZENDESK_TOKEN"}`. A catalog implementor can
copy that reference into an exported bundle or descriptor, contain no secret
bytes, and believe it satisfies the explicit rule at line 140. An implementation
applying line 130 rejects that disclosure. The two outcomes cannot both implement
the same distribution contract. File or custody version references have the same
ambiguity. This is a contradictory specification permission, not a claim that
the current runtime exposes such data or that a locator itself grants access.

Smallest coherent remedy: permit only safe auth-profile identifiers and
configuration requirements in authored/distributed catalog artifacts; exclude
actual environment/file/custody credential locators and values from bundle
content, descriptors, indexes and every public catalog output. Keep actual
references in receiver-owned admitted runtime configuration/private bindings.
Distinguish ordinary bundle artifact locations from credential addresses.

## Reviewed coverage and observations

- Service v1alpha1, v1alpha2 and compatibility: explicit codecs and paths, strict
  old-reader refusal, projection eligibility, singular profile selection, lookup
  and disclosure precedence, result/effect knowledge, audit acknowledgements and
  byte/deadline accounting. The existing legacy no-retry rule remains intact.
- Delegation: canonical leaf subject versus gateway presentation; optional
  tenant/realm/executor; exact request correlation and proof-purpose separation;
  bounded proof time, post-nonce acknowledgement recheck, leaf-only approval
  redemption, abort/open fencing, replay admission and no resend after uncertainty.
- Operations/idempotency: pre-gate and post-gate recovery, independent approval
  spending, immutable original outcomes, stable namespaces and complete
  fingerprints, authoritative miss/refusal winner recheck, current result
  admission, fixed known-result retention and no expiry of unresolved work.
- Auth connection/profile/acquisition/custody/capability/evidence and management:
  baseline viability versus per-operation eligibility; explicit anonymous/parent
  modes; immutable capture, identity lineage and current-generation admission;
  reserve/authorize/response-store/publication recovery; protected completion and
  instance/acquisition/connection target distinction; fixed scope and permission
  budgets; local revocation versus provider outcome.
- New read-refresh-once: explicit v1alpha2 native combined-profile selection,
  definitive complete 401 trigger, no implicit activation/identity probes, one
  selected source attempt, exact acknowledged current successor, a new terminal
  dispatch admission, two business requests maximum, one original deadline and
  permission-call ledger, refusal after cancellation/supersession/uncertain
  publication. The fourteen RR01–RR14 textual cases were read against their rules.
  No current adapter claims support; this pass found no additional semantic
  blocker in that binding.
- Catalog: optional inventory versus execution authority, safe error handling,
  generic read versus mutation outcomes, selected limits and no-retry behavior,
  provenance/format refusal, and the credential-reference contradiction above.
- Design §31 persistence inventory: singular logical owners, coupled binding and
  mutation metadata groups, separate custody/audit/nonce/spend acknowledgements,
  owner-specific retention and anti-rollback history. The remaining persistent
  Connection/Acquisition/AuthProfile/custody/Audit/discovery/assignment entity graph
  is expressly undeclared at `docs/design.md:1360`; this inventory does not claim
  complete storage modeling. Its named ESS/modeling gates remain prerequisites
  before entity-bearing implementation decomposition, not runtime faults found
  by this review.
- ESS declarations inspected for the reviewed protocols: service_wire,
  declarations, delegation, mutations, idempotency, credentials,
  credential_evidence, refresh, auth_access and connection_admission. Checked the
  declared identities/relations and terminal graphs against their prose owners.
  Trusted decision inputs, equality, live authority, byte pinning and atomicity
  remain explicitly unexecuted predicates; their absence from executable ESS
  checks is not mislabeled a newly discovered counterexample.

Historical context read includes the accepted F01–F15 source findings, the E01–E33
external intake, auth refresh/evidence adversary records, final federation and
idempotency reviews, and the profile/persistence recheck. Those reviews inform
scope; their prior verdicts do not substitute for this pass.

## ACO-S-04 residual ownership check

The immutable `docs/evidence/adapter-ownership-20260909/semantics-recheck.md`
correctly records its earlier native Kubernetes tuple leak. At this review's
exact commit, `contracts/service/compatibility.md:159` delegates the exact target
codec and native scope interpretation to the adapter and links the Kubernetes
auth binding. It retains shared coverage, reader, error and exhaustion semantics
without declaring the native seven-field tuple or all-namespaces Service rule.
**ACO-S-04 is resolved in this snapshot.** The old report is preserved unchanged.
No completed ownership audit or layout gate was rerun.

## Executed checks and limits

- Exact checkout HEAD verified as the commit above; `git status --short` was empty.
- Managed worktree session-start and heartbeat succeeded for this reviewer's own
  session. No other session's lease or worktree lifecycle was modified.
- Pinned tool: `ess 0.20.0`.
- `ess specify validate --path ess`: exit 0;
  `connectors v1 — 14 file(s), valid`.
- `ess specify compile --path ess --format json`: exit 0; retained output at
  `.local/spec-completion-20260909/shared-compiled.json`.

The ESS commands establish declaration/reference validity, not execution of
business retries, authentication, proof verification, clocks, persistent
transactions or provider calls. No new program, Python, runtime test, full Rust
build, integration call, tracked mutation or planning write was performed. No
model probe is claimed to prove SCA-01: it is a manual cross-clause contradiction.

Native logs/documents and the pending media/vocabulary/index changes were
deliberately excluded from this initial pass. The coordinator will provide a
final integration commit for their review, SCA-01 recheck and a whole-spec verdict.

```findings
[
  {
    "file": "contracts/catalog/v1alpha1/semantics.md",
    "line": 140,
    "category": "semantics",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "pre-existing",
    "message": "SCA-01 (P2): Catalog value freedom permits runtime credential references in bundles, descriptors, indexes and public results, contradicting the same contract's no-credential-address rule and safe profile/descriptor projection. Limit catalog artifacts to safe auth-profile/configuration requirements and keep actual credential locators in admitted receiver runtime configuration/private bindings."
  }
]
```
