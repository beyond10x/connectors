unit: story:contracts-log-continuation then story:contracts-document-admission — F12/F14 native specification completion
verdict: green (native handoff; shared patch, integration gate and independent final review remain coordinator work)
cases: 83 manual cases (41 logs, 42 documents); executed ESS commands 4 before + 4 after, all exit 0; no runtime cases
origin: n/a
wrote-outside-worktree: none (required lease metadata is managed by worktree hooks)
needs-coordinator: yes — /home/timo/.local/state/worktree/trees/b10x/connectors_v2/specs-datasources-20260909/.local/spec-completion-20260909/shared-datasource.patch

## Unit and acceptance

F12 now has proposed dispositions for LD-A-01–03 and LD-B-01–04, occurrence-preserving
bounded progress, truthful saturation/local omission and all current-context
refusals. F14 has proposed dispositions for LD-A-04–08 and LD-B-05–08, admitted
scoped retrieval, moving/cached object rules, faithful bounded native bodies and
bounded body-bearing search. The principal remedies already existed at base
8e1836cad8ae1b2127ce9ae306c6d8131960db4c; this pass completes their evidence and
remaining native consistency decisions. No finding is self-approved or closed in
planning by this handoff.

## Diff

Observed `git diff --stat` (untracked evidence is listed separately):

```text
 .../atlassian/contracts/documents/v1alpha1/cql.md  |  14 ++-
 .../contracts/documents/v1alpha1/semantics.md      | 121 +++++++++++++++++++--
 adapters/atlassian/design.md                       |   9 +-
 .../docker/contracts/logs/v1alpha1/semantics.md    |  58 +++++++++-
 .../contracts/logs/v1alpha1/semantics.md           |  18 ++-
 adapters/loki/contracts/logs/v1alpha1/semantics.md |  32 +++++-
 6 files changed, 232 insertions(+), 20 deletions(-)
```

New files: the exact 22-path list is `new-files.txt` beside this report. They live
only below the assigned Loki/Kubernetes/Docker logs and Atlassian documents native
`evidence/20260909/` directories: 83 manual scenarios, 16 finding dispositions,
two model-check records, three provider-source manifests/notes, nine source
archives, and one derived PageBulk schema-facts extract. Original review reports,
root evidence snapshots and provider cache files were not changed.

The changes are uncommitted on specs/datasources-20260909. No git add/commit/stash,
branch change, worktree provisioning/cleanup, planning write, runtime implementation
or executable helper file was used.

## Before-correction cases

`f12-before.md` records the three initial manual red cases: simultaneous Loki
count/byte triggers, native unpaged tail/cutoff causes and missing source archives.
`f14-before.md` records the initial six document red cases and an additional
zero-candidate bulk-request ambiguity identified before its correction. The
coordinator's Docker tail-unit counterexample was supplied before its correction
and is now D10. These are textual failures under the explicitly adapted charter;
there is no fabricated failing runtime suite.

Notable corrections:

- Loki evaluates page_limit and response_bytes independently; native/source causes
  persist per observation and emitted-line causes are per page.
- Docker requires verified native tail/exhaustion mapping; LF/frame counts alone
  are insufficient. Valid multi-line messages are not protocol errors. Local
  output keeps the first decoded occurrences in completion order and reports
  page_limit; native saturation/source cutoff make full dropped totals unknown.
  Without the selected mapping the binding refuses Unavailable and cannot
  advertise this profile. The current vendor archive does not prove that mapping.
- Confluence accepts absent/null bulk subtype only under the mandatory page
  predicate, refuses explicit conflicts, and requires usable exact identity,
  current status, scope and storage. An optional returned representation marker
  must agree with the fixed storage format.
- CQL lineages/branches are immutable and share a bounded chain pool. A failed
  page cannot consume its existing cursor or publish a provisional child.
- Empty CQL candidate pages make no bulk request; nonempty bulk requests use the
  exact candidate count and documented comma-separated ID arrays.
