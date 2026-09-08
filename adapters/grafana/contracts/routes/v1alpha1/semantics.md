# grafana grafana-datasource-proxy/v1alpha1

**Status:** proposed native parent binding, not implemented. Implements
[route.mediated_http/v1alpha1](../../../../../contracts/discovery/mediated_route/v1alpha1/semantics.md).
The shared contract owns one-hop placement, independent child/parent admission,
fixed-target continuity, bounded forwarding and safe failures. This adapter owns
provider lookup, exact proxy construction and target/evidence interpretation.

## Native mapping and source evidence

Grafana execution boundary: seal the UID as the opaque binding; on child invocation re-resolve it and prefix only the reviewed target-relative path with Grafana's data-source proxy route; refuse arbitrary suffixes, absolute URLs, redirects outside the aperture, unknown plugin types, missing targets, stale evidence, cross-placement parents, nested parents; degrade dependants on parent revocation or mapping change; generic `grafana.proxy` out of model | same, lines 161-176 | preserve

- Grafana profile: the parent compares the sealed semantic target against current admitted evidence. Provider re-resolution is an explicit bounded validation step, not a hidden unlimited read on every forward. Missing/changed/unknown target equality is route_unavailable and degrades the child; a public UID/digest or unchanged type alone does not prove equality.

The source identity and observed target come from this adapter's
[discovery binding](../../discovery/v1alpha1/semantics.md), never caller request
fields. Reviewed source-specific mappings cover prometheus/loki/alertmanager;
unknown types never fall through to an arbitrary proxy. The declaration's target
identifiers do not import those adapter implementations. A composition explicitly
installs and admits compatible children.

Seal the datasource UID and fixed semantic destination/tenant/authentication
placement. Re-resolve only as an explicitly budgeted validation step. Unknown,
missing or changed equality is route_unavailable and degrades the child.
An unchanged UID/type or a successful request cannot prove hidden target equality.
The parent owns downstream provider tenant headers; a child cannot override them.


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
