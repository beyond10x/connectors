# Connectors CLI: from 0.7.x to v2

## Runtime foundation, 2026-09-10

Production now consumes the generated parser for private setup and passive
configured adapter inventory. SQLite authority initialization is implemented;
credential custody, connection management and supervised invocation remain
unfinished. The [foundation guide](local-runtime-foundation.md) records the
available commands and refusals. The dated comparison below retains the earlier
runtime baseline; it does not override this update or migrate an installed
0.7.x deployment.

## Current local direction, 2026-09-09

The [local CLI contract](../contracts/cli/v1alpha1/semantics.md) now owns the
selected next interface. The compact groups are `setup`, `adapters`, `connections`
and `operations`; global options select a TOML configuration, private state
directory and human/JSON output. Each configured adapter has `startup = "on-demand"`
(the default) or `startup = "automatic"` when the local host starts. Ordinary
listing never implicitly starts that host or an adapter.

Credentials use a local OS keyring, with separate private non-secret metadata.
Protected terminal, file and stdin entry are application-local acquisition paths;
ordinary management requests and diagnostic output contain no credential bytes.
Durable credential reuse, same-identity repair and terminal revoke are specified.
Linux is the first selected platform. Cloud Identity, hosted Secrets and federation
are separate later bindings; none is a prerequisite for this local stage. MCP is
deferred.

The generated CLI package verifies parsing, type/source selection and process
behavior using recording handlers. It does not replace the executable's current
`describe`, `invoke` or `serve` implementation. Those compatibility paths retain
their current wire codecs and output; the new grouped commands have their own
explicit process contract. Production local custody and adapter supervision still
require runtime work.

The planning owners are `story:local-cli-binding-semantics` and
`story:local-cli-ess-surface`. They supersede the undecided local-surface items
below. The installed 0.7.x binary remains separate; no state or secret is migrated
automatically.

## Historical comparison, 2026-09-08

The remainder preserves the earlier runtime comparison and proposed hosted scope.
Its references to "today" mean that date. In particular, its exclusions of a
personal configuration/state directory, undecided output/entry options and
Secrets-only custody are superseded for the local stage above. They are not
current restrictions on the local CLI design.

- **Status:** forward-looking. The v2 CLI has three verbs today. Most rows below describe a
  planned area or a deliberate non-equivalent. Every "planned" item names the contract it waits
  for; every "not carried" item names the boundary that forbids it.
- **Audience:** people and agents who use the installed `connectors` 0.7.x command line, including
  the `connectors` skill (`~/.codex/plugins/cache/beyond10x/connectors/0.8.1/skills/connectors/SKILL.md`,
  which targets 0.7.1).
- **Rule stated by the operator on 2026-09-08:** v2 does not aim for CLI parity where parity would
  cross a v2 specification or boundary. A v1 verb whose meaning depends on a mandatory catalog, a
  single all-provider daemon, a saved login that redirects commands, or a client flag that widens
  authority has no v2 equivalent and will not get one.

## 1. Baselines

| | v1 | v2 |
|---|---|---|
| Binary | `connectors 0.7.0` installed (`connectors --version`); skill targets 0.7.1 | `apps/connectors/src/main.rs`, built to `target/debug/connectors`; not installed |
| Surface | 8 groups, 27 leaf paths (`../connectors/ess/system/components.yaml`, the `unspecified-path` list) | 3 verbs: `describe`, `invoke`, `serve` |
| Global options | `--output text|compact|json|yaml`, `--config`, `--state-root`, `--target local|hosted` | none; each verb takes `--endpoint`, `--token-file`, `--allow-plaintext` |
| Configuration | one personal file, default `~/.config/b10x/connectors.toml` (`../connectors/crates/connectors-runtime/src/composition.rs:1148-1154`) | one YAML per running service, copied from `examples/` (`README.md`, "Run services") |
| State | one personal state root, default `~/.local/state/b10x/connectors` (`composition.rs:1138-1144`) | no client-side state; services own their state |
| Credentials | stored by the daemon; hidden prompt on `setup connect`; never in argv | `credential: {kind: file | env}` bindings in the service config; files `0600`, owner-only, no symlinks (`contracts/service/v1alpha1/semantics.md`, Auth and configuration) |
| Output | four renderings; failures on stdout for json/yaml | pretty JSON on stdout; a JSON error object on stderr and exit 1 (`apps/connectors/src/main.rs:47-51`) |
| Protocol selection | `--protocol-version v2|v3`, no negotiation | one wire version compiled into the client (`connectors_core::WIRE_VERSION`), no negotiation |