- Entire encoded/decoded source envelopes, including unused metadata/expansions,
  count toward source ceilings. Body/result truncation causes have independent
  predicates, and original byte counts require complete valid acquisition.
- E26 configuration outline values are labelled illustrative versus selected
  document/CQL defaults and maxima. Native CQL quoting and relocated references
  are corrected.

## Executed checks

All model commands used:
`/home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess`.
Its version output was `ess 0.20.0` (exit 0).

Before and after the respective native pass:

```console
ess specify validate --path adapters/loki/spec/ess
connectors_loki v1 — 2 file(s), valid
ess specify validate --path adapters/atlassian/spec/ess
connectors_atlassian v1 — 2 file(s), valid
```

Each of the four validation invocations exited 0. Exact compile commands, each
exit 0, were:

```console
ess specify compile --path adapters/loki/spec/ess --format json --out .local/spec-completion-20260909/loki-before-ir.json
ess specify compile --path adapters/loki/spec/ess --format json --out .local/spec-completion-20260909/loki-after-ir.json
ess specify compile --path adapters/atlassian/spec/ess --format json --out .local/spec-completion-20260909/atlassian-before-ir.json
ess specify compile --path adapters/atlassian/spec/ess --format json --out .local/spec-completion-20260909/atlassian-after-ir.json
```

The emitted JSON is retained in those exact scratch files (later compile stdout
is also retained beside them). `cmp` before/after for each root exited 0. Loki IR
SHA-256 is 70baafbf6eb852679d7a3a82f9310d17d7563002093e92af1bc594aef8cbda02;
Atlassian IR is 230cf351782cde13f59fe09d451d84b382180861ab7438c509b161c02a5ee3a9.
Each has 1 domain, 3 types, 0 entities and 0 commands. Unchanged IR is expected:
this prose/evidence correction adds no new model declaration. UNMAPPED predicates
remain explicit; compiler success does not prove provider, parser, reducer,
authority, decoder, cache or cursor behavior.

Final standalone checks:

```console
git diff --check
git apply --check .local/spec-completion-20260909/shared-datasource.patch
cmp .local/spec-completion-20260909/loki-before-ir.json .local/spec-completion-20260909/loki-after-ir.json
cmp .local/spec-completion-20260909/atlassian-before-ir.json .local/spec-completion-20260909/atlassian-after-ir.json
```

All four exited 0 with no output. The shared patch remains unapplied in this tree.
Its SHA-256 is 94577488b0f9c46261495ba11137246b0ab2402e5f5401e4f616fbb0292a4b9c.

Provider checks: original Loki/Kubernetes manifest hashes matched (three OK);
six CQL hashes were recorded from the retained original cache and matched on
checking (six OK). All nine `gzip -dc <archive> | cmp - <source>` comparisons
exited 0. The source cache remained read-only. The existing Confluence v2 source
hash is 451377c5a598ee8155acc11b611404f309bed4a4292ea87f88ed3bfed38fa0a8, matching
the derived-facts source record. Literal byte-length checks returned 43 and 8
(exit 0). These check provenance/length only, not runtime behavior.

## Coordinator actions and boundaries

Apply shared-datasource.patch: generic page_limit may describe retained remainder
or an explicitly selected native terminal omission. Loki retained continuation
does not change. The other hunks only repair obsolete shared ESS owner-section
comments. Shared files and all planning remain untouched here.

Integrate the native files plus new evidence, run the shared/full integration gate
once, freeze the resulting inputs, and dispatch independent final review before
closing F12/F14. Runtime/provider conformance, exact upstream source/license
adoption, public codecs and standalone packaging remain later prerequisites.

No cold Rust build, cargo fmt --all, runtime suite, Python, new helper script,
provider call, remote publication or worktree cleanup was performed. The tree and
scratch stay available for coordinator integration. Lease session
specs-datasources-implementor-20260909 will be released immediately before the
final handoff response; that response reports the hook result.

## Outside-worktree writes

None. Source cache/toolchain access was read-only. Only the required worktree
session-start/heartbeat/session-end hooks manage their own external lease state.
