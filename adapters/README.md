# Adapter ownership and extraction

`adapters/<owner>/` is the extraction boundary. It owns native behavior, authored
specifications, upstream pins/licenses, generated projections, fixtures, design
notes and implementation. Specification-only directories are valid; they do not
need a placeholder Cargo package or an advertised runtime declaration.

```text
adapters/loki/
├── design.md
├── contracts/logs/v1alpha1/semantics.md
├── spec/ess/
│   ├── system.yaml
│   └── domains/reads.yaml
├── upstream/          # Add exact sources, licenses and pins when adopted
├── generated/         # Compiler-owned projections, when generated
└── src/, tests/       # Add only when implementation is authorized
```

Only the authored documents/model currently exist for Loki. The example does not
claim its runtime, fixtures, upstream package or release tooling has been built.

## Shared dependency and independent versioning

Root [contracts](../contracts/README.md) and [ESS](../ess/system.yaml) own shared
protocols and provider-independent facts; the Rust contract bindings and SDK consume
them. An adapter implements explicitly selected shared contract/profile versions.
Its native profile version, upstream API pin, implementation version and adapter
specification format version evolve independently.

When an adapter is extracted, carry its entire owned directory and replace local
workspace dependencies with an exact reviewed shared-contract/SDK release or
immutable commit and content digest. Include the matching schemas and conformance
inputs. The shared package must contain no sibling adapter inputs. Shared contracts
may continue to be released from this repository; a separate contracts repository
is optional and is not required merely to extract Loki.

There is currently no published shared package or adapter release. This working
tree uses local paths and proposed contract versions. Selecting a reviewed snapshot,
packaging its shared dependencies, rewriting Cargo workspace inheritance and
proving a standalone build/gate are extraction prerequisites, not completed work.
No remote, release or dependency lock format is introduced by this layout change.

A native parser, selector, representation, provider target tuple or source-specific
limit changes in its adapter. Changing it does not require changing the shared
contract package unless a shared guarantee or codec actually changes. Sharing a
result envelope does not make native query/continuation semantics shared.

## Owners

| Owner | Native authority |
|---|---|
| [Atlassian](atlassian/design.md) | [Document/CQL profiles](atlassian/contracts/documents/v1alpha1/semantics.md), [grammar](atlassian/contracts/documents/v1alpha1/cql.md), [ESS](atlassian/spec/ess/system.yaml); Jira and Confluence remain distinct bindings within the intentionally combined adapter |
| [Docker](docker/design.md) | [Container logs](docker/contracts/logs/v1alpha1/semantics.md), lifecycle intent and [ESS](docker/spec/ess/system.yaml) |
| [Kubernetes](kubernetes/design.md) | [Discovery](kubernetes/contracts/discovery/v1alpha1/semantics.md), [permissions](kubernetes/contracts/auth/v1alpha1/semantics.md), [routes](kubernetes/contracts/routes/v1alpha1/semantics.md), [logs](kubernetes/contracts/logs/v1alpha1/semantics.md), restart intent and [ESS](kubernetes/spec/ess/system.yaml) |
| [Grafana](grafana/design.md) | [Datasource discovery](grafana/contracts/discovery/v1alpha1/semantics.md), [proxy route](grafana/contracts/routes/v1alpha1/semantics.md), [ESS](grafana/spec/ess/system.yaml) |
| [Loki](loki/design.md) | [LogQL profile](loki/contracts/logs/v1alpha1/semantics.md), tenant binding and [ESS](loki/spec/ess/system.yaml) |
| [Prometheus](prometheus/design.md) | [PromQL series profile](prometheus/contracts/series/v1alpha1/semantics.md), native configuration and future model obligations |
| [Alertmanager](alertmanager/design.md) | Native alert records, configuration and future model obligations |
| GitLab | Existing spec, pinned upstream, generated ESS/bindings, runtime and tests already live in gitlab/ |
| SQL | Existing declaration, native runtime and tests live in sql/ |
| [Catalog](catalog/design.md) | Optional catalog adapter design |
| [SIP](sip/design.md) | [Dial/effect/media binding](sip/contracts/dial/v1alpha1/semantics.md), native configuration and protocol obligations |
| [RTVBP](rtvbp/design.md) | [Session/authority transport](rtvbp/contracts/session/v1alpha1/semantics.md), native configuration and codec obligations |
| [MCP](mcp/design.md) | [Pinned specification revisions](mcp/contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md); protocol contracts, transport/capability profile selection and native model remain unauthored |

Each authored native model lives in `spec/ess` with namespace
`connectors_<owner>.<domain>`. Shared ESS includes none of these roots and imports
none of their types. Each compiles independently. Native-to-shared projection is
an explicit adapter binding obligation, not a cross-root ESS import or arbitrary
JSON bag. Existing compiler-owned `generated/ess` follows its separate generation
workflow.

A child consumes shared mediated capabilities without knowing its concrete parent.
Parent discovery owns native provider recognition; recognition does not import the
recognized adapter implementation. Concrete pairings and integration suites belong
to [monitoring composition](../docs/compositions/monitoring.md).
[Media composition](../docs/compositions/media-session.md) compares independent
protocol bindings; it is not a fictional combined provider adapter.

Historical design/review evidence may reference old paths and remains immutable.
It is migration evidence, not an extracted adapter's normative dependency. Exact
adopted upstream sources and fixture inputs must be packaged with their adapter;
central review snapshots do not substitute for those packaging inputs.

## Boundary gate

```sh
TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked --offline -p connectors-build -- ess-boundary
```

The repository's Rust gate scans all shared ESS paths, comments and decoded YAML
keys/scalars against [reviewed terminology](../crates/connectors-build/ess-boundary.json)
plus every adapter directory name and optional spec/adapter.json id. Reading an
id discovers an alias; full adapter-declaration validation belongs to the separate
specification workflow. Design-only/spec-only owners participate without fake
runtime declarations. All authored spec/ess roots validate and compile separately.

No per-file suppressions are allowed. The reviewed shared_protocol_names policy
keeps SIP protocol vocabulary valid when a SIP adapter directory exists; it cannot
suppress a forbidden native term or an adapter-path dependency. Shared adapter-path references, source
symlinks, foreign namespaces, unloaded/nested model YAML and manifest/source
mismatches fail. Shared protocol terms such as HTTP, OAuth and SIP are legitimate.
The lexical gate cannot infer unfamiliar provider semantics: new shared types
still require semantic review. It does not validate native behavior or historical
review snapshots, and it is not a provider-name ban on informational index links.
