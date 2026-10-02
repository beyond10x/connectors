# Official Kubernetes upstream inspection

Bounded source inspection, 2026-10-02. Only official upstream bytes were fetched into `.local/next-milestones/k8s-upstream/`; no generation/import command, build, provider call, AEP/model/source edit or implementation acceptance occurred. `gh api` was read-only after the session's established Connectors GitHub capability gap; downloads used credential-free exact-commit raw URLs.

## Verified source pin

`gh api repos/kubernetes/kubernetes/git/ref/tags/v1.31.5` resolves `refs/tags/v1.31.5` to annotated tag object `9108404f76a428f9c14da276f075bdf8bafdf707`. Reading that tag object resolves to commit **`af64d838aacd9173317b39cf273741816bd82377`**, tag `v1.31.5`, release-robot tagger timestamp `2025-01-15T14:32:36Z`.

This is the requested Kubernetes version corresponding to the owned k3s 1.31.5 baseline. It is proof of the upstream Kubernetes tag target, not independent proof of the K3s binary/image's provenance or a cryptographic tag-signature verification. The repository fixture also records an observation on v1.31.5 at `adapters/kubernetes/tests/local_runtime.rs:66`.

Exact URL prefix for every downloaded file:
`https://raw.githubusercontent.com/kubernetes/kubernetes/af64d838aacd9173317b39cf273741816bd82377/`

| Upstream path | Bytes | SHA-256 |
|---|---:|---|
| `api/openapi-spec/swagger.json` | 3,277,085 | `ddcb3d5c3d85849f1fadb56387d3a7bf44712da291e5bd08b34df4e8a4247f8d` |
| `api/openapi-spec/v3/api__v1_openapi.json` | 1,845,061 | `9774af2f5f5cdfbeb2557e5e1ec491341025f993639fbef912e2c7cbd97b21a2` |
| `api/openapi-spec/v3/apis__apps__v1_openapi.json` | 833,849 | `91c2ff75cfcb3f4f743bf08bda2d1b9c893a4987c792200df0364d6d52a9cdc9` |
| `api/openapi-spec/v3/apis__discovery.k8s.io__v1_openapi.json` | 150,293 | `b21cb43462abd4cb660b71bd14694d09e3d6890a1b4ad8b257bbac41ee1d341d` |
| `LICENSE` | 11,358 | `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30` |

The exact root LICENSE is Apache License, Version 2.0, January 2004. Retain it with the source provenance. `swagger.json` declares Swagger **2.0** and is not an admissible source for the current v2 importer. The three official group documents exist in the same pinned tree, each declares **OpenAPI 3.0.0** (`info.version` is `unversioned`, so the immutable URL/commit/hash must carry version provenance). All original documents are below the importer's 16,777,216-byte per-source bound; no extraction/conversion is necessary merely to satisfy version or size.

## Exact selected GET operations

| Group | Full path | `operationId` | Local consumer(s) |
|---|---|---|---|
| core | `/api/v1/namespaces/{namespace}/pods` | `listCoreV1NamespacedPod` | `resources.list`, kind `pods` |
| core | `/api/v1/namespaces/{namespace}/services` | `listCoreV1NamespacedService` | `resources.list`, kind `services` |
| core | `/api/v1/nodes` | `listCoreV1Node` | optional `hosts.discover` |
| apps | `/apis/apps/v1/namespaces/{namespace}/deployments` | `listAppsV1NamespacedDeployment` | `resources.list`, kind `deployments` |
| discovery | `/apis/discovery.k8s.io/v1/namespaces/{namespace}/endpointslices` | `listDiscoveryV1NamespacedEndpointSlice` | `resources.list`, kind `endpointslices`; `endpoints.discover` |

The last row is one upstream GET with two local consumers. Six fixed internal mappings therefore require only five provider operation IDs. Existing v2 uniqueness is on local operation IDs; import records a coverage entry per local mapping while retaining one identical selected upstream path/method (`crates/connectors-spec/src/v2.rs:176-240`, `:491-492`). Do not collapse the two local cursor/result obligations.

## Measured parameter forms and smallest concrete importer mismatch

