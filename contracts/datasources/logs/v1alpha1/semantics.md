# datasource.logs/v1alpha1

**Status:** proposed; public log codecs and provider behavior are not implemented.

## Shared contract and adapter ownership

A log read returns a bounded observation of available source logs, with explicit
selection, order, truncation and provenance. It proves neither archival retention,
absence of late arrivals, a global snapshot nor continuity across rotation/restart.

The adapter owns the versioned profile identifier, typed native input/selection,
source protocol, stream metadata, ordering and source-exhaustion proof. Profile
identifiers are open. The shared family does not impose a query language, absolute
time window, provider catalog or universal continuation model.

Current proposed bindings are [Loki](../../../../adapters/loki/contracts/logs/v1alpha1/semantics.md),
[Kubernetes](../../../../adapters/kubernetes/contracts/logs/v1alpha1/semantics.md)
and [Docker](../../../../adapters/docker/contracts/logs/v1alpha1/semantics.md).
These index links create no dependency on a concrete adapter. Each adapter carries
its own profile schema and conformance obligations when extracted.

## Result obligations

The result contains `lines`, `selection`, `order`, `complete`, `truncation`,
`next_cursor` and `provenance`. The selected profile supplies a closed typed
selection and stream schema; these are not arbitrary provider-object bags.

Each line has `timestamp_unix_ns`, `stream`, `line`, `line_truncated`,
`redacted` and `source`. A timestamp is a canonical decimal string at actual
source precision; null is permitted only when explicitly selected by a profile
that cannot establish a representable timestamp. Do not fabricate or round one.
The profile defines stream/source identity and ordering. Returned metadata is a
value, not evidence of scope or authority. Equal timestamp/text/labels do not
prove identical occurrences and cannot alone justify deduplication.

Redaction is an explicitly admitted optional host filter, independently flagged;
it does not promise secret-free content. A profile defines its deterministic
redaction and UTF-8-safe clipping order. Content clipping does not itself remove
an occurrence. All byte bounds distinguish decoded content from serialized
escaping/framing and encoded/decoded provider bytes.

Provenance identifies the admitted instance/source/resource and original
observation time, with a nullable source revision. Reusing a retained observation
preserves its provenance rather than presenting cache access time as freshness.

## Completeness and loss accounting

`complete = source_exhausted AND no_occurrences_omitted AND final_retained_page`.
An unpaged result is its final page. Source exhaustion must be established by
the adapter's supported native protocol and effective provider/proxy limits.
A malformed response, unknown partial extension, cap or interrupted EOF cannot
silently become complete empty logs. A terminal partial result may have no cursor.
Line clipping is reported separately and may coexist with complete:true.

Truncation causes are distinct and ordered:
provider_limit, provider_partial, stream_limit, page_limit, response_bytes,
source_bytes, line_bytes. The profile selects applicable causes and precisely
defines their trigger. Provider_limit represents source saturation; provider_partial
requires a supported indication; stream_limit represents omitted groups;
page_limit/response_bytes describe an undelivered retained remainder; source_bytes
describes an explicitly valid bounded streaming cutoff; line_bytes describes
clipping on this page. Record every applicable cause.

Occurrences_dropped and stream_groups_dropped are exact nonnegative totals only
when fully observed evidence establishes each total, otherwise null for the
affected total. A known local omission count is not a complete total when provider
loss remains unknown. Retained remainders are not dropped occurrences. Clipping
alone drops zero occurrences.

## Admission and continuation

Current host principal, visible operation, source/connection, scope and applicable
credential/route evidence are admitted before dispatch. Known revocation and
current binding fences govern publication and disclosure. These host checks do
not lock future provider permissions. No stale-on-error, cross-principal cache
reuse, live follow or automatic redispatch is selected by this shared contract.

Continuation requires explicit adapter support. A token grants no authority;
current admission precedes disclosure of cursor validity or retained content.
Its private context binds instance, operation/profile/contract/projection,
source/connection/authority, credential and parent/route fences, principal/tenant,
scope/policy/configuration, exact selection, limits and redaction. A token exposes
no credentials, origin or private route locator.

The profile must distinguish immutable retained paging from repeated provider
queries, define monotonic progress and repeated-cursor behavior, and bound expiry
and capacity without renewing observation age. Expired/lost/evicted/mismatched
state is StaleCursor after admission; unavailable authority/cache infrastructure
is Unavailable. No hidden requery or direct-route fallback may replace retained
state. Timestamp/text hashes cannot invent missing native occurrence identity.

## Resource limits, errors and verification

Each profile declares finite input, metadata, encoded/decoded source, retained
state, serialized-result, provider-call and deadline ceilings. Permission,
metadata and mediated work consume the same original budget. Retention admission
accounts backing allocations, context and indexes atomically across concurrent
requests and declares bounded per-instance and per-principal/tenant capacities.

The selected service binding supplies safe errors and its maximum result/deadline.
Deadline exhaustion is Timeout; it is neither clean EOF nor cutoff success.
Malformed, oversized and interrupted observations are Unavailable unless an
explicit profile rule establishes a valid streaming cutoff. No raw provider
errors, query text, origins or credentials enter ordinary diagnostics.

[Service compatibility](../../../service/compatibility.md) governs the wire binding:
an ordinary result slot does not confer log semantics on existing typed readers.
[Shared ESS](../../../../ess/domains/datasource_reads.yaml) models generic
collection/page facts and decisions; native selectors belong to adapter ESS.
Parser, ordering, full context equality, byte accounting and current publication
remain unimplemented binding obligations. Native profiles own their adversarial
fixtures; schema acceptance alone does not execute those obligations.
