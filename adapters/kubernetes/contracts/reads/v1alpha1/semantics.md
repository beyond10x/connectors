# kubernetes implemented read profiles/v1alpha1

This adapter owns the native behavior of its existing local implementation.
Shared wire envelopes and admission remain in
[service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md).
This relocation changes no runtime or selected codec.

Kubernetes: paginated allowed resource lists, EndpointSlice-derived endpoint
observations, and explicitly enabled node/host discovery. Namespace and kind
allowlists precede HTTP dispatch. ResourceVersion and continue tokens remain
observable; expired snapshots return a stale-cursor failure. Endpoint observations
carry source UID, namespace, address/port/protocol, service association, readiness
and observation revision. Classifications are candidates, not connection grants.
Discovery never dials, authenticates to, or materializes the observed endpoint.
Source facts: https://kubernetes.io/docs/reference/using-api/api-concepts/ and
https://kubernetes.io/docs/concepts/services-networking/endpoint-slices/.
Source access: 2026-09-08.

## Descriptor visibility

The implemented adapter filters disabled hosts.discover from its public descriptor.
With a current descriptor revision, shared public lookup returns not_found before
the separate adapter-internal forbidden guard. An old revision fails stale_description
first. This is a native implementation fact, not a universal shared disabled-operation
policy.