## 2. The boundaries that decide non-parity

| Boundary | Source | v1 features it removes |
|---|---|---|
| The catalog is an optional service, never a compile or runtime dependency | `docs/design.md` § 1.1 item 12; `contracts/catalog/v1alpha1/semantics.md` ("optional; grants nothing") | `inspect providers`, provider picking in `setup init --integration` and `setup connect` |
| A host does not import every concrete adapter; adapters are separate executables | `docs/design.md` § 3.2, § 17.1 | one `serve local` daemon that links all providers |
| The same outward contract works locally, remotely or through federation; placement is configuration | `docs/design.md` § 3.2, § 17.1 | a separate `serve hosted` mode; `--target local|hosted` as a mode switch |
| A saved hosted login never silently redirects a local command; explicit target selection | `docs/design.md` § 17.3 | implicit target from a saved session |
| Authority is receiver-owned configuration and policy; a request cannot widen it | `docs/design.md` § 3.2, § 7; `contracts/service/v1alpha2/semantics.md` § 4 (proposed) | `--allow writes`, `--operator-network` as client flags |
| Discovery never dials, authenticates or materializes; an observation is not a grant | `contracts/service/v1alpha1/semantics.md`, Kubernetes paragraph; `docs/design.md` § 10 | `connection activate` and `connection materialize` as one-step actions |
| Telephony controls are a media session capability, not an operation | `docs/design.md` § 14.1 | `operation signal` |
| One explicit codec per interaction; no automatic fallback or resend, even if a future binary supports several | [service compatibility](../contracts/service/compatibility.md) | `--protocol-version` |
| Secrets never travel through the CLI to a hosted instance as values the CLI can read back; custody is the Secrets service | ADR 0023; `docs/design.md` § 12 | `admin credentials set` in its current shape (open, see § 6) |

## 3. Path-by-path mapping

"Today" is what `target/debug/connectors` does now. "Planned" names the contract or design section
the feature waits for; verb names are not decided unless stated. "Not carried" cites the boundary.

