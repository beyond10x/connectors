# Grafana datasource discovery/v1alpha1

**Status:** proposed profile `grafana-datasources`, not implemented. Implements
[resource_discovery/v1alpha1](../../../../../contracts/discovery/resources/v1alpha1/semantics.md).
The exact current source is the configured admitted Grafana connection/authority;
backend origins and caller data cannot select another source.

The receiver declaration `datasources.observe` fixes this contract/profile and
source selection. Input is closed to limit/cursor. Missing, ambiguous or incompatible
source refuses before work; explicit Invocation.connection must match. Request
profile, origin, mapping, target-adapter or filter overrides are invalid_input.
Configuration/mapping/projection changes require new scope/selection identity;
old cursors refuse rather than being reinterpreted.

Enumerate data-source metadata, never contact its backend. Every valid configured
source object remains observable, including unknown plugin types. Recognition maps
reviewed native type values prometheus/loki/alertmanager to the corresponding
provider marker with confidence:declared. Unknown plugins have recognition:null
and candidate:null. A closed configured allowlist of type plus UID SHA-256 narrows
candidacy; removal of candidacy does not prove provider disappearance. Candidate
mappings require installed compatible route/auth bindings and current admission;
there is no generic proxy fallback.

The selected partition codec is `{id,kind:collection,namespace:null,state}`, one
collection partition. Shared state, scope, ordering, coverage/publication, history
and finite scan limits apply. A successful short response proves exhaustion only
under the reviewed provider-list predicate, not merely by page length.

The provider UID is sealed into a private opaque locator/digest. Backend origins,
UIDs, secure config, credentials, headers and raw dashboard/query objects must not
enter public observations or diagnostics. A title-only rename preserves identity.
Changed UID, type or semantic target needs a new observation; old children never
repoint automatically. Absence requires complete comparable collection.

A stable UID/type cannot prove an unchanged hidden destination, tenant or auth
placement. The exact parent verifier must establish equality of all route-relevant
target facts under the admitted profile; missing/changed/unknown equality refuses
materialization/revalidation. This verifier and coherent-list proof remain native
authoring/implementation prerequisites. Shared history cannot supply fresh authority.

Typed profile, recognition vocabulary and partition kind live in
[authored ESS](../../../spec/ess/domains/discovery.yaml).
The [route binding](../../routes/v1alpha1/semantics.md) owns forwarding.
No child adapter implementation is imported merely to recognize its provider type.

## Evidence and fixtures

Original evidence is `../connectors/providers/grafana.toml` discoveries/mappings
and `../connectors/docs/design/08-discovery-observations-and-mediated-connections.md`.
The old route adapter name `grafana_datasource_proxy_v1` maps to the proposed
`grafana-datasource-proxy` profile.

Required native cases: three known types plus an unknown plugin yield four
observations and at most three candidates; one list sends no backend request;
allowlist/type/UID/hidden-target changes cannot widen access; title-only rename
preserves identity; incomplete coverage cannot infer disappearance. Output and
diagnostic fixtures must exclude UIDs/backend URLs. These are specification cases,
not execution results.