For all five selected operations, GET has no `requestBody`, parameters are inline (zero parameter `$ref`s), and no `style`, `explode`, `allowReserved` or `allowEmptyValue` override is present. The four namespaced paths have one required path parameter, `namespace`; nodes has no required parameter. All query parameters are optional.

Mapped parameter schemas are identical across the selected operations where present:

| Parameter | Location | Exact schema |
|---|---|---|
| `namespace` | path | `{"type":"string","uniqueItems":true}` |
| `limit` | query | `{"type":"integer","uniqueItems":true}` |
| `continue` | query | `{"type":"string","uniqueItems":true}` |

**Concrete static refusal:** current `import` rejects every mapped schema key outside its allowlist, which omits `uniqueItems` (`crates/connectors-spec/src/v2.rs:345-374`). Thus untouched existing-v2 generation is not a credible completion claim for these exact bytes. No importer was executed: this conclusion follows from the measured schemas and exact rejecting branch, not a recorded red run.

The smallest proposed importer extension is an explicitly reviewed scalar-schema rule for the pinned documents' boolean `uniqueItems` annotation, preserving the original source bytes/coverage and validating that the selected type is scalar. Do not silently strip source bytes, admit arbitrary unknown keywords, or claim array uniqueness support. Array-specific keyword applicability needs to be justified in the Connectors import contract before implementation and proved with refusal/compatibility tests. This is narrower than a new selector/mixed-coverage format, but it is still a compiler semantic change.

The earlier possible `format:int64` blocker is **not present on these selected mapped parameters**: their schema has only `type` and `uniqueItems`. No mapped scalar min/max/pattern/enum/format constraints appear. Integer formats and complex structures elsewhere in response schemas do not pass through this mapped-parameter allowlist and must not be mislabeled as the same blocker.

Optional, deliberately unmapped query parameters are `pretty`, `allowWatchBookmarks`, `fieldSelector`, `labelSelector`, `resourceVersion`, `resourceVersionMatch`, `sendInitialEvents`, `timeoutSeconds`, and `watch`. Their booleans, camel-case names and `uniqueItems` are not reasons to expand the selected mapping: current import records excluded optional parameters without applying mapped-input lowering to them (`crates/connectors-spec/src/v2.rs:459-486`). Preserve omission and exclusion coverage; no watch/filter capability is selected.

## Whole-source versus selected-source limits

The importer validates original-source size/hash and OpenAPI version **before** selection. It then retains the selected GET operations, their path-level parameters, security declarations and transitive response-schema references (`crates/connectors-spec/src/v2.rs:274-290`, `:487-539`). It does not import every provider operation merely because the original document contains it.

Whole-document inspection found core/apps/discovery contain respectively **239 / 157 / 24 component schemas**, no non-`#/components/schemas/` `$ref`s and no unresolved `$ref` targets. These whole-document totals give an upper bound below the importer's 4096 distinct retained-reference limit for any selected closure; they are not claimed selected-closure counts. References remain opaque response facts; ESS's subsequent OpenAPI import may still refuse unsupported response constructs, and its output must remain honest retained evidence rather than a successful-import claim. The actual generation/lowering/build pipeline remains unexecuted.

## Continuation semantics and next bounded step

The official selected `continue` description restricts values to tokens from preceding compatible queries and says a consistent-list restart omits the field. The `limit` description establishes that a page may contain fewer items, even zero, while continuation still indicates more; servers can also ignore the requested limit. Neither description directly establishes that explicitly sending an empty continuation equals omitting the request parameter. Keep existing authored signed-cursor admission and truthful completeness/oversize behavior.

Therefore retain the prior options report's explicit omission decision: current renderer always emits mapped query entries; require an authored, narrowly specified absent-continuation omission adapter or a bounded optional-query generation extension. Do not treat the response's empty continuation statement as proof about request serialization.

The concrete next design scope is now **three official pinned v3 sources + three private v2 bundles, scalar `uniqueItems` import handling, explicit optional-continuation serialization, and aggregate public-coverage/packaging checks**. A new finite-selector profile is not required by these measured source facts. Exact source pinning uncertainty is resolved; importer acceptance, deterministic output, runtime equivalence, image acceptance and any chosen semantic change still require their own reviewed implementation and executed evidence.
