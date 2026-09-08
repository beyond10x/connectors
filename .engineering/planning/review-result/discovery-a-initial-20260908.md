---
format: aep.planning-md/1
id: review-result:discovery-a-initial-20260908
kind: review-result
status: active
title: Discovery coverage and composition reviewer A initial
relations:
- reviews: story:contracts-discovery-coverage
- reviews: story:contracts-host-composition
revision: 1
---
# Discovery coverage and host composition — reviewer A initial

**Verdict: needs revision. Findings: P0 0 / P1 0 / P2 8 / P3 0.** This independent read-only review covers the existing discovery-coverage (F13) and host-composition (E03) stories. It does not close discovery-profiles (E07/E14/E32) or select an implementation.

Baseline: `0e4a8c11885e7008b2825cdcd0d2855cd241d9b4`. All 38 inspected/reference inputs were frozen with `git show 0e4a8c1:<path>` under `sources/`, with exact SHA256 in `source-hashes.json`. Later working-tree edits are excluded. No other reviewer output was consulted, no tracked/planning file was modified, and no runtime test was run. Provider-specific upstream facts were not re-researched; the findings concern the supplied contracts and their ownership boundaries.

## DC-A-01 — P2 — The scope that can authorize withdrawal has no exact identity

**Sources:** discovery/resources:15,41–56,66–68,81–82; auth/evidence:124–132; design:468–487.

The new guard correctly requires a complete authoritative observation over the represented scope, but the only generation key specified is source_connection. Namespaces, selected resource classes, source/profile/configuration revision, provider authority, caller disclosure and the checking credential can vary. A complete scan of namespace a after configuration narrows from a+b could therefore be read as authority to withdraw b. A result filtered for one caller must not rewrite another caller's authoritative source inventory either.

**Required correction:** define a canonical coverage domain using the already admitted source/binding identity, receiver-selected discovery declaration/profile revision and exact finite selection. Distinguish provider scan coverage from result disclosure and current host policy. State which scope changes create a new domain or invalidate old observations; a removed namespace, withdrawn mapping/allowlist or denied caller is not proof that the provider object disappeared. No absence outside the complete comparable domain may cause confirmed withdrawal. This uses the existing source identity as an input; it does not settle E32's future Kubernetes identity representation or E07's operation/profile naming decision.

## DC-A-02 — P2 — Permission coverage, provider exhaustion and public page completion are conflated

**Sources:** discovery/resources:31–57,68,79,88–96; auth/evidence:128–134; Kubernetes adapter:69,90,97; design:963.

The discovery payload has one complete boolean, while generation replacement needs proof that all intended provider objects were observed. Authorization coverage proves only allowed/denied exact targets. It does not establish provider-list exhaustion, absence of unvisited pages, a consistent provider snapshot, or that a cap did not discard objects. A public page can have a continuation even after the host collected a complete source snapshot. Conversely, a final successful checked page can still omit denied targets or hit resource_limit.

**Required correction:** specify these three dimensions separately. Define complete scan criteria, partial causes and the resulting response/publication disposition for denied targets, object cap, response-byte cap, timeout, provider failure, unknown status and invalid provider continuation. Unknown permission retains F08's unavailable refusal before business work; do not turn it into a successful unchecked/denied prefix. If partial scan facts are retained or disclosed, they must be explicitly distinguishable from an authoritative complete generation. Hitting the numeric cap alone is not proof of truncation or exhaustion: state how exact-at-cap with proven exhaustion differs from unknown/more remaining. A complete empty comparable scan can prove absence; all-denied and failed scans cannot.

## DC-A-03 — P2 — Atomic replacement lacks an acceptance/publication fence for overlapping scans

**Sources:** discovery/resources:23,66–79,88–90; mediated_route:40,60,63–65,84; auth/connection:84–87,94–106; design:959–961.

Readers must never see mixed generations, but nothing identifies a refresh attempt or prevents an earlier scan from completing after a newer scan and replacing it. Partial/failed attempts have no specified relationship to the last complete publication. Configuration, parent credential publication, policy withdrawal or restart can also occur while provider pages are being accumulated. Merely allocating the next monotonic number at completion can make stale contents look newest.