| v1 path and flags | v2 today | v2 planned | Not carried, and why |
|---|---|---|---|
| `setup init --config --state-root --integration --allow-exec-auth --force` | copy `examples/<adapter>.yaml`, edit, start the adapter binary | config validation in the "connection/config management" area (`docs/design.md` § 17.3); verb undecided | one personal file aggregating providers: each service has its own typed config that refuses unknown fields; `--integration` from a catalog: catalog is optional; `--allow-exec-auth`: Kubernetes exec plugin is an `auth.capability` profile decided in the service config, not a CLI flag |
| `setup connect <provider>` (guided, hidden prompt) | not available; only file/env credential bindings | `auth.acquisition/v1alpha1` (proposed): OAuth2 code, client credentials, static entry; the hidden-prompt rule is preserved when it lands | provider picker from a catalog: optional catalog |
| `setup completions` | not available | yes; clap generates it | — |
| `inspect doctor --config --state-root` | `describe --endpoint … --token-file …` per service, plus `GET /healthz` (liveness only) | `auth.evidence/v1alpha1` (proposed) value-free readiness; `connections.list` safe status | one aggregate over a personal state root: there is no client-side state root |
| `inspect providers` | not available | `catalog.list`/`catalog.describe` when a catalog service is configured (`catalog/v1alpha1`, proposed) | a mandatory catalog |
| `inspect auth` | not available (a descriptor never lists credentials) | `connections.list` with safe status (`auth.connection/v1alpha1`, proposed) | reading a credential value: never, unchanged from v1 |
| `session login --no-browser --timeout-seconds` / `session logout` | not available; `--token-file` holds a static service token | Identity access token source for the `identity-audience` admission profile (`service/v1alpha2`, proposed); keyring storage undecided | a login that changes the default target of later commands |
| `serve local --config --state-root` | `serve --config federation.yaml` runs the federation host; each adapter runs as its own binary: `connectors-gitlab --config gitlab.yaml`, `connectors-kubernetes …`, `connectors-sql …` | host launches adapter executables from a local composition ("local adapter lifecycle", `docs/design.md` § 17.1–17.3); verb undecided | one daemon linking every provider |
| `serve hosted` | same binaries; admission is configuration (`static-bearer` today, `identity-audience` proposed) | — | a separate hosted mode |
| `serve mcp` | not available | inbound `/mcp` as a host transport binding onto governed operations (ADR 0028); how it is started is undecided | MCP as the internal contract between client and adapters |
| `connection candidates --target --query --limit` | not available | `resource_discovery/v1alpha1` + `auth.connection` `managed` profile (both proposed) | — |
| `connection activate` | not available | verification through `auth.evidence` after an explicit connection is configured | contacting a provider as part of a listing |
| `connection list --target --query --limit` | not available; each service instance has exactly one implicit connection | `connections.list` (`auth.connection`, `configured` then `managed`) | — |
| `connection observations` | `invoke --operation endpoints.discover` and `hosts.discover` on a Kubernetes service (`README.md`, Supported surfaces) | `resource_discovery` (proposed) | — |
| `connection materialize` | edit `sql.yaml` with the selected endpoint and a credential binding, restart (`contracts/service/v1alpha1/semantics.md`, End-to-end acceptance) | managed connection creation from an observation (`auth.connection` `managed`) | automatic materialization from an observation |
| `event search` / `event receive --channel --after --limit --wait-ms` / `event replay` | not available | `events` family: deferred, no document yet (`contracts/README.md`, "Deferred families") | replay as a retry strategy (`docs/design.md` § 15.2: receipt is not approval) |
| `operation search --query --limit --protocol-version` | `describe` returns every enabled operation with schemas; filter locally (`jq '.operations[] | select(.id | test("issue"))'`) | caller-visible projection under `identity-audience` (`service/v1alpha2` § 3.2, proposed); admitted connections per operation with `auth.connection` | server-side intent search over a descriptor |
| `operation describe --operation` | `describe` (whole descriptor; one call) | — | the per-operation `description_ref` lease: replaced by the descriptor `revision`; a stale revision returns `stale_description` and the client describes again |
| `operation signal` | not available | `media/v1alpha1` sessions (proposed): DTMF is a session capability | DTMF as an operation |
| `operation invoke --operation --connection --description-ref --input-json|--input-file|--input - --approval-evidence-ref --protocol-version` | `invoke --endpoint … --token-file … --operation <id> --input <file>`; the client describes first and sends the current revision | `--connection` with `auth.connection` `managed`; `approval {reference, evidence}` from the mutation profile; executor assertion and Identity token from `service/v1alpha2`; stdin and inline JSON input undecided | `--protocol-version`; `--allow writes` and `--operator-network` |
| `admin integrations status --endpoint --access-token-file --access-token-stdin --no-browser` | not available | `auth.evidence` and `connections.list` under `identity-audience` | — |
| `admin credentials set` | not available; deployments bind file/env credentials | `auth.acquisition` static entry over Secrets custody (ADR 0023) | see § 6: whether a CLI may carry a value to a hosted instance at all |
| global `--output text|compact|json|yaml` | pretty JSON only | undecided | — |
| global `--target local|hosted` | `--endpoint <url>` on every call; through a federation host the operation id carries the source prefix, `gitlab__project.get` (`README.md`) | — | target aliasing by saved login |
| global `--config`, `--state-root` | not on the client; `--config` on each service | — | a client-side state root |

## 4. Concept mapping

