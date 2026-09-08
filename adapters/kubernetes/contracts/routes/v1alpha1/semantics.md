# kubernetes kubernetes-service-proxy/v1alpha1

**Status:** proposed native parent binding, not implemented. Implements
[route.mediated_http/v1alpha1](../../../../../contracts/discovery/mediated_route/v1alpha1/semantics.md).
The shared contract owns one-hop placement, independent child/parent admission,
fixed-target continuity, bounded forwarding and safe failures. This adapter owns
provider lookup, exact proxy construction and target/evidence interpretation.

## Native mapping and source evidence

Kubernetes `kubernetes_service_proxy_v1`: fixed Service and port; input cannot choose namespace, Service, Pod, port, or suffix; resolves only reviewed Prometheus/Loki/Alertmanager operations; requires `get` on `services/proxy` for that namespace and Service at invocation; sends target-relative GET through the API server; no fallback | `../connectors/docs/design/10-local-kubernetes-context-and-resource-discovery.md:102-116` | preserve

- Kubernetes profile: before each forward, require current exact permission evidence for `get`, core/v1, resource services, subresource proxy, in the fixed namespace and Service name. Collect an uncached result under [evidence §4.4](../../auth/v1alpha1/semantics.md#44-exact-authorization-targets-and-fan-out-budget-f08), sharing the original child's target/call/deadline budget; fresh exact evidence may be reused only under that rule. A denial is route_refused; required evidence unavailable or budget-exhausted is unavailable, with no proxy forward. No generic parent grant or list-services result satisfies this check.

The source identity and observed target come from this adapter's
[discovery binding](../../discovery/v1alpha1/semantics.md), never caller request
fields. Reviewed source-specific mappings cover prometheus/loki/alertmanager;
unknown types never fall through to an arbitrary proxy. The declaration's target
identifiers do not import those adapter implementations. A composition explicitly
installs and admits compatible children.

The target fixes namespace, Service identity and port. Forward only admitted
target-relative GET operations through the API server's services/proxy binding.
Before forwarding use the exact (get,"","v1",services,namespace,name,proxy)
permission under the [native authorization contract](../../auth/v1alpha1/semantics.md).
Service-list permission is not proxy permission; a list denial alone does not
prove proxy denial. Exact proxy denial is route_refused, required unknown or
budget-exhausted permission is unavailable. Neither sends a proxy request.


Exact vendor proxy path forms, native suffix/parameter allowlists, bounded
re-resolution and the complete fixed-target proof remain explicit adapter
authoring/implementation obligations. No path is invented by this relocation.
Unsupported guarantees fail closed before advertisement. Concrete child pairings
and direct/mediated equivalence suites belong to composition; child code consumes
shared capabilities without importing this binding.

## Verification

Required native cases include changed/missing fixed target, unknown mapping,
absolute/path traversal/undeclared suffix rejection, expired/changed evidence,
parent revocation and budget exhaustion, with no direct fallback. Native target
selectors cannot retarget the sealed route. These are specification cases, not
executed fixture results.
