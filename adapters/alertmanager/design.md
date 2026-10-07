# alertmanager adapter design

**Status:** proposed; not implemented. Proposed independent service `connectors.alertmanager`.

## Scope and dependencies

Operation `alerts.list` selects `datasource.records/v1alpha1` profile
`alertmanager-alerts`, preserving the old `alertmanager-alerts-list` declaration. This slice is read-only.
Native bounded alert records are not series. Their exact schema, filters and safe projection remain authoring obligations; no complete alert binding is claimed.

The adapter depends on shared service, datasource and auth contracts/SDK.
Direct placement needs admitted native-origin egress. Mediated placement consumes
an injected shared mediated-HTTP capability in a statically composed process;
child code knows no concrete parent implementation. Concrete installation and
integration suites belong to [composition](../../docs/compositions/monitoring.md).

## Records model

[Authored ESS](spec/ess/system.yaml) (`connectors_alertmanager.alerts`, `ess/23`) declares
the `alertmanager-alerts` profile, the label-matcher string and an all-optional
`AlertSelection` (`filter`, `silenced`, `active`) taken from the fluxplane
`alertmanager.alerts` input. It declares no alert record. No source states the alert's
identity, label shape, state values or relations, so each is an `UNMAPPED:` marker in
[domains/alerts.yaml](spec/ess/domains/alerts.yaml). So are the `inhibited` filter, the
matcher grammar, the meaning of an absent filter and the page bounds. The owner settles
them before the U24 story is scheduled.

## Route

The route is a native adapter, as this design and parity unit U24 already say
(`docs/fluxplane-plugin-parity.md:799`). Three facts decide it:

| Fact | Native adapter | Catalog provider |
|---|---|---|
| Placement through Grafana's data-source proxy (`alertmanager.via_parent`) | designed | refused: "No mediated routes" (`adapters/catalog/design.md:122`) |
| Result | a typed `datasource.records` item | provider JSON passed through unchanged (`contracts/catalog/v1alpha1/semantics.md:105`) |
| Upstream document | maps requests through adapter-kind v2 | required, pinned under `upstream/` |

The catalog is smaller only for direct placement, an untyped body and a pinnable vendor
document. Not checked: whether that document exists. The repository contradicts itself:
`contracts/catalog/v1alpha1/semantics.md:57` and `adapters/catalog/design.md:140` call the
v0.31.0 document (prefix `0fac1b1f`) vendor OpenAPI 3; `docs/compositions/monitoring.md:29-33`
and Sources below call it repository-authored. Neither file is here or in a local
checkout, and nothing was fetched. `adapters/catalog/design.md:78` also maps `alerts.list`
to `generic-http`, this design to `datasource.records`. Both contradictions go to the owner.

## Operations

| fluxplane operation | calls | unit | Connectors operation |
|---|---:|---|---|
| `alertmanager.alerts` (and the undeclared `alertmanager.alerts.list`) | 1 + 1 | U24 | `alerts.list`, records / `alertmanager-alerts` |
| `alertmanager.test` | 0 | — | `connections revalidate` once a connection exists (`docs/fluxplane-plugin-parity.md:81-83`) |
| `alertmanager.silence.list`, `.create`, `.delete` | 0 | — | out of scope |

Sources: `docs/fluxplane-plugin-parity.md:423-436,717,799` and the fluxplane alertmanager
skill reference.

## Authentication and configuration

| Profile | Scheme / purpose / subject | Acquisition | Capability |
|---|---|---|---|
| `alertmanager.anonymous` | none / anonymous / none | static_config | http-anonymous, no credential placement |
| `alertmanager.bearer` | http_bearer / service_account / app | static_config | http-bearer, pinned material |
| `alertmanager.basic` | http_basic / service_account / app | static_config | http-basic, pinned user/password |
| `alertmanager.via_parent` | parent / mediated_access / none | static_config route materialization | mediated-http, parent authenticates fixed hop |

