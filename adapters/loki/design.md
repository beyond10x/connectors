# loki adapter design

**Status:** proposed independent service `connectors.loki`, not implemented.

## Scope and dependencies

Operation `logs.query_range` selects `datasource.logs/v1alpha1` profile
`logql-range`, preserving the old `loki-query-range` declaration. This slice is read-only.
Native semantics and future fixtures are owned by [logql-range](contracts/logs/v1alpha1/semantics.md).

The adapter depends on shared service, datasource and auth contracts/SDK.
Direct placement needs admitted native-origin egress. Mediated placement consumes
an injected shared mediated-HTTP capability in a statically composed process;
child code knows no concrete parent implementation. Concrete installation and
integration suites belong to [composition](../../docs/compositions/monitoring.md).

## Authentication and configuration

| Profile | Scheme / purpose / subject | Acquisition | Capability |
|---|---|---|---|
| `loki.anonymous` | none / anonymous / none | static_config | http-anonymous, no credential placement |
| `loki.bearer` | http_bearer / service_account / app | static_config | http-bearer, pinned material |
| `loki.basic` | http_basic / service_account / app | static_config | http-basic, pinned user/password |
| `loki.via_parent` | parent / mediated_access / none | static_config route materialization | mediated-http, parent authenticates fixed hop |

Anonymous uses only declared verification if required. Bearer/basic require
identity/validity and declared verification. Mediated uses current parent/route
binding and declared child verification through that route. Verification is an
explicitly admitted bounded native read under the same native scope as ordinary
work; a trivial query must not bypass containment.

The old ambiguous `loki.none` label is descriptive history, not an accepted alias. The selected profile fixes credential placement and applicable evidence under [auth.profile §4.2](../../contracts/auth/profile/v1alpha1/semantics.md#42-explicit-access-bindings-without-child-credentials). Missing bearer/basic material cannot select anonymous. A parent-mediated child has no local credential generation or provider account; it neither inherits the parent's grant nor receives the parent's secret. Direct bearer/basic profiles require a reviewed identity-validation mechanism for that deployment before advertisement; a successful query alone cannot invent a stable account identity. These are proposed profiles, not implemented auth support.


Proposed direct example, requiring a new strict configuration reader:

```json
{
  "auth_profile": "loki.anonymous",
  "http": {
    "base_url": "https://loki.internal",
    "extra_headers": {
      "X-Scope-OrgID": "tenant-a"
    }
  },
  "adapter": {
    "query_scope": {
      "required_equalities": [
        {
          "label": "job",
          "value": "api"
        }
      ]
    }
  }
}
```

The native provider tenant uses X-Scope-OrgID where required by the admitted
deployment. [Shared header rules](../../contracts/auth/capability/v1alpha1/http-headers.md)
govern placement, conflicts and revision fencing. `tenant_header` is an earlier
descriptive label, not a key. Current HttpConfig lacks this extension. A provider
tenant is not application authority.

These configuration outlines require a selected new strict reader. The anonymous form requires the explicit auth_profile selector and forbids credential/registration members, including explicit null. Bearer/basic forms require their configured references; missing or null is invalid, not anonymous. Mediated configuration selects `loki.via_parent` and contains the sealed route binding, with no child base URL or credentials. static_config activates these bindings under host admission; it creates no auth.begin session or protected-entry action.

The admitted parent route owns downstream tenant headers; the child has no direct `extra_headers` override through mediation. Refuse a requested tenant binding the parent route cannot establish.


## Sources and remaining obligations

Original evidence: `../connectors/providers/loki.toml`,
`crates/integration-monitoring/`, `crates/monitoring-model/` and design 08 at
81459ac4. The predecessor `specs/loki/` HTTP OpenAPI 3.1 document is
repository-authored, not a vendor specification. Adopt exact inputs, full digest
and license under this adapter's upstream/ before generation/extraction; central
review archives are historical context, not a packaged native dependency.

The planned Connectors adapter-kind v2 owns request mapping; response interpretation
is an adapter obligation. Unimplemented stubs cannot be advertised. Native query,
auth, decoding, byte/deadline and disclosure fixtures belong here; composition
tests do not replace them. Deferred: live tail and durable/archival continuation.
