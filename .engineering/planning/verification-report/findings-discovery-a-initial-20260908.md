---
format: aep.planning-md/1
id: verification-report:findings-discovery-a-initial-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:discovery-a-initial-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: a4ff99dac9c0a13def3b66b70ccd60dd79eafc2ae5abd11dba4f763d0ce33848
relations:
- verifies: review-result:discovery-a-initial-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:discovery-a-initial-20260908

This supplements [the immutable original](../review-result/discovery-a-initial-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

The original report's verdict and scope remain authoritative; no new verdict is assigned.

## Transcription method

8 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": ".engineering/planning/review-result/discovery-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-01 — P2 — The scope that can authorize withdrawal has no exact identity\n\n**Sources:** discovery/resources:15,41–56,66–68,81–82; auth/evidence:124–132; design:468–487.\n\nThe new guard correctly requires a complete authoritative observation over the represented scope, but the only generation key specified is source_connection. Namespaces, selected resource classes, source/profile/configuration revision, provider authority, caller disclosure and the checking credential can vary. A complete scan of namespace a after configuration narrows from a+b could therefore be read as authority to withdraw b. A result filtered for one caller must not rewrite another caller's authoritative source inventory either.\n\n**Required correction:** define a canonical coverage domain using the already admitted source/binding identity, receiver-selected discovery declaration/profile revision and exact finite selection. Distinguish provider scan coverage from result disclosure and current host policy. State which scope changes create a new domain or invalidate old observations; a removed namespace, withdrawn mapping/allowlist or denied caller is not proof that the provider object disappeared. No absence outside the complete comparable domain may cause confirmed withdrawal. This uses the existing source identity as an input; it does not settle E32's future Kubernetes identity representation or E07's operation/profile naming decision."
  },
  {
    "file": ".engineering/planning/review-result/discovery-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-02 — P2 — Permission coverage, provider exhaustion and public page completion are conflated\n\n**Sources:** discovery/resources:31–57,68,79,88–96; auth/evidence:128–134; Kubernetes adapter:69,90,97; design:963.\n\nThe discovery payload has one complete boolean, while generation replacement needs proof that all intended provider objects were observed. Authorization coverage proves only allowed/denied exact targets. It does not establish provider-list exhaustion, absence of unvisited pages, a consistent provider snapshot, or that a cap did not discard objects. A public page can have a continuation even after the host collected a complete source snapshot. Conversely, a final successful checked page can still omit denied targets or hit resource_limit.\n\n**Required correction:** specify these three dimensions separately. Define complete scan criteria, partial causes and the resulting response/publication disposition for denied targets, object cap, response-byte cap, timeout, provider failure, unknown status and invalid provider continuation. Unknown permission retains F08's unavailable refusal before business work; do not turn it into a successful unchecked/denied prefix. If partial scan facts are retained or disclosed, they must be explicitly distinguishable from an authoritative complete generation. Hitting the numeric cap alone is not proof of truncation or exhaustion: state how exact-at-cap with proven exhaustion differs from unknown/more remaining. A complete empty comparable scan can prove absence; all-denied and failed scans cannot."
  },
  {
    "file": ".engineering/planning/review-result/discovery-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-03 — P2 — Atomic replacement lacks an acceptance/publication fence for overlapping scans\n\n**Sources:** discovery/resources:23,66–79,88–90; mediated_route:40,60,63–65,84; auth/connection:84–87,94–106; design:959–961.\n\nReaders must never see mixed generations, but nothing identifies a refresh attempt or prevents an earlier scan from completing after a newer scan and replacing it. Partial/failed attempts have no specified relationship to the last complete publication. Configuration, parent credential publication, policy withdrawal or restart can also occur while provider pages are being accumulated. Merely allocating the next monotonic number at completion can make stale contents look newest.\n\n**Required correction:** choose a bounded serialized refresh or equivalent stale-publisher rejection per exact coverage domain. Capture the source/configuration/authority and provider snapshot coordinates the attempt relies on; recheck applicability before one acknowledged atomic publication of the complete set and private bindings. A canceled, failed, stale or superseded attempt cannot replace a newer accepted publication, infer withdrawal or revive a target. An ambiguous publication acknowledgement is not proof of either a new or absent generation; observe the owning authority without blindly rescanning/replaying effects. State restart treatment so lost transient state cannot reset generation numbers into reusable cursor/route authority. This is a required semantic fence, not a request to implement a store or distributed transaction."
  },
  {
    "file": ".engineering/planning/review-result/discovery-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-04 — P2 — Retained stale observations and route revalidation do not have a closed trace\n\n**Sources:** discovery/resources:68,78–79,95–98; mediated_route:40,55,60,63–65,83–85; auth/profile:108–110; auth/connection:84–85,94–112; Grafana adapter:117.\n\nThe proposed route is fixed to observation id + generation, yet the parent re-resolves the UID for the current generation. The same word generation also appears for parent credential material. It is unclear whether a routine same-target observation refresh permanently breaks the child, silently advances its admission, or permits explicit same-binding revalidation. Retaining the previous complete set after an incomplete scan must not turn that history into present permission or suppress a known contrary target observation.\n\n**Required correction:** distinguish public observation publication generation, private credential generation and route/configuration revision. Define freshness/expiry and retained historical status, and the admitted path that revalidates an unchanged fixed target against a current accepted observation without changing the connection ref. Old admissions and verification do not automatically advance. Exact current route permission, source revocation, target disappearance/change and mapping withdrawal remain authoritative despite retained history; local revoked/disabled precedence stays intact. Distinguish list-services denial from the separate get-services/proxy permission rather than copying a namespace denial into an unrelated authorization fact. New route kind, parent, fixed target, access mode or owner still requires a new connection under the settled AP rule."
  },
  {
    "file": ".engineering/planning/review-result/discovery-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-05 — P2 — Stable provider locator equality alone can hide a changed mediated target\n\n**Sources:** discovery/resources:26,45,78,95; mediated_route:24–25,60–65; Grafana adapter:20,114–117; Kubernetes adapter:98; design:485.\n\nObservation identity is stable while the private Grafana UID or Kubernetes Service UID is unchanged. The forwarding check mentions missing/type-changed sources, but the stable provider object may retain its UID/type while its hidden destination, fixed port or admitted routing/tenant configuration changes. Re-resolving that locator without defining the sealed target comparison can repoint an existing child. Rename behavior is also stated more broadly in the Grafana document than the resource identity rule.\n\n**Required correction:** define the private semantic target/binding evidence that must remain equal across re-observation and route validation, independent of public id or display title. Keep sensitive coordinates private; safe opaque references/digests are not authority by themselves. Specify whether a label-only rename preserves identity while invalidating/revalidating presentation evidence, and make the adapter examples agree. A changed fixed target or incompatible type cannot be accepted by simply updating the observation generation; it degrades/refuses the old route and requires the separately admitted new binding/connection. Concrete provider extraction fields can remain an explicit profile-authoring gate, without claiming E14 recognition is solved."
  },
  {
    "file": ".engineering/planning/review-result/discovery-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-06 — P2 — The concrete composition wiring owner is still assigned to generic server.rs\n\n**Sources:** mediated_route:13–14,43–46,99–102; Grafana adapter:9–11,115,132; design:153–164,229–253,832–836; current host server.rs:14,31–47; SDK lib.rs:17–31,54–57.\n\nThe host obligation table assigns composition loading to connectors-host/src/server.rs, while the architectural invariant forbids host-to-concrete-adapter dependencies. The existing server accepts an already constructed Arc<dyn Adapter>; it neither selects provider implementations nor supplies a mediated implementation. Co-location itself is compatible with the design, but the caller constructing both concrete libraries and registering private route ports has no named owner.\n\n**Required correction:** name an explicitly authored application/composition executable as the wiring root. It may depend on the selected parent and child adapter libraries plus generic host/SDK infrastructure, construct them, and inject/register their narrow ports. Parent and child libraries depend on SDK/contracts, never each other; the generic host imports no concrete adapters or provider registry/enum. Provider-specific target interpretation stays with the parent implementation; generic admission, binding, infrastructure and lifecycle remain host ports; product selection belongs to the composition. Add a compact dependency/port diagram and align the obligation table. This needs neither new loader implementation nor a new ESS adapter kind."
  },
  {
    "file": ".engineering/planning/review-result/discovery-a-initial-20260908.md",
    "category": "legacy-source-excerpt",
    "severity": "warning",
    "message": "## DC-A-07 — P2 — Same composition is too broad for the private one-process route boundary\n\n**Sources:** mediated_route:43,67,92,101; Grafana adapter:11,115,130–132; design:793–816,834; CLI migration:55.\n\nThe route says same host process or composition, but the design also uses composition for collections of separately launched or remote services. Sharing a deployment, machine, namespace or configuration file does not make a private Rust port callable across processes. A broad reading would require an unspecified remote provider-traffic proxy, contrary to the selected boundary.\n\n**Required correction:** explicitly restrict this mediated-route profile to parent and child instances wired into the same process/trusted composition runtime with live injected ports. Distinguish that runtime from deployment compositions of service endpoints. A remote operation gateway can forward an ordinary child operation to the owning composition; it cannot export/call the private route port or make an unreachable direct target reachable. Refuse missing registrations, incompatible target/profile, separate-process placement and nested parents before advertising a callable child or dispatching provider traffic. Each adapter retains independently usable direct service/library packaging; no composition is mandatory for direct use. Local supervision/readiness must report missing bindings without dynamic loading, direct fallback or fabricated support."
  },
  {
    "file": "ess/domains/declarations.yaml",
    "category": "legacy-review",
    "severity": "warning",
    "message": "## DC-A-08 — P2 — Proposed observation/route lifecycles and ownership are described as recorded without an actual model\n\n**Sources:** discovery/resources:109–119; mediated_route:100,108–109; auth/connection:161–165; ess/domains/declarations.yaml:40–108; auth_access.yaml:87–100; design:487,959–961.\n\nResourceObservation and RouteBinding tables state entity identities, lifecycles and relations; the resource document even says multiplicity is recorded on Connection.parent reverse. Those declarations are not present in ESS, and Connection persistence is explicitly still unmapped. The two-state observed/withdrawn lifecycle is also unable to describe partial attempts, stale retained observations or valid same-identity re-observation without further decisions. Adapter-transient bindings backed by host metadata blur which owner acknowledges durable publication and recovery.\n\n**Required correction:** make these tables truthful: distinguish settled decision/value shapes from proposed persistent entities/relations, and name the logical host publication/binding owner versus adapter-private interpretation/transient scan work. If the settled coverage/publication/revalidation decisions are typed now, use the applicable ESS value workflow and explicit UNMAPPED live predicates and ownership rather than inventing a persisted lifecycle or a fake owns edge. Do not claim the existing ESS graph already records a Connection.parent cardinality. Retention, recovery and stable generation authority need explicit semantic requirements even if their storage realization remains separately owned.",
    "line": 40
  }
]
```

