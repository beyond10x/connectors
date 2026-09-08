# datasource.series/v1alpha1

**Status:** proposed; no public series codec or runtime behavior is implemented.

## Shared obligations

A series profile returns bounded labeled samples with explicit temporal selection,
resolution, ordering, completeness and provenance. It preserves native query and
sample meaning. Series are not record or log pages, and a transport-compatible
result slot does not establish series compatibility. See
[service compatibility](../../../service/compatibility.md).

The adapter owns its versioned profile identifier, request/result schema, native
query language, time precision, sample value encoding, supported result kinds,
provider errors and concrete limits. The shared contract has no closed provider or
profile catalog. Each advertised schema is closed; profile-specific structures
remain typed in the owning adapter rather than becoming an arbitrary payload bag.

The [Prometheus series binding](../../../../adapters/prometheus/contracts/series/v1alpha1/semantics.md)
owns the currently proposed range-query profile and all native projection rules.
Its instant and label names are reserved/refused in that adapter and are not shared
profile vocabulary or advertised support. This index link does not require that
adapter to exist for another implementation to conform to the shared contract.

## Admission, completeness and limits

Current principal, visible operation, connection/source, scope and applicable auth
and route bindings are admitted before dispatch and disclosure. Neither query text
nor returned labels grant authority. An adapter must establish its native query
containment before advertising scoped support; syntax acceptance alone is not proof.
Provider addresses and credentials remain receiver-owned.

`complete` is false whenever a supported provider partial indication or local
omission affects the selected result. A binding must define its proof of exhaustion
and distinguish malformed/unsupported source results from valid partial results.
It preserves sample precision and special values under its declared codec, without
silent aggregation, unit conversion or downsampling. It states the ordering of
series and samples and every truncation rule.

Every binding declares finite query/window/resolution, series/sample, metadata,
encoded/decoded source, public-result, provider-call and deadline limits. Cached
results retain original provenance and require current complete-context admission;
caching is permitted only under explicit profile rules. This shared contract does
not grant a cache, a retry, a provider-specific error mapping or an unbounded query.

Native schemas and independent fixtures must cover precision, partial results,
malformed input, scope refusal, byte/deadline exhaustion and current authority. No
ESS shape or generated schema is evidence of executing those provider obligations.
