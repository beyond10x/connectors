# Artifact provenance model evidence

Unit scope: shared immutable Source/Bundle declarations and embedded Curation,
with explicit reference reuse. Normative owner is
[the catalog artifact model](../../../contracts/catalog/v1alpha1/semantics.md#9-ess-declarations-and-optional-catalog-state).
The current strict adapter/generation readers and all existing identities remain
unchanged. Catalog publication/service implementation is deferred.

## Executed model checks

At base 8a5cf563fc717fd4b23e4dd7d470d0c967f39461, pinned ESS 0.20.0
`specify validate --path ess` passed with 14 files and
`specify compile --path ess --out .local/model-closure-20260909/baseline-ir.json`
passed with 218 declarations. After the coordinator artifact additions, the same
commands passed with 15 files and 229 declarations. These are unit checks before
the parallel auth/audit/discovery roots are integrated.

[Compiled-IR checks](artifact-model-checks.json) compare the actual baseline and
new IR: all eleven existing identities, lifecycles, field types and named types
are unchanged; original adapter-operation ownership is preserved. The two added
entities are Source and Bundle. Source joins are references, Bundle has no
index-publication state, and no CatalogIndex entity is declared.

## Manual semantic cases

After the unit checkpoint `efb8c2fbb934e5918c2023396cd67e60220fb79f`, the
repository boundary gate refused the optional adapter name in the new shared
model's summary/comment and normative path. The model now uses generic
distribution wording and cites design §31.2, which links the owning artifact
rules. No policy exception was added. Running the existing CLI's `ess-boundary`
command with pinned ESS then exited 0 for the shared root and all five native
authored roots. The retained unit hash manifest describes the initial unit
checkpoint; final integrated evidence will pin the later comment/design changes.

These are textual scenario analyses, not executed runtime or model reducers.

| Case | Required selected result |
|---|---|
| Two adapters reference the same pinned Source | Both references are valid; neither owns deletion of the Source |
| Two Bundle digests realize revisions of one logical adapter | Each keeps its own specification digest; logical adapter identity does not establish exact revision equality |
| One Bundle digest appears in two index generations | One immutable Generated declaration; no competing indexed/superseded lifecycle |
| Replace one selecting index | Retire that selection; no Bundle/Source deletion cascade |
| Same source URL/revision arrives with changed bytes or provenance | Conflicting immutable record refuses replacement; require an explicitly reviewed source revision |
| Same retained original bytes undergo a named transformation | Preserve original digest and ordered input/output digests separately |
| Transform output is claimed as original vendor bytes | Refuse the false provenance; matching output hash does not authenticate the original |
| Source license evidence absent | No license permission or proof of license absence is inferred |
| Required curation absent | Operation remains unresolved for that profile; optional model presence does not manufacture default approval |
| Curation sets realization generic but obligations remain unsatisfied | Do not advertise callable support; declaration alone is not a completed binding |
| Source/Curation fields sent to unchanged strict reader | Reader retains its current accepted syntax; internal ESS is not a wire/format expansion |
| Adapter configuration refers directly to its service without any catalog | Auth/invocation semantics are unchanged; artifact references confer no execution authority |

Digest/key uniqueness, file/path integrity, source/license truth, transform
lineage, required curation/discriminator consistency and retained-consumer deletion
checks remain explicit implementation predicates. The model's successful compile
does not execute any row above. No generator, storage backend, catalog service,
runtime helper or provider integration was added for these declarations.
