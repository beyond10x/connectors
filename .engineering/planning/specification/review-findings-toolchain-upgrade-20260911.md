---
format: aep.planning-md/1
id: specification:review-findings-toolchain-upgrade-20260911
kind: specification
status: draft
title: Correct review findings diagnostics and pin current ESS and AEP tools
relations:
- specifies: initiative:complete-local-connectors
revision: 1
---
## Outcome

Remove the 139 misleading or historical review-findings warnings without changing
original review bodies or verdicts, and select verified current ESS and AEP tools
through exact repository pins rather than ambient executables.

## Authority and baseline

The operator requested this work on 2026-09-11 after the HTTP/catalog ADR handoff.
Baseline is Connectors 857569291b1a0601b27ad2bb8b050cdefa022f2f. AEP 0.55.0 reports
139 missing-block warnings: 88 already contain an explicit empty findings block;
51 use historical prose or JSON formats. AEP's validator currently tests whether
parsed findings are empty rather than whether a block exists. Original immutable
reviews must remain byte-identical. No review outcome or provider completion is
inferred from correcting this representation.

Verified upstream tags on 2026-09-11: ESS 0.22.2 at
6b666e58f2e87dd8798d27f935e9a012203296a3 and AEP 0.55.0 at
28abe09bb6e5b0a6b4db839f6bf5693957d39324. AEP remote main is
4eb999e0ae3cc77d1c387152e23a85ad4eae86dc at discovery. Connectors currently pins
ESS 0.20.0 at 6f7ef46163e758f3401945d1a946e0fc80ebc003; AEP execution is ambient,
separate from the existing pinned protocol documents. Record any local AEP fix as
an exact source revision, not as an upstream release that does not exist.

## Work and verification

Correct AEP's empty-versus-absent diagnostic, retain absent/malformed refusals,
and support source-bound structured supplements for historical immutable reviews
if the current CLI cannot represent the correction. Supplements describe the
original review and are not new critic rounds or fabricated approval/outcome
evidence. Preserve source bytes, source references, historical verdicts and each
finding's actual recorded scope. Unknown classifications remain explicit.

Upgrade ESS through the existing source receipt resolver, regenerate its owned
outputs and verify shared/native validation, drift, conformance, repository/MSRV
gate and affected website references/examples. Pin AEP's executable identity and
use it in the repository gate; distinguish that binary pin from the protocol-doc
pin. Do not silently change protocol lifecycles or use an ambient older binary.

Run targeted AEP regression and required upstream checks for the validator change.
Retain before/after validation, exact binaries/source/lock identities, immutable
review hashes and actual commands/results. Missing prerequisites remain named.

## Coordination

This is an interactive, single-writer task in managed worktrees. The existing
GitLab session remains the sole planning writer on integration main and was
notified of this tooling/review scope. No global binary replacement occurs during
its active gate. Integration will use the agreed checkpoint and AEP commands;
there is no manual merge of concurrent planning journals. No new runtime entity,
provider implementation or story decomposition is selected here.
