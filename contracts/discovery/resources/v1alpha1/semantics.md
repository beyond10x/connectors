# resource_discovery/v1alpha1

- **Status:** proposed, not implemented.
- **Family:** discovery. Siblings: [mediated_route](../../mediated_route/v1alpha1/semantics.md); `endpoint_discovery/v1alpha1` and `host_discovery/v1alpha1` in [service v1alpha1](../../../service/v1alpha1/semantics.md).
- **Recorded:** 2026-09-08.

## 1. Identity

| Field | Value |
|---|---|
| Contract | `resource_discovery/v1alpha1` |
| Profiles | `grafana-datasources`, `kubernetes-service-targets` |
| Relation to `endpoint_discovery` | same observation discipline (no dial, no credential, candidates not grants); differs in that the observed resource has an opaque locator rather than an address and port, and in that a recognized observation names a target adapter that could be reached *through* the source |

Three discoveries are distinct: service, contract, resource (`docs/design.md:319-323`). This contract is resource discovery for provider objects whose address is either hidden by the provider (a Grafana data source's backend origin is never exposed) or not routable from the observer (a Kubernetes Service). An observation carries a stable source identity, an observed resource reference, a type/profile, owner scope, an address *or opaque locator*, reachability context, observation time/revision, and known authentication requirements without values (`docs/design.md:468`).

## 2. Old evidence and disposition

| Old surface | Source | Disposition |
|---|---|---|
| Grafana `[[discoveries]]`: `grafana-data-sources` from `grafana-datasources-list`; mappings `observed_type` prometheus/loki/alertmanager → `target_provider` with `route_adapter = grafana_datasource_proxy_v1` | `../connectors/providers/grafana.toml`, discoveries block | preserve as profile `grafana-datasources` with a `targets` mapping table |
| Observation carries only safe normalized metadata: identity, declaration identity, source Connection, observed type, bounded title, evidence generation and digest, plus a hidden resource binding; unknown types observable but yield no candidate | `../connectors/docs/design/08-discovery-observations-and-mediated-connections.md:105-121` | preserve |
| Evidence is generation-bound; refresh replaces the set atomically; missing resource, changed type, changed parent generation, or withdrawn mapping makes candidates stale and degrades materialized children | same, lines 116-121 | refine: atomic publication of an explicitly classified view; only complete comparable coverage proves absence; other changes invalidate eligibility without inventing disappearance |
| Kubernetes core/v1 Service recognizer by name/labels: `grafana`, `prometheus`, `loki`, `alertmanager` → target provider; type `kubernetes_service`; label namespace/name; private binding is the Service identity; SSAR before each list; at most `resource_limit` objects; no Secrets, ConfigMaps, env, EndpointSlice addresses, external URLs, scans | `../connectors/docs/design/10-local-kubernetes-context-and-resource-discovery.md:73-100` | preserve as profile `kubernetes-service-targets` |
| Argo CD recognized, stops at the observation | same document, amendment 2026-08-20 | preserve the principle: a recognizer may name a target with no route |
| Hosted Grafana: closed data-source allowlist by provider type plus UID SHA-256; missing, renamed, or type-changed sources become degraded and never widen to an untyped proxy | old design 08, amendment 2026-08-17 | refine title-only rename versus changed routing identity under §4.3; preserve the closed allowlist and no generic proxy |
| Discovery never dials, authenticates to, or materializes the observed endpoint | `contracts/service/v1alpha1/semantics.md`, Kubernetes paragraph | preserve |

## 3. Types

An admitted observation operation returns pages of one published discovery view. The receiver-selected declaration/profile fixes the coverage source; request data cannot choose another profile. The concrete declarations and recognition rules in §4.5 fix their bindings; `resources.observe` here names the conceptual read, not a reserved wire id or wildcard dispatcher. This selected payload is new, not the current Page reader:

```json
{ "limit": 100, "cursor": null }
```

```json
{
  "items": [
    {
      "id": "obs_…",
      "source_connection": "conn_source_1",
      "observed_type": "recognized_kind",
      "title": "example-resource",
      "recognition": { "provider": "recognized_kind", "confidence": "declared" },
      "locator": { "kind": "opaque", "digest": "sha256:…" },
      "candidate": { "target_adapter": "recognized_kind", "route_profile": "example-route", "confidence": "declared" },
      "auth_requirement": { "kind": "inherited_from_source" },
      "reachability": "via_source_only",
      "generation": 41,
      "observed_at_unix_ms": 0,
      "evidence": { "state": "observed", "last_seen_generation": 41, "valid_until_unix_ms": 300000 }
    }
  ],
  "next_cursor": null,
  "complete": true,
  "generation": 41,
  "scope_ref": "scope_source_1",
  "selection_revision": "selection_7",
  "coverage": { "complete": true, "partitions": [ { "id": "part_all", "kind": "example_partition", "state": "complete" } ] },
  "retention_truncated": false,
  "provenance": { "instance": "…", "resource": "source:resources", "observed_at_unix_ms": 0, "source_revision": "41" }
}
```

| Field | Rule |
|---|---|
| `locator.kind` | `opaque` (digest of the provider identity; the identity itself stays private), `address` (reserved; `endpoint_discovery` covers it today) |
| `recognition` | required-null when unrecognized, otherwise the adapter-declared provider marker and shared declared/inferred confidence; an inferred marker is not proof of an API or an available adapter |
| `candidate` | required-null unless the selected mapping supplies a potential adapter/route/auth placement; confidence agrees with recognition. Recognition can exist without a candidate, without an installed target adapter |
| `auth_requirement.kind` | `inherited_from_source` (mediated route uses the source connection's credential), `separate` (a direct connection needs its own), `unknown` |
| `reachability` | `via_source_only`, `direct_possible`, `unknown` |
| `generation` | non-reusable publication revision in one qualified coverage scope; every item on a page names that view's generation, not its last positive observation or a credential generation |
| `scope_ref`, `selection_revision` | bounded safe host-issued references to the exact admitted scope and membership definition in §4.1; opaque ids are not authority |
| `coverage` | provider collection coverage for every requested partition; complete only when every comparable partition is completely exhausted under §4.2 |
| `evidence` | publication classification observed/stale/withdrawn, last positive observation generation and original positive validity deadline; classification is not current permission or a renewed clock lease |
| `retention_truncated` | bounded historical rows were omitted under §5; omission is not confirmed withdrawal |

Coverage partitions have stable id, an adapter-owned kind/coordinate codec, and shared state complete/capped/denied/unavailable/not_scanned. The profile fixes the complete finite partition set and its exact scope interpretation. A public label or empty result cannot establish another scope. Arrays are ordered by stable id and include every requested partition once. Successful denied-subset results also declare the authorization coverage in [evidence §4.4](../../../auth/evidence/v1alpha1/semantics.md#44-exact-authorization-targets-and-fan-out-budget-f08); permission and provider exhaustion are different facts.

Evidence state observed means positively observed when this view was published, with its own timestamp/deadline; stale means retained without a comparable current positive observation, and withdrawn means confirmed absent under complete comparable coverage. The clock can make even an observed row unusable before another publication. A stale/withdrawn row may retain its former candidate as historical description, but is never materialization or dispatch authority. Safe presentation labels and references do not expose the hidden provider locator, fixed target or credential generation.

Errors: base codes plus the selected auth connection_not_ready refusal; forbidden when the profile is not enabled, not_granted for current receiver grant denial under the extended binding. These are existing codes, not new error strings.

## 4. Rules

- No effect: observing never dials the observed resource, never resolves a credential for it, never downloads or executes anything (`docs/design.md:483`).
- Source authentication is a candidate credential-placement fact, never inherited host authority. inherited_from_source may select only the explicit via_parent profile after materialization validates the supported fixed parent route and child/parent admissions; it grants no arbitrary target authentication or direct fallback. Separate/unknown requirements do not become anonymous because no credential was found.
- Candidates are not connections: a candidate says "an adapter of kind X could be bound through this source". Materialization is an explicit, host-owned configuration step (`docs/design.md:472-481`) that creates an `auth.connection` with `route.kind = via` and a `route.mediated_http` binding.
- Recognition is closed: the mapping table is part of the adapter specification; an unknown type is observable with `candidate: null`; nothing falls through to a generic proxy.
- Identity stability follows §4.3: provider object identity, compatible type and sealed semantic target must agree; a display label alone is not identity.
- Atomic refresh: the host publishes the whole classified view and matching private bindings together under §4.4. A partial view never replaces unknown history with invented absence; publication does not merge unlabelled rows from different attempts.
- Private data: provider UIDs, backend URLs, proxy paths, secure JSON, headers, and parent credentials are absent from observations, descriptors, logs, and audit (old design 08 §4 rule preserved).

### 4.1 Exact coverage scope

The logical collection key is `(owning instance, source connection, receiver-selected discovery declaration/profile and revision, admitted provider authority, normalized finite membership selection, interpretation/projection revision, host admission/disclosure scope)`. Configuration supplies these inputs; a caller-filtered result is never the global source inventory. The host retains their canonical equality form privately and exposes only safe scope_ref/selection_revision. The adapter profile defines configured source identity and any separately proven physical identity; a display/context label alone establishes neither.

Membership selection includes every configured namespace/resource selector/filter that can exclude an object. Partitions divide this exact scope; a complete namespace a proves nothing about b. A changed source/profile/provider authority, narrower namespace set, mapping/allowlist/projection or disclosure scope creates a different coverage scope/selection revision. Its complete result cannot withdraw observations from the previous one. Locally excluded candidates cease eligibility immediately under current policy, but exclusion is not provider disappearance. Unchanged partitions may be compared only when their full membership/interpretation/admission coordinates are equal; label equality is insufficient. This first profile requires equality of the whole selection revision for absence comparison rather than attempting predicate subsumption across changed filters.

Each scan also pins the current private parent credential generation, binding/configuration revision and current policy observation. These are publication/admission fences, not public scope or credential identifiers. Refreshing same-identity material does not prove inventory continuity by itself; every provider request uses the validated pinned material and current exact authorization.

### 4.2 Complete and incomplete collection

A complete partition requires definite successful authorization, all provider pages under the declared coherent-list predicate, well-formed complete responses, all selected objects considered before filtering/projection, and proved provider exhaustion without an object/byte/call/deadline cap or unresolved continuation. A short/empty page is not exhaustion unless that provider's reviewed profile explicitly defines it so. Native provider revision/continuation tokens are private provider evidence, not host publication generation. If the provider profile cannot establish its required consistency/exhaustion predicate, it cannot advertise authoritative absence for that partition.

| Scan fact | Published observation effect | Ordinary result |
|---|---|---|
| Complete comparable partition, including a complete empty partition | Fresh positive rows observed; previously known absent rows in that same partition become withdrawn | Complete collection only if every partition is complete; public paging is separate below |
| Denied subset, other targets allowed | No provider request to denied partitions; retain nonterminal prior rows as stale and already withdrawn rows as withdrawn; denial creates no withdrawal. Positives from permitted partitions follow their actual scan coverage | Explicit authorization and provider coverage, root complete:false |
| All targets denied, or unknown/stale/unavailable required authorization before reads | No resource requests; a fenced failure observation may mark affected nonterminal history stale and preserves terminal withdrawn history; failure proves no absence | forbidden for all denied; unavailable for unknown required permission, as F08 specifies. No success-shaped unchecked prefix |
| Object/byte/call/provider-deadline cap, provider failure, invalid continuation or malformed page after admitted reads | Publish validated positives already collected only where their observation evidence remains trustworthy; retain nonterminal unobserved prior rows as stale for incomplete partitions and preserve withdrawn rows. Discard untrusted page data; publication still requires the live outer deadline below | Partial classified view with coverage capped/unavailable/not_scanned and complete:false; no cursor inventing further provider work |
| Configuration/source/policy/credential fence changed or publication unavailable | Do not publish the stale attempt or use its rows as fresh authority | Applicable current refusal, or unavailable when the metadata result cannot be established |

Exactly resource_limit objects with positively proved exhaustion may be complete; reaching the number while more/unknown objects remain is capped. Counts include examined objects before candidate/visibility filtering, so filtering cannot hide unbounded scans. An actual complete partition within a partial overall scan may withdraw only its own comparable absent rows; denied/capped/failed partitions provide no negative evidence. A result's `coverage.complete` is true only if every requested partition is complete. Authorization.complete=true cannot supply that proof.

Merge precedence preserves terminal facts: a retained withdrawn incarnation remains withdrawn through every later denied/capped/unavailable/not_scanned partition. It cannot become stale or observed. A reappearing provider identity receives a new incarnation/id even in a partial scan, while the old withdrawn row may remain until bounded eviction. Only nonterminal previously observed/stale rows use stale retention. Eviction never permits reuse of the old id without verifiable incarnation continuity.

Partial publication retains the original timestamps, last_seen_generation and validity deadlines of historical rows, never the latest attempt time as fresh evidence. An unsuccessful scan may be recorded without returning a successful page. Previously retained metadata remains historical and is returned only under current explicit metadata/discovery disclosure admission; cached history cannot restore withdrawn result access. It cannot suppress known revocation, permission denial, target change or policy withdrawal. Collection failure alone does not assert that a provider object was deleted.

### 4.3 Observation identity and fixed targets

The host-issued observation id belongs to a qualified source and one observed resource incarnation: private provider identity, compatible observed type and the sealed route-relevant target evidence. Equality must cover fixed destination/resource/port and any route-owned tenant/authentication placement that changes the admitted target or authority boundary. These private comparisons are supplied by the reviewed parent profile; a UID, public digest, name or successful proxy response alone cannot prove them. Where the provider cannot furnish sufficient stable evidence, materialization/revalidation remains unavailable rather than claiming target equality. No secret value or secret-derived comparison token is exposed publicly. If eviction/restart loses verifiable incarnation continuity, a new observation id is required; a previously issued id cannot be recreated from provider UID alone.

A title-only rename preserves the id and fixed target; a new publication still needs fresh route revalidation before old pins advance. A type/semantic-target change or same native coordinate with a different provider identity creates a new observation id. The old record becomes stale/ineligible (binding changed), not “deleted” unless complete comparable coverage also establishes absence. Existing child connections never repoint to the new id. Candidate mapping/allowlist removal independently disables materialization/route eligibility without asserting disappearance. A positively withdrawn observation incarnation is terminal: later reappearance receives a new id and requires a newly admitted child connection. Re-observation after mere staleness can retain the id only on proved exact equality. Local child revocation/disablement remains authoritative in every case.

### 4.4 Publication, retention and public paging

One logical host metadata authority owns collection publication and private observation bindings. Adapter code owns provider enumeration/normalization and transient scan work; it cannot independently publish a durable generation. Each admitted attempt has a unique private id and captures the exact scope, current predecessor publication, source/configuration/credential/policy fence and both original deadlines (provider work and outer execution/publication/response). At one atomic publication point the host rechecks those facts, atomically writes the entire classified view plus private binding index, allocates a non-reusable generation and acknowledges it. Overlapping attempts may collect, but only one can publish against a predecessor; a later completion from the losing/stale attempt is discarded, never assigned a newer generation merely because it finished later. No retry of that stale content under a new predecessor is allowed.

Provider work and publication have distinct, nested original deadlines: the ordinary selected ceilings are 15 s provider work inside 20 s total execution, with connect inside provider work; a lower admitted remaining deadline wins. Authorization and collection share the provider deadline, pinned before provider work and never reset by pages or checks. At that deadline stop sends, cancel outstanding work and discard late/untrusted provider responses. Already trustworthy positives may form a partial capped view only while the original outer execution deadline still permits bounded publication and response. No extra provider work is allowed during this finalization. PublicationFacts.deadline_current means that outer deadline is still live at the atomic publication point; it never means renewing the provider budget. If the outer deadline expires before publication, refuse without publishing this attempt. If acknowledgement becomes unknown after a possible commit, use the same-attempt observation rule below; do not undo an acknowledged commit or emit success after the outer response deadline.

Known configuration/permission/revocation changes cut off conflicting admission immediately through their owning fences; a stale scan cannot undo them. Unknown publication acknowledgement grants neither a new generation nor absence: query that same metadata authority by attempt id, or refuse unavailable. Do not publish/rescan automatically to manufacture certainty. Restart must recover verifiable committed scope/generation/private-binding continuity; otherwise invalidate old handles and establish a new non-reusable scope epoch before new observation. It cannot reuse old numbers or reconstruct target authority from safe public metadata. Storage durability and exact cross-owner transactions remain required bindings, not supplied by this value model.

A fresh initial observation request performs the admitted bounded collection, then pages one acknowledged immutable view in observation-id byte order. A public cursor is not a provider continuation. It binds owning instance/source, exact scope/selection, publication generation, projection/disclosure/authorization coverage, page position and expiry; a newer publication in that same scope or changed current authorization/coverage invalidates it (stale_cursor after current access admission, with denied access refused first). Limit changes cannot retarget the scope. Subsequent pages do not rescan or reset provider budgets. Each page reports the same coverage and generation; exhaustion of public rows only makes next_cursor null. Root complete is true only on the final page of a complete collection with no retention truncation. A final page from an incomplete collection remains complete:false and next_cursor:null, which is truthful terminal partial output rather than a retry promise.

Snapshot classifications describe their publication time; original evidence deadlines and current authority are re-evaluated before materialization/route use. Neither immutable paging nor retained history preserves a permission grant. At most one current view per admitted scope is served; newer publication invalidates older cursors, so historical views need no paging lease beyond the bounded metadata-retention policy. The supporting strict reader must understand all root/row/coverage fields and F08 authorization coverage; the existing Page is unchanged.

### 4.5 Fixed declarations and adapter-native interpretation

Every authored operation id fixes one explicit contract/profile and receiver-owned
source-selection rule. An admitted invocation resolves exactly one configured
source binding. Missing, ambiguous or incompatible source selection refuses before
provider work; explicit Invocation.connection must match under the service binding.
A gateway alias resolves to the same source-qualified leaf and cannot replace its
meaning. Duplicate/conflicting declarations fail authoring/admission. These
documents do not add descriptor fields or operations to existing strict readers.

Input is closed to limit/cursor. A caller profile, source URL, scope override,
mapping table or target adapter is invalid_input. Configuration owns all native
selection/mapping, and its revision participates in scope/cursor equality.
Authorization preflight and collection coverage are independent.

The adapter owns native object validation, observed_type, closed recognition
mapping, selected partition codec, coherent-list/exhaustion proof, configured
source identity and fixed-target verification. Unknown types can remain observable
with recognition:null and candidate:null; recognition never creates a grant or an
implemented capability. A configured origin/display name cannot silently become
a globally attested physical provider identity. Each native binding must state
the continuity guarantee its evidence can actually establish.

Current native bindings are
[Kubernetes Service discovery](../../../../adapters/kubernetes/contracts/discovery/v1alpha1/semantics.md)
and [Grafana datasource discovery](../../../../adapters/grafana/contracts/discovery/v1alpha1/semantics.md).
They travel with their adapters. Shared ESS treats profile/provider/partition-kind
identifiers as opaque; the shared contract has no closed installed-provider catalog.

## 5. Limits

| Concern | Rule |
|---|---|
| Examined provider objects / retained rows | resource_limit ≤500 per scan and per published scope view; applies before projection and includes retained stale/withdrawn rows in the view |
| Page | 1–100 as `datasource.records` |
| Refresh | on demand and on a configured interval; no watch |
| Scan work | ≤64 resource-list requests plus the separately bounded F08 authorization calls; ≤4 simultaneous provider requests across both classes, shared original provider deadline and 4 MiB aggregate response bytes. Lower configured limits allowed; no automatic provider-page retry |
| Scope/cursors | ≤64 partitions, ≤256 admitted coverage scopes per composition; ≤300 s cursor lifetime and positive evidence age, bounded further by provider/configuration/credential validity |
| Retained history | At publication, keep observed rows first, then stale/withdrawn rows by descending original last-seen time and id. Retain history at most 600 s from last positive observation, without renewing it on failed scans. A view may be served only before its earliest retained row expires; thereafter retire that view and its cursors atomically, and require a fresh admitted collection before serving another view. This preserves immutable pages without extending history or silently filtering a page. Retired metadata is inaccessible and subject to the same finite cleanup bound; recovery never makes expired history addressable. Drop excess/expired history with retention_truncated:true, never as confirmed withdrawal. Missing private evidence makes dependent route admission unavailable; a child reference cannot pin unbounded history |

Observation ids, scope refs and selection/partition refs are bounded safe strings (≤256 UTF-8 bytes); native partition coordinates and target descriptions obey their smaller provider/F08 limits. Coverage plus rows must fit the selected 4 MiB result ceiling; if truthful metadata cannot fit, refuse unavailable rather than silently omitting requested partitions. Concurrent scan/provider work remains subject to the composition's aggregate resource limits. Public generation is an integer in 1..9007199254740991; before exhaustion the host creates a new non-reusable scope epoch and invalidates old handles rather than wrapping. Removed/inactive scopes share the finite scope bound and retire their history under the same retention rule; excess new scopes refuse capacity. A provider protocol requiring a larger/deeper scan cannot claim complete coverage under this first profile.

## 6. Conformance scenarios (`docs/design.md:989`)

- Fixture changes a data source type during a complete comparable scan → old `id` withdrawn, new `id` issued, generation incremented. Without proved complete comparable coverage the old row is stale/ineligible; a title-only rename keeps its id.
- Serialize all outputs and logs; grep for the fixture's UID and backend URL → no match.
- Cursor from generation 40 used at generation 41 → `StaleCursor`.
- Complete a+b, followed by denied/capped/failed b → b retained stale, never withdrawn; a complete comparable empty b later proves absence.
- Confirmed withdrawal, then denied/capped scan, then same provider identity reappears → old row remains withdrawn; reappearance has a new id/child.
- Provider deadline expires with trustworthy positives and a live outer deadline → bounded partial publication; outer deadline expires before publication → refusal, no new view.
- Complete a-only selection after narrowing from a+b → no disappearance fact about the old b scope.
- All permissions allowed but provider continuation remains → coverage incomplete; exactly-at-cap with proved end may be complete.
- Old scan finishes after a newer publisher or a source/credential/configuration change → stale publication refused, no route resurrection.
- Title-only rename → same id/fixed target; same UID with changed hidden target or port → new id, old child refuses.
- A repeated partial scan cannot grow retained history beyond the bound or refresh its original expiry; eviction is unknown, not deletion.
- No materialization occurs from observation alone: the fake host's connection store is unchanged after observe.

## 7. Compatibility
- [Service compatibility](../../../service/compatibility.md) is authoritative for the binding. Discovery observations and page generation/coverage use a selected new payload schema. Existing endpoint discovery and the closed Page reader do not gain fields automatically.


## 8. SDK and host obligations

| Obligation | Where |
|---|---|
| `ResourceObservation` type | `crates/connectors-contracts/src/lib.rs` |
| Host materialization step: candidate + operator configuration → `auth.connection` (`via`) + mediated route binding; refuses when the profile is not `via_source_only`-capable | new host module |
| Atomic scope/view/private-binding publication and recovery | logical host metadata authority; adapter supplies normalized interpretation through private ports, not a second durable owner |

## 9. ESS entities

| Entity | Notes |
|---|---|
| `ResourceObservation` persistent identity/relations | Proposed, not yet declared. Its logical source, incarnation, target interpretation and retained history belong to the host metadata boundary above; exact persistent ownership/cardinality remains UNMAPPED under the model/binding prerequisites recorded in design §31 |
| `Materialization` | not an entity here; it is the creation of a `Connection` with a parent |
| Multiplicity: one resource may have several source observations; one observation may back several child connections | Conceptual relationship only; no Connection.parent reverse relation is currently recorded in ESS. Ownership remains UNMAPPED (`docs/design.md:487`) |
| Coverage/publication/revalidation values | [discovery.yaml](../../../../ess/domains/discovery.yaml) models shared states and facts. Profile/provider/partition-kind identifiers are opaque here; typed recognition inputs, provider vocabularies and partition coordinates live in [adapter-owned models](../../../../adapters/README.md). Provider completeness, scope equality, current time/authority, atomic publication, retention and route revalidation are not executed schema predicates |

## 10. Open decisions

| Decision | Default taken |
|---|---|
| Whether `endpoint_discovery` becomes a profile of this contract | later; both stay until a third discovery profile exists |
| Refresh interval | configuration; default 300 s |

Persistence ownership is consolidated in [design §31](../../../../docs/design.md#31-host-persistence-ownership-and-atomicity). DiscoveryPublicationPort owns the complete view/private-index transaction; required current binding comparisons share the selected host metadata authority. This inventory does not supply a backend or execute its atomicity predicates.