**Required correction:** choose a bounded serialized refresh or equivalent stale-publisher rejection per exact coverage domain. Capture the source/configuration/authority and provider snapshot coordinates the attempt relies on; recheck applicability before one acknowledged atomic publication of the complete set and private bindings. A canceled, failed, stale or superseded attempt cannot replace a newer accepted publication, infer withdrawal or revive a target. An ambiguous publication acknowledgement is not proof of either a new or absent generation; observe the owning authority without blindly rescanning/replaying effects. State restart treatment so lost transient state cannot reset generation numbers into reusable cursor/route authority. This is a required semantic fence, not a request to implement a store or distributed transaction.

## DC-A-04 — P2 — Retained stale observations and route revalidation do not have a closed trace

**Sources:** discovery/resources:68,78–79,95–98; mediated_route:40,55,60,63–65,83–85; auth/profile:108–110; auth/connection:84–85,94–112; Grafana adapter:117.

The proposed route is fixed to observation id + generation, yet the parent re-resolves the UID for the current generation. The same word generation also appears for parent credential material. It is unclear whether a routine same-target observation refresh permanently breaks the child, silently advances its admission, or permits explicit same-binding revalidation. Retaining the previous complete set after an incomplete scan must not turn that history into present permission or suppress a known contrary target observation.

**Required correction:** distinguish public observation publication generation, private credential generation and route/configuration revision. Define freshness/expiry and retained historical status, and the admitted path that revalidates an unchanged fixed target against a current accepted observation without changing the connection ref. Old admissions and verification do not automatically advance. Exact current route permission, source revocation, target disappearance/change and mapping withdrawal remain authoritative despite retained history; local revoked/disabled precedence stays intact. Distinguish list-services denial from the separate get-services/proxy permission rather than copying a namespace denial into an unrelated authorization fact. New route kind, parent, fixed target, access mode or owner still requires a new connection under the settled AP rule.

## DC-A-05 — P2 — Stable provider locator equality alone can hide a changed mediated target

**Sources:** discovery/resources:26,45,78,95; mediated_route:24–25,60–65; Grafana adapter:20,114–117; Kubernetes adapter:98; design:485.

Observation identity is stable while the private Grafana UID or Kubernetes Service UID is unchanged. The forwarding check mentions missing/type-changed sources, but the stable provider object may retain its UID/type while its hidden destination, fixed port or admitted routing/tenant configuration changes. Re-resolving that locator without defining the sealed target comparison can repoint an existing child. Rename behavior is also stated more broadly in the Grafana document than the resource identity rule.

**Required correction:** define the private semantic target/binding evidence that must remain equal across re-observation and route validation, independent of public id or display title. Keep sensitive coordinates private; safe opaque references/digests are not authority by themselves. Specify whether a label-only rename preserves identity while invalidating/revalidating presentation evidence, and make the adapter examples agree. A changed fixed target or incompatible type cannot be accepted by simply updating the observation generation; it degrades/refuses the old route and requires the separately admitted new binding/connection. Concrete provider extraction fields can remain an explicit profile-authoring gate, without claiming E14 recognition is solved.

## DC-A-06 — P2 — The concrete composition wiring owner is still assigned to generic server.rs

**Sources:** mediated_route:13–14,43–46,99–102; Grafana adapter:9–11,115,132; design:153–164,229–253,832–836; current host server.rs:14,31–47; SDK lib.rs:17–31,54–57.

The host obligation table assigns composition loading to connectors-host/src/server.rs, while the architectural invariant forbids host-to-concrete-adapter dependencies. The existing server accepts an already constructed Arc<dyn Adapter>; it neither selects provider implementations nor supplies a mediated implementation. Co-location itself is compatible with the design, but the caller constructing both concrete libraries and registering private route ports has no named owner.

