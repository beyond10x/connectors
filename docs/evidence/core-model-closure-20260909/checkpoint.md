# Reviewed v0.1.0 specification baseline

The selected Kubernetes (including discovery), GitLab and SQL specifications
have completed semantic remediation and finite ESS model closure. Both independent
reviewers approve exact source `7e8c718303a43bc7ec4a02d6a76581d92802bf6b`, and
the full repository gate on that source passed. The release commit adds only
planning and retained evidence to that reviewed source; its local annotated
`v0.1.0` tag identifies the resulting clean commit.

This is a specification milestone. The earlier first-slice runtime is preserved;
this model stage changed no runtime, strict reader, adapter-native source, Cargo
manifest or lockfile. There are zero tracked Python files. The next authorized
implementation may use these typed homes without another open-ended specification
phase. Optional capabilities and actual storage/crypto/clock/codec/conformance
work remain explicitly distinguished in [design §31.2](../../design.md#312-revisions-retention-and-modeled-scope)
and [CHANGELOG.md](../../../CHANGELOG.md).

## Independent reviews and findings

| Review | Exact source | Verdict | Immutable report SHA-256 |
|---|---|---|---|
| A initial auth/artifact | `b5abe434fbb34dfd889303f19933a9bff9e2e23a` | needs-revision, MCA-01 | `fda2e612565addf565d5103403156c52fee2d075a6e308584d74db2b3fabeb41` |
| A complete stage | `7e8c718303a43bc7ec4a02d6a76581d92802bf6b` | approve, MCA-01 resolved | `d0ea490bfe98506f5ed98c2abceb6cce9df90f895dbb3ca1f6d282a19a267356` |
| B complete stage | same final source | approve, no findings | `809634a9ecaaa282d072d9980f313b240cb21f96bb198f728f129f65b01b29b0` |

[Initial A](reviews/reviewer-a-initial.md), [final A](reviews/reviewer-a-final.md)
and [final B](reviews/reviewer-b-final.md) are unchanged reviewer output. AEP
records each verdict and its fixed/no-op disposition. B did not read A's findings.
The [MCA-01 correction](../model-auth-closure-20260909/correction-mca01/addendum.md)
adds one bounded current-generation Connection baseline snapshot. The audit unit's
earlier private-identity correction is retained in its
[verification](../model-audit-closure-20260909/verification.md).

The [preceding checkpoint](../spec-completion-20260909/checkpoint.md) retains
the independent approvals and closure of all 48 original findings across 27
remediation stories. This final review covers the subsequent model delta; it
does not rewrite or claim to re-execute those historical reviews.

## Executed validation

`CARGO_BUILD_JOBS=2 cargo run -p connectors-build --locked --offline -- gate --msrv`
exited 0 on the reviewed source with the pinned ESS 0.20.0 binary and recorded
AEP build on PATH. The [verbatim gate log](full-gate-7e8c718.log) and
[structured summary](gate-summary-7e8c718.json) record:

- 58 Rust tests passed across 38 runners, zero failures; MSRV 1.88.0 check passed.
- Shared ESS: 18 files, 314 declarations, 20 entities. All eleven original
  identities/lifecycles/field types and all original named types are preserved.
- Shared ownership boundary plus five authored native roots validate and compile
  independently. Existing adapter and generated-artifact checks pass.
- 315 structural scenarios synthesized, including 34 authored, with zero refusals.
  These are not executed persistent/provider conformance scenarios.

Two actual canonical compiles produced identical IR, SHA-256
`b1ecc905c7595f64c3ad4fe57d256c19423a30cf6933926e6edbfa7248b56bcc`.
Two schema generations each produced 336 files; `diff -qr` found no differences.
The [generation log](schema-generation-7e8c718.log) records the output set.
All 26 newly added normative/design/changelog links resolve, including five
anchors; the [link audit](new-link-audit-7e8c718.json) retains exact scope. Unchanged
links retain the preceding checkpoint's 702-link audit.

Planning validation is retained verbatim before and after final review intake.
Its known warning about historical prose and empty `findings: []` reviews is an
upstream enumeration limitation, not a missing semantic finding or a failed gate.
The immutable reviewer outputs are preserved instead of inventing findings to
silence that warning.

## Source and recovery

[Reviewed source archive](reviewed-spec-sources-7e8c718.tar.gz), SHA-256
`bbaf54bf8263f79f85e11819cdb4e6864fa31115d1e8e6a1639c4860e909f995`,
contains contracts, ESS, adapters and their evidence, crates, strict spec kinds,
Cargo files, CHANGELOG, README, design and compositions at exact reviewed source.
It excludes this central evidence directory and planning history to avoid recursive
archives. Gzip-compressed briefs/handoffs/patches preserve original bytes and original
checkout context. [SHA256SUMS](SHA256SUMS) covers the retained packet.

Unit commits `95b1ebdd9b10355a944d5a681a83a9b3dc073e8a` (auth),
`84be6ef55fbcc37e5838660ec02336b8c45cc9b8` (discovery) and
`28c4e666cf651bb40831918d6bdbfe1e022260c5` (audit) are ancestors of the
reviewed integration commit, published only to the local-recovery bare repository.
Their completed trees were removed by exact reviewed GC ids; the
[dry run](cleanup/worker-cleanup-dry-run.json) and
[apply result](cleanup/worker-cleanup-apply.json) record recovery proof.
Raw scratch is retained outside the trees under
`/home/timo/.local/state/worktree/recovery/connectors_v2-model-closure-20260909/`.

Both completed reviewer trees were also removed after their reports and raw scratch
were preserved, as recorded by the [reviewer dry run](cleanup/reviewer-cleanup-dry-run.json)
and [apply result](cleanup/reviewer-cleanup-apply.json). Final integration cleanup
follows final local recovery publication and clean-main/tag checks. Older retained design
and Kubernetes-drive trees are outside this task's cleanup. No external remote,
deployment, Atlas registration or consumer publication is part of this milestone.
