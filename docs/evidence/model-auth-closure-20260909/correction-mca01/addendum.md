# MCA-01 correction addendum

Reviewer A finding MCA-01 (P2) identified that the auth model omitted the existing
EvidenceSnapshot carrier promised by the Connection/evidence contracts. Correction
base: `6cee258b34ae0c9e0b0a5de5a5ed7fd04e336e4e`. Initial unit verification is retained
unchanged; this addendum records only the correction.

Connection now has
`baseline_evidence: Optional<connectors.credential_evidence.EvidenceSnapshot>`.
The existing EvidenceSnapshot already holds a list of individual EvidenceCheck
values. The material baseline has five material check kinds and at most one
explicit universal verify_operation, so a singular aggregate is sufficient.
A list of snapshots would introduce unneeded generation/history multiplicity.

The owning [Connection §4.2](../../../../contracts/auth/connection/v1alpha1/semantics.md#42-persistent-binding-identity-and-retention)
selects at most one row per applicable baseline kind, at most six rows; baseline
scope_check uses the profile minimum, and operation-specific scopes/permissions/
verification cannot overwrite it. Existing [evidence §§3–4.4](../../../../contracts/auth/evidence/v1alpha1/semantics.md#3-types)
separate baseline and exact-operation reductions and retain operation evidence in
transient DispatchAdmission/selected bounded permission-cache semantics. No new
persisted evidence entity, history, or side store is introduced.

The carrier must match the active generation and exact Connection binding/current
profile context. It is absent before initial publication, without active material,
and for anonymous/parent-authenticated children. Candidate validation stays in the
existing transient acquisition/validation protocol. Publication/recollection replaces
one value under the current fence; source generation/time can transfer only under
the existing explicit lineage rules. Known cutoffs win over cached positives.

Retention and freshness are distinct: at most one bounded current-generation
snapshot may retain historical stale/negative checks; they grant no use. Replace
or clear it with removal of current material, and clear on local terminal revoke.
Per-check cached-use ceilings remain 300 seconds for reads, 60 for mutation use,
or smaller provider/profile bounds. Installation must fit the host's declared
finite metadata byte limit, with refusal rather than truncation of required checks.
This correction does not add a new archive or change fixed Connection identity.

## Focused cases inspected

These are manual contract/model checks, not runtime execution:

| Case | Required result |
|---|---|
| First acquisition before publication | No Connection baseline carrier; candidate evidence remains private/transient, and absence proves no successful check. |
| Current g1 with a g0 snapshot or different instance/profile/authority | Refuse installation/use; same public Connection ref cannot make stale material evidence current. |
| Missing optional write grant or denied operation permission | Exact operation refuses under its existing rules; that failure cannot replace the minimum-scope baseline or poison unrelated reads. |
| Duplicate baseline check kind or permission_check in baseline | Refuse baseline installation. No list of per-operation snapshots grows on Connection. |
| Same-identity refresh | Construct a new matching-generation value; only explicitly transferable checks retain original source generation/deadline. Permission/verification cannot acquire fresh authority by copying. |
| Expired check, revoked profile/policy, consumed refresh or local revoke | Required reuse refuses; retention never refreshes a deadline or erases a known cutoff. Local terminal revoke clears the cached baseline while remaining terminal. |
| Anonymous or parent-authenticated child | No material generation/account or EvidenceSnapshot is fabricated; existing configuration/route verification remains separate. |
| Snapshot count/check/byte bound exceeded | Refuse installation; retain at most one bounded baseline, without silently dropping required checks or inventing a side store. |

## Executed verification

Pinned ESS 0.20.0; TMPDIR and scratch stay inside the auth tree. The validation input
is a copy of the real auth unit ESS root with the already supplied exact shared
registration/reference patches applied. No guessed entity or discovery stub exists.
The cross-worker mediated-route field remains for the coordinator's integrated root.

- `ess specify validate --path <scratch>/mca01/work/ess`: exit 0, 15 files valid;
  [log](validate.log).
- `ess specify compile --path <scratch>/mca01/work/ess --out <scratch>/mca01/model.json`:
  exit 0, 252 declarations; [log](compile.log).
- Compile the same real root with the auth file from 6cee258: exit 0, 252 declarations;
  [before log](before-compile.log). Canonical IR comparison shows **the only change is
  the one Connection field**, with no existing identity, lifecycle, other field,
  type or command changes; [comparison](model-diff.json).
- `ess generate --path <scratch>/mca01/work/ess --kind schema --out <scratch>/mca01/schema`:
  exit 0, 266 artifacts; [log](schema.log). The new field references the existing
  EvidenceSnapshot definition and is optional by omission in this private projection;
  [inspection](carrier-schema-inspection.json). It adds no public reader field support.
- `git diff --check`: exit 0.

Count/subset uniqueness, exact generation/binding/current-profile equality, metadata
bytes, per-check freshness/invalidation, atomic installation and known-cutoff
preservation are explicit owner predicates. Optional and the unchanged list-bearing
EvidenceSnapshot schema do not execute them; shape-valid contradictions remain
possible. No runtime test, build, publication or planning mutation was performed.
Root integrates this correction and obtains the independent reviewer recheck.