**Required correction:** name an explicitly authored application/composition executable as the wiring root. It may depend on the selected parent and child adapter libraries plus generic host/SDK infrastructure, construct them, and inject/register their narrow ports. Parent and child libraries depend on SDK/contracts, never each other; the generic host imports no concrete adapters or provider registry/enum. Provider-specific target interpretation stays with the parent implementation; generic admission, binding, infrastructure and lifecycle remain host ports; product selection belongs to the composition. Add a compact dependency/port diagram and align the obligation table. This needs neither new loader implementation nor a new ESS adapter kind.

## DC-A-07 — P2 — Same composition is too broad for the private one-process route boundary

**Sources:** mediated_route:43,67,92,101; Grafana adapter:11,115,130–132; design:793–816,834; CLI migration:55.

The route says same host process or composition, but the design also uses composition for collections of separately launched or remote services. Sharing a deployment, machine, namespace or configuration file does not make a private Rust port callable across processes. A broad reading would require an unspecified remote provider-traffic proxy, contrary to the selected boundary.

**Required correction:** explicitly restrict this mediated-route profile to parent and child instances wired into the same process/trusted composition runtime with live injected ports. Distinguish that runtime from deployment compositions of service endpoints. A remote operation gateway can forward an ordinary child operation to the owning composition; it cannot export/call the private route port or make an unreachable direct target reachable. Refuse missing registrations, incompatible target/profile, separate-process placement and nested parents before advertising a callable child or dispatching provider traffic. Each adapter retains independently usable direct service/library packaging; no composition is mandatory for direct use. Local supervision/readiness must report missing bindings without dynamic loading, direct fallback or fabricated support.

## DC-A-08 — P2 — Proposed observation/route lifecycles and ownership are described as recorded without an actual model

**Sources:** discovery/resources:109–119; mediated_route:100,108–109; auth/connection:161–165; ess/domains/declarations.yaml:40–108; auth_access.yaml:87–100; design:487,959–961.

ResourceObservation and RouteBinding tables state entity identities, lifecycles and relations; the resource document even says multiplicity is recorded on Connection.parent reverse. Those declarations are not present in ESS, and Connection persistence is explicitly still unmapped. The two-state observed/withdrawn lifecycle is also unable to describe partial attempts, stale retained observations or valid same-identity re-observation without further decisions. Adapter-transient bindings backed by host metadata blur which owner acknowledges durable publication and recovery.

**Required correction:** make these tables truthful: distinguish settled decision/value shapes from proposed persistent entities/relations, and name the logical host publication/binding owner versus adapter-private interpretation/transient scan work. If the settled coverage/publication/revalidation decisions are typed now, use the applicable ESS value workflow and explicit UNMAPPED live predicates and ownership rather than inventing a persisted lifecycle or a fake owns edge. Do not claim the existing ESS graph already records a Connection.parent cardinality. Retention, recovery and stable generation authority need explicit semantic requirements even if their storage realization remains separately owned.

## Minimum verification packet

- Complete a+b snapshot, followed separately by capped scan, denied b, provider failure, narrowed a-only scope and a caller-filtered projection: no false withdrawal of b.
- Permission coverage all allowed but provider continuation remains; complete scan served over several public pages; exactly-at-cap with proven end versus cap with unknown/more; complete empty scope versus all denied.
- Two overlapping attempts finishing out of order; configuration/credential/policy change before publication; failed or ambiguous publication; restart loses transient bindings.
- Retained historical observation plus exact proxy denial; parent revocation; list-only denial distinguished from route permission; fresh unchanged target revalidation; same UID/type with changed fixed target; label-only rename; confirmed disappearance and later re-observation.
- Stale public cursor versus current disclosure; no private UID, URL, credentials, route coordinates or credential-generation IDs in payload metadata.
- One explicitly wired executable with parent and child ports; parent and child each built/used without the sibling; separate-process deployment refuses this private route profile; missing capability, remote metadata-only discovery and nested parent fail without fallback.

The final packet should distinguish declared textual traces and schema shape checks from actual provider, clock, store, route and process behavior. E07/E14/E32 remain draft and dependent; exact operation naming, full Kubernetes recognizer rules and unsupported cluster identity must not be marked closed by this two-story review.
