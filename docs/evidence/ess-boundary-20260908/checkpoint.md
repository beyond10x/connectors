# Shared ESS provider boundary, 2026-09-08

Owner: `story:shared-ess-provider-boundary`. Baseline HEAD is
`dd08cfd60aa43740da21946666dc0fa89c1160dc`, with the in-progress datasource drafts
already present. This checkpoint covers the operator-requested ownership audit
and regression gate; it does not close the datasource stories or their findings.

## Result

Shared ESS retains provider-independent facts, decisions, lifecycles and opaque
adapter references. Provider representations, scope kinds, discovery profiles,
recognizer catalogs/inputs, partition coordinates, permission target tuples,
container lifecycle intent, conditional Deployment preparation and LogQL range
selection have typed homes in five independent [adapter models](../../../contracts/adapters/README.md).
The shared root imports none of them. Native target equality, provider predicates
and public codecs remain explicit adapter obligations; private references do not
replace public target tuples or prove current admission.

The existing Rust repository gate and its focused `ess-boundary` command share
one implementation. It scans paths, comments and decoded YAML with the reviewed
provider/native-term policy and automatically discovered adapter names. It rejects
adapter paths in shared sources, source symlinks, foreign domains and incomplete
or unloaded YAML inventories. The pinned ESS compiler checks all six model roots
independently, rejecting unresolved cross-root dependencies. No Python helper was
added or executed. Protocol vocabulary such as HTTP/OAuth2/SIP remains permitted.
Unfamiliar terminology still requires semantic review; lexical checks are not a
proof of provider independence or runtime behavior.

## Verification

`TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run -p connectors-build --locked --offline -- gate --msrv`
exited **0**; AEP on PATH was the retained exact-source 0.54.0 binary. The
[full transcript](full-gate.log) records formatting, descriptor drift, workspace
build, **55 passing Rust tests**, Clippy with warnings denied, library/CLI
dependency boundaries, Rust **1.88.0**, independent ESS validation/compilation,
and planning validation. Five Rust tests exercise the new boundary, with multiple
negative and positive cases in each relevant matrix. There was one full gate run,
and it passed. Later edits only updated ownership prose, review/evidence records
and the planning story.

ESS 0.20.0 compiled shared Connectors **14 files / 218 declarations** plus
Atlassian **2 / 3**, Docker **2 / 3**, Grafana **2 / 4**, Kubernetes **4 / 11** and
Loki **2 / 4**. The existing conformance synthesis compiled **222 scenarios,
34 authored, zero refusals**. These are schema/reference and scenario-compilation
checks, not provider runtime conformance results.

The independent gate reviewer reproduced the original casing bypass, then verified
**11 rejected negative cases** against the corrected command and a passing shared
protocol/unrelated-word fixture. See [review checks](review-checks/) and the
[final gate review](gate-final.md). A separate foreign-type negative control was
refused with `undeclared_reference`, exit 1 ([diagnostic](negative-dependency.log)).
The shell link audit checked **182 local targets**, with none missing. `git diff
--check` passed. Full-gate planning validation was 112 artifacts, valid, with 65
existing review-format warnings. Final post-recording validation is **116 artifacts,
valid, 67 warnings**, retained in [planning validation](planning-validation.log).
The two added warnings misclassify the reviewers' explicit empty findings blocks;
this known AEP defect is already filed as `story:empty-findings-are-reviewed` in
the sibling AEP backlog. Both approval blocks remain present and unchanged.

## Review dispositions

Both independent reviewers approved the corrected boundary scope:
[semantic approval](semantics-final.md) and [gate approval](gate-final.md).

### ESB-S-01

Fixed: moved LogDirection, LogScopeEquality and LogRangeSelection into the Loki
model, documented native input ownership and added the two identified native
structure names to the boundary policy. Generic collection/page facts stay shared.
Source: [initial semantic review](semantics-initial.md).

### ESB-S-02

Fixed: Grafana owns its exact three-provider recognition enum and its projection
obligation; it neither imports Kubernetes recognition nor expands shared enums.
Source: [initial semantic review](semantics-initial.md).

### ESS-GATE-01

Fixed: the matcher compares concatenated complete-word sequences, recognizing
GitLab/LogQL/acronym variants while preserving unrelated words such as cargo.
Unit and independent command-level negative cases cover the correction.
Source: [initial gate review](gate-initial.md).

## Retained evidence and record fidelity

The [reviewed source archive](reviewed-sources.tar.gz), SHA-256
`3d3bb8c062bc90e511515e9b9b43fb11c0b55f88b6860dc0a8ffd823351e9440`,
contains the final boundary source/models and relevant ownership documents.
[Source hashes](source-hashes.txt) identify its individual inputs. Historical
review evidence elsewhere in the repository was preserved unchanged.

The first initial-review imports were refused because root requested unsupported
`id`/`artifact` findings keys. The gate report also initially used non-native
`major`/`new` severity/origin spellings. Reviewers reissued format-compatible
reports without changing their prose or findings: [semantics](semantics-initial-aep.md)
and [gate](gate-initial-aep-v2.md). These exact reviewer-authored reissues were
imported through AEP; the original reports and intermediate gate reissue remain
retained. No planning file was edited directly and no immutable review was rewritten.
