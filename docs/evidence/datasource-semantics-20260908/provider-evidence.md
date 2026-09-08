# Datasource provider evidence — 2026-09-08

This record supports F12 log continuation and F14 document admission. It distinguishes provider declarations, predecessor behavior and receiver-selected semantics. No live tenant, daemon or cluster conformance was exercised. The initial independent reviews remain needs-revision; obtaining these sources alone closes no finding.

## Provenance

[provider-source-hashes.json](provider-source-hashes.json) records original URLs, byte lengths, SHA-256 and compressed copies. Moving vendor documents are captured by exact bytes and acquisition URL; their query-string version or snapshot version is not a promise that the URL is immutable. Repository source URLs use the selected release commit. Predecessor files come from Connectors commit 81459ac42ddd518d3942f4b079841e9e0ed6efc8.

Connectors 0.7.0 offered no admitted operation for the requested official-documentation search. That capability gap was reported before using web search and direct HTTPS retrieval of official sources. Provider business operations and external accounts were not invoked.

## Loki

- [Official v3.7.0 HTTP source](https://github.com/grafana/loki/blob/3361de24b692875d77bd7433cd6baa7c68dc0ef9/docs/sources/reference/loki-http-api.md), range-query section: the same endpoint supports streams and metrics; the line limit applies to streams. Start is inclusive, end exclusive, direction controls timestamp order and interval sampling can omit entries. The contract selects JSON log results and sends no interval/metric step.
- [Pinned LogCLI query implementation](https://github.com/grafana/loki/blob/3361de24b692875d77bd7433cd6baa7c68dc0ef9/pkg/logcli/query/query.go), lines 177–200: batching refuses when the timestamp-overlap entries fill a batch. Inclusive replay of the last timestamp, and the backward one-nanosecond adjustment, do not establish stable identity for arbitrary repeated identical occurrences.
- [Pinned LogQL AST](https://github.com/grafana/loki/blob/3361de24b692875d77bd7433cd6baa7c68dc0ef9/pkg/logql/syntax/ast.go) and [parser](https://github.com/grafana/loki/blob/3361de24b692875d77bd7433cd6baa7c68dc0ef9/pkg/logql/syntax/parser.go): distinguish concrete stream-selector/pipeline roots from sample expressions. Some literal/vector types implement both interfaces, so a LogSelectorExpr interface check alone is not the selected log-only predicate.

Inference/selection: retained local ordinals can page one bounded captured observation while preserving multiplicity. They cannot recover entries excluded by the provider's saturated cap. Effective provider/proxy limits, incomplete-response signaling, matching parser behavior and current authority require binding evidence before advertisement. Neither source proves global snapshot isolation, unlimited history or immediate remote cancellation.

## Pod and container logs

- [Kubernetes v1.35.0 PodLogOptions](https://github.com/kubernetes/kubernetes/blob/66452049f3d692768c39c797b21b793dce80314e/staging/src/k8s.io/api/core/v1/types.go), lines 7169–7223: relative sinceSeconds, optional sinceTime, tail count and timestamp output. There is no upper-time input. LimitBytes may return slightly more/fewer bytes and split the last line. With TailLines, split-stream selection is restricted to combined output. The contract selects finite combined reads and a receiver byte cutoff without pretending that limitBytes or receiver time establishes an exact provider interval.
- [Docker Engine API v1.56](https://docs.docker.com/reference/api/engine/version/v1.56.yaml), ContainerLogs and ContainerAttach: finite stdout/stderr selection, since/until, tail, timestamps, supported logging drivers and raw/multiplexed transport. Logs does not set Content-Type, so the selected decoder uses inspected TTY mode. The exact archived source and hash are retained in the [restart/visibility provider manifest](../restart-visibility-20260908/provider-source-hashes.json).

Receiver-selected call/byte/time caps are normative design choices, not provider defaults or benchmark results. The selected Docker name/label observation does not make inspect-plus-read atomic against concurrent changes. Provider-native pod relative time and Docker integer-second selection do not become Loki nanosecond continuation.

## Confluence

The captured [official v2 OpenAPI](https://dac-static.atlassian.com/cloud/confluence/openapi-v2.v3.json?_v=1.8516.103), OpenAPI 3.0.3 / info 2.0.0, declares GET /pages with page-ID filters (maximum 250), space-ID filters (maximum 100), status, subtype, body-format and pagination. The result is PageBulk, with id, spaceId, status, version and BodyBulk representations. The default statuses include current and archived; a current-page profile must select current explicitly. The actual HTTP base is /wiki/api/v2.

GET /pages/{id} has no space-ID filter. Selecting the collection endpoint with conjunctive ID and space filters is a concrete route to provider-scoped body retrieval for an unseen ID. The declaration promises permission-filtered pages; it does not prove an atomic transaction between independent metadata/body requests or continued membership until the caller receives a response.

The captured [official v1 OpenAPI](https://dac-static.atlassian.com/cloud/confluence/swagger.v3.json?_v=1.8516.103) retains /wiki/rest/api/content/search. It does not contain the old content/{id} GET selected by the predecessor catalog. Absence from this document is not proof that a deployed endpoint has been removed; the new profile must make its chosen v2 binding explicit. The old content endpoint also permitted content kinds other than pages.

## Jira

The captured [official Jira platform OpenAPI](https://dac-static.atlassian.com/cloud/jira/platform/swagger.v3.json?_v=1.8516.103), OpenAPI 3.0.1 / info 1001.0.0-SNAPSHOT-c008a50be871e07da906dfe1b414dddd7ac4b1e0, contains the selected /rest/api/2 paths. The v3 filename does not imply REST API v3 behavior.

GET /rest/api/2/issue/{issueIdOrKey} documents case-insensitive and moved-key fallback, returning the resolved issue without a redirect. Input key prefixes and redirect refusal cannot prove current project membership. Selecting fields=project supplies a candidate bounded metadata lookup; raw issue fields/body are not part of that lookup's public disclosure.

GET /rest/api/2/search/jql accepts bounded JQL, fields selection and reconcileIssues (at most 50 exact issue IDs). The default fields are IDs only; a full native field result must explicitly request fields=*all. The [official reconciliation description](https://developer.atlassian.com/cloud/jira/platform/search-and-reconcile/) distinguishes ordinary eventual search consistency from reconciliation of named IDs; reconciled results must still satisfy JQL.

Inference/selection: exact canonical ID plus receiver-owned project predicates and reconciliation can support a bounded scoped detail read. This is not a transaction spanning alias lookup, body read and future project moves. A provider's updated field is not automatically a global issue-content version, and no such version is invented for caching.

## Independent intake

Exact reports are [review A](reviews/a-initial-report.md) and [review B](reviews/b-initial-report.md). Their separate subsequent provider analyses are [A's supplement](reviews/a-provider-evidence-supplement.md) and [B's supplement](reviews/b-provider-evidence-supplement.md). Those supplements add no verdict or finding. [archive-manifest.json](reviews/archive-manifest.json) preserves all frozen review inputs, including their original relative layout, with exact hashes. Reports remain unchanged and AEP retains their verbatim bodies.