| v1 concept | v2 concept | Note |
|---|---|---|
| catalog / provider / integration | adapter instance (one running service with one typed config) | the catalog becomes an optional service that lists bundles and grants nothing |
| connection | connection: `configured` profile today (one per instance), `managed` proposed | stable across reauthorization in both |
| description lease (`description_ref`) | descriptor `revision` | `stale_description` → describe again → resubmit deliberately; never replayed |
| grant | policy decision with `grant_ref` on the audit row (`service/v1alpha2`, proposed) | fail closed when the policy store is unavailable |
| approval evidence ref | `approval: {reference, evidence}` on the invocation after the safe preparation read exposes the canonical leaf subject ([F03](../contracts/service/delegation.md), proposed) | one-time spending by the executing leaf before its dispatch gate; gateway forwarding never spends |
| `connector_audit_ref` | required nullable `audit_ref` plus `audit_status` on the proposed extended response | refs name acknowledged records only; unavailable audit uses null, and explicit unaudited static reads use null / not_required; see [response rules](../contracts/service/compatibility.md#5-extended-responses-audit-and-mutation-observation) |
| owner context (tenant, agent, revision, authority snapshot) | verified context from the credential; agent identity as an `executor` assertion the receiver checks (proposed) | the body can never set tenant or realm |
| personal-local vs hosted | one binary; the admission profile in configuration | `static-bearer` for local; `identity-audience` for hosted |
| event channel | deferred `events` family | no date |
| voice session, DTMF | `sessions/v1alpha1` + `media/v1alpha1` (proposed) | separate from operations |

## 5. What a 0.7.x user can do with v2 today

```sh
# build once
mkdir -p .local/tmp
TMPDIR="$PWD/.local/tmp" cargo build --workspace --locked

# run one adapter from an example config (credential files must be 0600, owner-only)
target/debug/connectors-gitlab --config gitlab.yaml

# discover its contract, then invoke
target/debug/connectors describe \
  --endpoint http://127.0.0.1:7101/ --allow-plaintext --token-file service.secret
target/debug/connectors invoke \
  --endpoint http://127.0.0.1:7101/ --allow-plaintext --token-file service.secret \
  --operation project.get --input examples/requests/gitlab-project.json

# run a federation host over configured downstreams and call through it
target/debug/connectors serve --config federation.yaml
target/debug/connectors invoke \
  --endpoint http://127.0.0.1:7100/ --allow-plaintext --token-file gateway.secret \
  --operation gitlab__project.get --input examples/requests/gitlab-project.json
```

Supported operations today: GitLab `project.get`, `issues.list`, `file.get`; Kubernetes
`resources.list`, `endpoints.discover`, optional `hosts.discover`; SQL `schema.list`, `query.read`
(`README.md`, Supported surfaces). Reads only; no OAuth, writes, events, sessions or process
execution are advertised.

Fixed limits a script must respect (`contracts/service/v1alpha1/semantics.md`, Wire boundary):
64 KiB request body, 4 MiB result, 1–100 requested records, 1,000 SQL rows, 300 s cursor TTL,
no automatic retry, no redirects.

## 6. Not decided

The proposal names `story:cli-governed-surface` as a future owner
(`docs/stack-integration-proposal.md` § 4, S8); that artifact has not been created.
These decisions remain unassigned in the planning store. Until they are recorded,
do not assume any of them.

1. Verb names for the config-validation, connection-management, service-discovery and
   local-lifecycle areas that `docs/design.md` § 17.3 lists.
2. Output modes beyond pretty JSON.
3. How the client obtains an Identity access token: file, stdin, browser flow, keyring.
4. Whether the client keeps a configured endpoint registry (`docs/design.md` § 6.1 "configured
   endpoint or directory entry") or takes `--endpoint` on every call for good.
5. Inline (`--input-json`) and stdin (`--input -`) input.
6. Whether any CLI path may carry a credential value to a hosted instance, given ADR 0023's rule
   that only the admitted Connector workload reads connector-created plaintext.
7. The installed binary name while 0.7.x and v2 coexist; both are `connectors` today.

## 7. Coexistence

- 0.7.x keeps running and keeps its four pinned consumers (`../atlas/docs/catalog.md:79-84`).
  v2 is not registered in Atlas and has no roadmap row; there is no cutover date.
- Nothing is migrated automatically. `docs/design.md` § 23.3 governs state: connection ids,
  credential references and cursors are re-established, not copied; no secret is moved through a
  document or a command line.
- A 0.7.x `connectors.toml` is not read by any v2 binary. A v2 service config is authored from
  `examples/` per service.
- The installed `connectors` stays 0.7.x. Run v2 from `target/debug/connectors` until § 6 item 7
  is decided.
