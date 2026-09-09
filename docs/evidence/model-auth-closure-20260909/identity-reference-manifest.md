# Final auth identity/reference manifest

Domain/file: `connectors.auth_bindings` / `ess/domains/auth_bindings.yaml`.
This supersedes preparation only by adding the explicit prepublication target
reference; existing identity/type choices are unchanged.

| Entity | Identity type | Meaning |
|---|---|---|
| AuthProfile | profile_record_ref: String | Private immutable exact adapter_id/profile_id/declaration_revision key; authored logical profile_ref remains separate |
| Connection | connection_ref: String | Existing stable host-qualified ref, privately allocated before initial material capture/publication |
| Acquisition | acquisition_ref: String | One coordinator's non-reusable acquisition ref |
| CustodyVersion | version_ref: String | Private exact scope_ref/store_version key |

All new instance_id references target existing
`connectors.declarations.ServiceConfiguration.instance_id: String`.
AuthProfile.adapter_id references existing AdapterSpecification.adapter_id: String.
Connection references AuthProfile through profile_record_ref, optional parent
Connection through parent_connection_ref, optional existing CredentialGeneration
through active_generation_id and optional/bounded custody through active/superseded
version references. Existing CredentialGenerationId remains a newtype of Uuid.
These are references, never deletion ownership.

Acquisition fixes target_connection_ref: String at creation, referencing the private
new or established repair Connection. Optional repair_connection_ref must equal
that target when present; optional result_connection_ref equals it exactly on
Completed. Every other state has no yielded result. No public pending Connection
is required; the private target allows the existing immutable generation to acquire
its real Connection reference without a cyclic publication prerequisite.

Coordinator joins are exact patches in the task scratch:
`shared-joins.patch` adds domain registration, CredentialGeneration.connection via
its unchanged String connection_ref, optional
OperationDeclaration.requires_auth: Optional<List<connectors.auth_bindings.AuthRequirement>>,
and stale owner-comment corrections. AuthRequirement is the adapter-local
`{profile: String, scopes: List<String>}` value. It neither replaces
OperationDeclaration.profile nor implies implemented reader support.

`mediated-route-join.patch` adds
Connection.mediated_route: Optional<connectors.discovery.MediatedRouteBinding>
only after the real discovery type is integrated. The route value carries no second
parent/child authority and no live observation foreign key that forces retention.
It was excluded from the independent auth unit compile, with no stub fabricated.

No existing entity identity, lifecycle, old field type or named type changed in the
unit plus available joins. CredentialGeneration fields remain immutable and have
no custody foreign key; retention links live on the Connection/coordinator. An
existing DispatchAdmission reaches Connection through its generation relation.