Anonymous uses only declared verification if required. Bearer/basic require
identity/validity and declared verification. Mediated uses current parent/route
binding and declared child verification through that route. Verification is an
explicitly admitted bounded native read under the same native scope as ordinary
work; a trivial query must not bypass containment.

Parity needs two of these profiles. The fluxplane `endpoint` method stores an optional basic
username and password, "only if the instance needs them" (fluxplane reference, Auth). That
is `alertmanager.anonymous` or `alertmanager.basic`; fluxplane's `endpoint_ref` becomes the
saved connection's `http.base_url`. Bearer and parent-mediated stay as designed; no parity call
needs them yet.

UNMAPPED: the verify operation. The predecessor named one
(`../connectors/providers/alertmanager.toml:11`, cited at `adapters/catalog/design.md:39`),
but that file is not available here. fluxplane `alertmanager.test` reads readiness and
version/cluster status. Which bounded native read becomes `verify_operation` is open.

The old ambiguous `alertmanager.none` label is descriptive history, not an accepted alias. The selected profile fixes credential placement and applicable evidence under [auth.profile §4.2](../../contracts/auth/profile/v1alpha1/semantics.md#42-explicit-access-bindings-without-child-credentials). Missing bearer/basic material cannot select anonymous. A parent-mediated child has no local credential generation or provider account; it neither inherits the parent's grant nor receives the parent's secret. Direct bearer/basic profiles require a reviewed identity-validation mechanism for that deployment before advertisement; a successful query alone cannot invent a stable account identity. These are proposed profiles, not implemented auth support.


Proposed direct example, requiring a new strict configuration reader:

```json
{
  "auth_profile": "alertmanager.anonymous",
  "http": {
    "base_url": "https://alertmanager.internal"
  }
}
```


These configuration outlines require a selected new strict reader. The anonymous form requires the explicit auth_profile selector and forbids credential/registration members, including explicit null. Bearer/basic forms require their configured references; missing or null is invalid, not anonymous. Mediated configuration selects `alertmanager.via_parent` and contains the sealed route binding, with no child base URL or credentials. static_config activates these bindings under host admission; it creates no auth.begin session or protected-entry action.

The admitted parent route owns downstream tenant headers; the child has no direct `extra_headers` override through mediation. Refuse a requested tenant binding the parent route cannot establish.


## Sources and remaining obligations

Original evidence: `../connectors/providers/alertmanager.toml`,
`crates/integration-monitoring/`, `crates/monitoring-model/` and design 08 at
81459ac4. The predecessor `specs/alertmanager/` HTTP OpenAPI 3.1 document is
repository-authored, not a vendor specification. Adopt exact inputs, full digest
and license under this adapter's upstream/ before generation/extraction; central
review archives are historical context, not a packaged native dependency.

The planned Connectors adapter-kind v2 owns request mapping; response interpretation
is an adapter obligation. Unimplemented stubs cannot be advertised. Native query,
auth, decoding, byte/deadline and disclosure fixtures belong here; composition
tests do not replace them. Deferred: webhooks and silence mutations.

## Out of scope

- **Silences.** List, create and delete have 0 calls (parity page, *Not planned until used*,
  `docs/fluxplane-plugin-parity.md:844`). Silence mutations were already deferred above.
  The fluxplane reference states enough to model them once a unit selects them: an `id`
  that delete takes, name/value matchers, state active/pending/expired, creator, comment,
  and a duration on create. Delete "expires" a silence rather than removing it.
- **Grafana's alert operations.** `grafana.alerts.active` and `grafana.alerts.silences.*`
  have 0 calls. They reach this adapter through `alertmanager.via_parent` and need the
  Grafana route (U05) (`docs/fluxplane-plugin-parity.md:287-290`).
- **In-cluster reach.** fluxplane reaches in-cluster instances by port-forward (U19). The
  Connectors counterpart is the Kubernetes service-proxy route
  (`contracts/discovery/mediated_route/v1alpha1/semantics.md:30`), which composition owns.
- **Webhooks**, as above.
