---
format: aep.planning-md/3
id: specification:mcp-local-http-binding-design-scope
kind: specification
status: draft
title: Scope the explicit native projection for persistent local MCP HTTP
relations:
- informed_by: epic:mcp-contracts
revision: 1
---
## Purpose

Read-only preparation for persistent local outbound HTTP after the authorized MCP contract waves. This is a design work item, not a newly invented approval gate. The operator already authorized specification refinement and runtime delivery; the three existing caller-assignment, outbound-process and external-execution blockers remain separate.

## Source-grounded reuse

The smallest candidate reuses shared Connection publication and credential lifecycle machinery through an explicit MCP adapter projection. Connection has stable identity, configured instance/profile, publication fence, active generation/custody version and irreversible revocation (ess/domains/auth_bindings.yaml:92). CredentialGeneration references Connection and ServiceConfiguration and records immutable history, not secret storage (ess/domains/credentials.yaml:26).

Reuse the existing AuthProfile/ProfileDeclaration, Acquisition, CredentialSet, CustodyVersion and RefreshAttempt contracts. Existing specified command families cover connection allocation/publication/revocation; acquisition begin/consume/candidate/complete; credential generation recording; custody write/acknowledgement/retirement; and reserve/authorize/store/recover/publish/quarantine refresh. Specified commands are not evidence of implemented MCP behavior. docs/design.md:1346 requires one linearizable metadata authority for the relevant publication, refresh authorization, alias exclusion, revocation and dispatch ordering, with custody as an ordered handoff.

Do not call this deletion ownership: CredentialGeneration references Connection. CustodyVersion formally references ServiceConfiguration, while historical Connection coordinates can survive without a live Connection foreign key (contracts/auth/custody/v1alpha1/semantics.md:70). Publication authority, custody retention and material deletion are distinct responsibilities. Native and shared ESS compile independently; adapters/README.md:68 requires an adapter projection rather than an invented cross-root ESS relation.

## Design facts the projection must select and review

1. Whether binding_ref names endpoint configuration or one identity-bearing access binding; permitted accounts/profiles/Connections per selection; and exact invocation selection.
2. Configured-instance ownership of profile, Connection and custody scope, including changes to endpoint, resource or issuer configuration.
3. Removal/reselection lifetime: preserve, detach or revoke the association, with no invented cascade.
4. The reviewed profile and mechanism establishing ExternalIdentity and provider_authority; issuer, resource audience and account identity are not interchangeable.
5. Association reuse and alias exclusion when selections reach the same rotating material. Distinct generation identifiers alone do not make independently refreshable aliases safe (contracts/auth/acquisition/v1alpha1/semantics.md:98).

These are inputs to a bounded design proposal under existing authority. The read-only inspection did not choose them and did not establish a cardinality from existing code. The native state model at adapters/mcp/spec/ess/domains/state.yaml:239 and final auth requirements at adapters/mcp/contracts/client/v1alpha1/auth.md:11 retain their UNMAPPED prerequisites. A later author must state selected design decisions explicitly, validate the actual native projection and review it against shared ownership before runtime stories. Merely validating an incomplete model does not close the mapping.

## Independent progress and evidence limits

Strict bounded HTTP exchange mechanics can proceed against the sibling's reviewed immutable observation domain without selecting durable credentials. The second authorized composition/CLI-intent wave also keeps these relations unresolved. This preparation ran no builds or validation and changed no source; it is cited design scoping, not a conformance report.
