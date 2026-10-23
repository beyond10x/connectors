# Connectors v2

Connectors gives applications and agents a consistent way to discover and use
integrations. Each connector exposes the typed operations of one provider's API through
shared, versioned contracts; you configure an instance of it, connect it once and invoke
its operations. An application can call a connector's service directly or reach several
through an optional federation host. The words are defined once, under
[Vocabulary](#vocabulary).

**Documentation: <https://beyond10x.github.io/connectors/>**, starting at
[Getting started](https://beyond10x.github.io/connectors/docs/getting-started). The site's
source is [website/](website/README.md): concepts, guides, the CLI, crate, contract and model
reference, and the status of every capability, each held by a test.

The aim is to make an integration useful on its own and predictable when combined
with others. Provider behavior stays with its connector's implementation; shared
contracts define what callers can expect from access, results and failures.

```mermaid
flowchart LR
    app[Application or agent] --> client[Generic client]
    client --> catalog[Catalog provider · GitLab]
    client --> host[Optional federation host]
    host --> kube[Kubernetes service]
    host --> sql[SQL service]
```

For example, Kubernetes can report a database endpoint, while the SQL connector
provides bounded reads from an explicitly configured database. Discovering the
endpoint supplies information; selecting its connection and granting access are
separate steps.

## Vocabulary

The CLI, the configuration and the library use these words, and every page of the
documentation links here for them:

| Term | Meaning | Google | GitLab |
|---|---|---|---|
| provider | The company or technology behind one or more connectors. | Google | GitLab |
| connector | One service or app of a provider, enabled in configuration. It points at one API. | Gmail, Drive, Calendar, Slides | GitLab |
| auth profile | One way to authenticate that a connector declares. | `google.oauth` | `gitlab.pat` |
| instance | Your name for one configured connector: its endpoint, an optional description, and at most one connection. Names are free (`private`, `work`). An instance belongs to one connector and is never shared across connectors. | `gmail/private` | `gitlab/work` at a self-hosted host |
| connection | The stored credential of one instance, obtained through one auth profile. | the consent behind `gmail/private` | the token behind `gitlab/work` |
| operation | One callable tool of a connector. | `users.drafts.send` | `merge_request.merge` |

Two instances of one connector exist when you connect twice, for example with two
accounts or two hosts; the instance name addresses the right one. Every operation of
an enabled connector is available on its instances. Configuration does not group
operations or hide a subset; a library that wants a narrower surface projects one over
it. A write that requires approval still needs a proof issued for its exact input.

"Adapter" is not one of these words. It is the internal name of an implementation (the
catalog engine, a credential binary) and of the process that serves an instance.

The typed model of providers, connectors and their instances, with its cardinalities, is
the generated
[declarations model](https://beyond10x.github.io/connectors/docs/reference/contracts/model/domains/connectors-declarations);
connections and their states are in the
[auth bindings model](https://beyond10x.github.io/connectors/docs/reference/contracts/model/domains/connectors-auth_bindings).

### Configure instances

A `connectors-local/5` configuration enables connectors by name and names their instances.
`setup init` creates the file and the metadata store; it still writes the older
`connectors-local/2` format with an empty legacy `[adapters]` table, so replace that file's
content with yours, keeping `owner_uid` (your user id):

```toml
format = "connectors-local/5"
owner_uid = 1000
default_instance = "gitlab/work"

[connectors.gmail]
enabled = true

[connectors.gmail.instances.private]
description = "personal mailbox"

[connectors.gitlab]
enabled = true

[connectors.gitlab.instances.work]
endpoint = "https://gitlab.example.com"
```

An instance without `endpoint` uses its connector's default (`https://gmail.googleapis.com` for
Gmail); a GitLab endpoint is the host. An instance names only what is yours: its endpoint, an
optional description and, where its connector declares more than one auth profile (Zendesk),
`profile`. Everything else comes from the connector shipped with the `connectors-catalog-provider`
installed beside `connectors`: the executable and its SHA-256, the configuration revision and
every operation. `default_instance` selects an instance for a command that names none.

```sh
connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" instances list
```

```json
{"adapters":[{"adapter":"gitlab.work","adapter_id":"catalog","connector":"gitlab","instance":"work","instance_id":"gitlab.work","restart":"never","startup":"on-demand"},{"adapter":"gmail.private","adapter_id":"catalog","connector":"gmail","instance":"private","instance_id":"gmail.private","restart":"never","startup":"on-demand"}],"source":"configuration"}
```

Results keep the legacy `adapter` alias and the internal `adapter_id` beside `connector` and
`instance` for the deprecation window.

### Select an instance

A command that acts on one instance takes `--connector` and `--instance`; an instance name is
unique within its connector only:

```sh
connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  connections list --connector gitlab --instance work
```

```json
{"connections":[],"observed_at_ms":1792181808006,"source":"authority","stale":false,"valid_until_ms":1792182108006}
```

The deprecated `--adapter <alias>` still selects the same instance (`--adapter gitlab.work`) and
says so once, on standard error, or as a `deprecations` member of the JSON envelope:

```text
deprecated: --adapter; select with --connector <connector> --instance <instance>
```

`instances list`, `describe`, `status` and `stop` replace the deprecated `adapters` group, and
`connections reconnect` replaces the deprecated `connections repair`; each old name runs the same
command and says it is deprecated the same way. The
[CLI reference](https://beyond10x.github.io/connectors/docs/reference/cli) lists every command.

### Connections: Live, Stale, Revoked

An instance holds at most one connection. `connections connect` on an instance that already
holds a Live or Stale connection is refused before it asks for a credential, naming what helps
instead; connect a second account as a second instance.

| From | To | When |
|---|---|---|
| Live | Stale | the provider refused the stored credential: an expired or revoked refresh token, a changed password, a revoked token |
| Stale | Live | `connections reconnect` published a new credential for the same instance and connection |
| Live, Stale | Revoked | `connections revoke` ended local use |

Refreshing an OAuth access token is not a state change; only a refused refresh token makes a
connection Stale, and no retry can renew it. An invoke on a Stale connection answers with the
next action `repair_connection`, which names `connections reconnect`.
`connections describe`, `reconnect`, `revalidate`, `launch` and `revoke` take the instance's
one connection when `--connection` is left out.

### Move a legacy configuration

A legacy `[adapters.<alias>]` entry keeps working: it is selected as connector `adapter_id`,
instance `<alias>`, until it is moved. `setup migrate` moves each catalog entry whose provider
ships a connector to `[connectors.<connector>.instances.<alias>]`, keeping its instance id,
configuration revision, executable and custody item names byte for byte, so its connections and
approval policy stay valid with no new consent. Other entries stay where they are, each with its
reason. `--dry-run` writes nothing; on a file with one legacy GitLab entry `forge`:

```sh
connectors --config "$PWD/config.toml" --state-dir "$PWD/state" --output json \
  setup migrate --dry-run \
  | jq -c '.result | {disposition, format_before, format_after}, (.entries[] | {connector, instance, disposition})'
```

```json
{"disposition":"planned","format_before":"connectors-local/2","format_after":"connectors-local/5"}
{"connector":"gitlab","instance":"forge","disposition":"migrated"}
```

`--apply` keeps the file it read beside the new one:

```sh
connectors --config "$PWD/config.toml" --state-dir "$PWD/state" --output json \
  setup migrate --apply | jq -r '.result.disposition, (.result.backup_path | split("/") | last)'
```

```text
applied
config.toml.before-migrate-20261016T201716Z
```

The moved instance is then selected as `--connector gitlab --instance forge`. Rolling back is
restoring the backup. The [CLI contract on legacy entries](contracts/cli/v1alpha1/semantics.md#migrating-adapter-entries)
lists every reason an entry is kept.

## Where the project stands

These connectors run today; each one's page lists its operations. The [changelog](CHANGELOG.md)
records each release's scope and remaining work; the [status page](https://beyond10x.github.io/connectors/docs/status) lists each capability with its test.

| Connector | Implemented operations | Binding |
|---|---|---|
| [GitLab](https://beyond10x.github.io/connectors/docs/connectors/gitlab/) (catalog provider) | [53](https://beyond10x.github.io/connectors/docs/connectors/gitlab/) | GitLab API v4 from the pinned OpenAPI source; one bound request per operation, the provider's body unchanged; merge guarded by five declared checks, auto-merge and reopen by the pinned head with their fixed body value, discussion resolve by the requested, resolvable discussion and the answered id and state, tag and release writes by the caller's commit, tag delete proven by a 404 after, pipeline, commit, file and branch writes by the caller's commit (commit and file writes onto an existing branch only), branch delete proven by a 404 after, project create by the namespace path and id; commit, file and project bodies sent as JSON through a cited media-type correction; code search with `scope` held to `blobs`; issue creation, notes and replies unguarded; the host's permitted operation ids are the scope |
| [Gmail](https://beyond10x.github.io/connectors/docs/connectors/gmail/) (catalog provider) | [10](https://beyond10x.github.io/connectors/docs/connectors/gmail/) | Gmail API v1 from the pinned Discovery document projected to OpenAPI; profile `google.oauth` (`oauth2_refresh`), one instance per Google account consenting to `gmail.readonly` and `gmail.compose`; mail leaves only as a draft: `users.drafts.send` guarded by a preflight `users.drafts.get` whose `message.id` must equal the input's `messageId`, `users.drafts.create` unguarded, `users.messages.send` not selected; verified against a local fixture only, see [the Gmail guide](docs/catalog-google-gmail.md) |
| [Google Drive](https://beyond10x.github.io/connectors/docs/connectors/drive/) (catalog provider) | [9](https://beyond10x.github.io/connectors/docs/connectors/drive/) | Drive API v3 from the pinned Discovery document projected to OpenAPI; profile `google.oauth`, one instance per Google account consenting to `drive.readonly` and `drive.file`; metadata writes only, no upload: `files.update` guarded by a preflight `files.get` whose `version` must equal the input's, `files.create` and `files.copy` unguarded; verified against a local fixture only, see [the Drive guide](docs/catalog-google-drive.md) |
| [Google Calendar](https://beyond10x.github.io/connectors/docs/connectors/calendar/) (catalog provider) | [6](https://beyond10x.github.io/connectors/docs/connectors/calendar/) | Calendar API v3 from the pinned Discovery document projected to OpenAPI; profile `google.oauth`, one instance per Google account consenting to `calendar.readonly` and `calendar.events`; `events.patch` and `events.delete` guarded by a preflight `events.get` (status `confirmed`, `etag` equal to the input's), `events.insert` unguarded; every write names `sendUpdates`; verified against a local fixture only, see [the Calendar guide](docs/catalog-google-calendar.md) |
| [Google Slides](https://beyond10x.github.io/connectors/docs/connectors/slides/) (catalog provider) | [5](https://beyond10x.github.io/connectors/docs/connectors/slides/) | Slides API v1 from the pinned Discovery document projected to OpenAPI; profile `google.oauth`, one instance per Google account consenting to `presentations` and `presentations.readonly`; `presentations.batchUpdate` guarded by a preflight `presentations.get` whose `revisionId` must equal the body's `writeControl.requiredRevisionId`, `presentations.create` unguarded; verified against a local fixture only, see [the Slides guide](docs/catalog-google-slides.md) |
| [Jira](https://beyond10x.github.io/connectors/docs/connectors/jira/) (catalog provider) | [17](https://beyond10x.github.io/connectors/docs/connectors/jira/) | Jira Cloud platform REST v3 from the pinned OpenAPI document; profile `atlassian.basic` (account email and API token as HTTP basic), shared with Confluence; `issue.transition.run` guarded by the issue's current status and the open transition's target before the POST, and the target status after it; `issue.create`, `issue.comment.add` and `issue.link.add` read the project or issues first; `issue.edit` and `issue.delete` are refused as `conflict` unless the issue's `fields.updated` is the caller's stamp, and read again after; rich text is Atlassian Document Format sent as given; the writes are verified against fixtures only; live, only the API gateway form with a service-account token has run (`issues.search`, 2026-09-30); see [the Jira guide](docs/catalog-jira.md) |
| [Confluence](https://beyond10x.github.io/connectors/docs/connectors/confluence/) (catalog provider) | [11](https://beyond10x.github.io/connectors/docs/connectors/confluence/): 4 under `confluence` (REST v2), 7 under `confluence-v1` (REST v1) | Confluence Cloud REST v2 from the pinned OpenAPI document, and REST v1 from a document this repository writes from Atlassian's reference pages; profile `atlassian.basic`, shared with Jira; reads only, one page per call; verified against local fixtures only, see [the Confluence guide](docs/catalog-confluence.md) |
| [Zendesk](https://beyond10x.github.io/connectors/docs/connectors/zendesk/) (catalog provider) | [10](https://beyond10x.github.io/connectors/docs/connectors/zendesk/) | Zendesk Support API from the pinned OpenAPI document; two profiles, `zendesk.oauth` (OAuth client credentials) and `zendesk.basic` (API token), chosen per instance by `profile`; the incremental exports read from a `start_time`; the three ticket writes are refused as `conflict` unless the ticket's `updated_at` is the caller's stamp, a note and a draft reply are private comments; the writes are verified with `zendesk.basic` only; verified against a local fixture only, see [the Zendesk guide](docs/catalog-zendesk.md) |
| [HubSpot](https://beyond10x.github.io/connectors/docs/connectors/hubspot/) (catalog provider) | [2](https://beyond10x.github.io/connectors/docs/connectors/hubspot/) | HubSpot CRM Objects `2026-09` from the pinned OpenAPI document; private-app token as `Authorization: Bearer` from keyring custody, profile `hubspot.private-app`; reads only, any object type, no time filter; verified against a local fixture only, see [the HubSpot guide](docs/catalog-hubspot.md) |
| [Slack](https://beyond10x.github.io/connectors/docs/connectors/slack/) (catalog provider) | [22](https://beyond10x.github.io/connectors/docs/connectors/slack/): 17 on the bot-token instance, 5 on a second instance with a user token | Slack Web API from the pinned Swagger 2.0 document projected to OpenAPI; two pinned instances, since no Slack connector declaration ships: a bot token as `Authorization: Bearer` from keyring custody, and a user token under `slack.user` for search; the document's `token` parameter is never sent; each write is a required-approval mutation guarded by an `auth.test` preflight and an answer that must both carry `"ok": true`, and `files.delete` also reads the file with `files.info` first; one page per call; `file.download` reads a file's `url_private` as base64 with its media type, length and SHA-256, at most 2 MiB, only on a connection whose `hosts` admits `https://files.slack.com` with the credential; a Slack error is returned as `200` with `"ok": false`; verified against a local fixture only, see [the Slack guide](docs/catalog-slack.md) |
| [Opsgenie](https://beyond10x.github.io/connectors/docs/connectors/opsgenie/) (catalog provider) | [5](https://beyond10x.github.io/connectors/docs/connectors/opsgenie/) | Opsgenie REST API from the pinned Swagger 2.0 document projected to OpenAPI, with the incident-to-alert link added by a cited amendment; API integration key as `Authorization: GenieKey <key>` (`auth.prefix`) from keyring custody, proved with `GET /v2/alerts`; reads only, one page per call over a `createdAt` window the caller advances; verified against a local fixture only, see [the Opsgenie guide](docs/catalog-opsgenie.md) |
| [Runpod](https://beyond10x.github.io/connectors/docs/connectors/runpod/) (catalog provider) | [4](https://beyond10x.github.io/connectors/docs/connectors/runpod/) | Runpod REST API v1 from the pinned OpenAPI source; API key as `Authorization: Bearer` from keyring custody; both writes are unguarded required-approval mutations, a create refused for lack of capacity is refused with code `capacity`, a create without a definite answer is `unknown`; verified against a local fixture only, see [the Runpod guide](docs/catalog-runpod.md) |
| [OpenRouter](https://beyond10x.github.io/connectors/docs/connectors/openrouter/) (catalog provider) | [4](https://beyond10x.github.io/connectors/docs/connectors/openrouter/) | OpenRouter API from the pinned OpenAPI document; management key as `Authorization: Bearer` from keyring custody, profile `openrouter.api-key`; two pinned instances, since no OpenRouter connector declaration ships; `key.create` is an unguarded required-approval write whose created key is held in custody as a new connection of a second instance and never returned; one page per call; verified against a local fixture only, see [the OpenRouter guide](docs/catalog-openrouter.md) |
| [Kubernetes](https://beyond10x.github.io/connectors/docs/connectors/kubernetes/) | [15](https://beyond10x.github.io/connectors/docs/connectors/kubernetes/), 10 of them when switched on in the native file | Kubernetes API, with namespace and resource-kind restrictions (`pods`, `services`, `deployments`, `endpointslices`, `replicasets`, `statefulsets`, `events`); namespaces are the configured ones, never the cluster list; saved bearer token through the local CLI. Helm release values and manifests are disclosed only as redacted projections. `pods.logs` (flag `pod_logs`) reads a bounded tail of one container's log; `contexts.list` lists the contexts of the one kubeconfig the local configuration names (`kubeconfig`), never credentials; `nodes.list` (flag `node_reads`) and `custom_resources.list` (the configured `custom_resources` collections) read one bounded page at a time; `pods.exec` (flag `pod_exec`) runs an argument vector in a named container, local CLI only, on private protocol two under approval, bounded in time and output, an unknown outcome never resent; verified against recorded answers and streams, see [the Kubernetes guide](docs/local-kubernetes-cli.md) |
| SQL ([PostgreSQL](https://beyond10x.github.io/connectors/docs/connectors/postgres/), [MySQL](https://beyond10x.github.io/connectors/docs/connectors/mysql/)) | 7 ([PostgreSQL](https://beyond10x.github.io/connectors/docs/connectors/postgres/), [MySQL](https://beyond10x.github.io/connectors/docs/connectors/mysql/)) | PostgreSQL or MySQL, chosen per connection by `engine` (PostgreSQL by default), with read-only transactions or sessions and execution deadlines; `profile.read` runs a read profile the native file names; saved password through the local CLI; see [the PostgreSQL guide](docs/local-postgres-cli.md) and [the MySQL guide](docs/local-mysql-cli.md) |
| [Tavily](https://beyond10x.github.io/connectors/docs/connectors/tavily/) | [3](https://beyond10x.github.io/connectors/docs/connectors/tavily/) | Tavily API from the pinned OpenAPI source, profile `tavily/2026-10`; API key from keyring custody, checked with `GET /usage`; every operation is a read Tavily serves as a POST; see [the Tavily guide](docs/local-tavily.md) |
| [Loki](https://beyond10x.github.io/connectors/docs/connectors/loki/) | [5](https://beyond10x.github.io/connectors/docs/connectors/loki/) | Loki HTTP API, one GET each; bearer token from keyring custody, proved with `GET /loki/api/v1/labels`; `logs.profile_lines` and `logs.call_trace` run line and trace profiles the native file names; no tenant header is sent; see [the Loki guide](adapters/loki/README.md) |
| [Grafana](https://beyond10x.github.io/connectors/docs/connectors/grafana/) | [5](https://beyond10x.github.io/connectors/docs/connectors/grafana/): 1, and 4 IRM reads on a second instance | Grafana HTTP API, one `GET /api/datasources`; service-account token from keyring custody, proved with that same GET; records carry uid, name, type, access and default flag only. The IRM reads run under `grafana.irm_service_account` on a stack whose `irm_api_url` is configured, read only. A Loki or Prometheus data source is read through Grafana as a Loki or Prometheus connection on its proxy path; see [the Grafana guide](adapters/grafana/README.md) |
| [Prometheus](https://beyond10x.github.io/connectors/docs/connectors/prometheus/) | [3](https://beyond10x.github.io/connectors/docs/connectors/prometheus/) | Prometheus HTTP API, one GET each; bearer token from keyring custody under `prometheus.bearer`, proved with `GET api/v1/status/buildinfo`; a range spans at most seven days and 2,000 points per series; no tenant header is sent; directly or through Grafana's data-source proxy; see [the Prometheus guide](adapters/prometheus/README.md) |

Credentials stay in the platform's store, fixed when the binary is built
([§5.1](contracts/auth/custody/v1alpha1/semantics.md#51-platform-bindings-of-the-local-host)): on
Linux the Secret Service, or the read-only `mounted_secret` binding a `[custody]` table selects
for a container with no keyring, which refuses OAuth refresh and approval signing
([§5.2](contracts/auth/custody/v1alpha1/semantics.md#52-the-read-only-mounted-secret-binding),
[guide](docs/local-mounted-secret-custody.md)); on macOS the login keychain; on Windows the
Credential Manager. Kubernetes and SQL also run as separate Rust services, which advertise no
writes; writes run only through the local CLI under approval. MCP has specified contracts and no
runtime; the two decision blockers on its contracts are cleared
([MCP](https://beyond10x.github.io/connectors/docs/connectors/mcp/)).

## Get started

Build the workspace from the repository root with Rust 1.91.0 or later. The
independent libraries are also checked on Rust 1.88.0:

```sh
cargo build --workspace --locked
```

Ordinary builds use checked-in generated Rust. They do not require ESS or refresh
vendor specifications; Cargo may need to download dependencies on the first build.

To use the local CLI, configure an instance as [Vocabulary](#vocabulary) shows and follow the
[Getting started](https://beyond10x.github.io/connectors/docs/getting-started) page and the
guide of its connector. To run a connector as a standalone service instead, choose a service
configuration from [examples](examples/), set its permitted resources and private credential
references, then follow [Run connector services](docs/running-services.md). For an end-to-end local
exercise with disposable Kubernetes and PostgreSQL services, use the
[live acceptance recipe](docs/live-e2e.md).

The [development guide](docs/development.md) covers the full gate, pinned
generation tools and embedding the internal adapter libraries.

## Explore the design

- [Vision](VISION.md): who this serves, the intended system and how we will judge success.
- [Contracts](contracts/README.md): shared guarantees, versions and support status.
- [Implementations](adapters/README.md): native contracts, provider ownership and extraction boundaries.
- [Connector vocabulary](docs/design/connector-vocabulary.md): why the terms above replaced the
  old ones, and what changed with them.
- [Architecture and design](docs/design.md): decisions, rationale and implementation boundaries.
- [Documentation website](website/README.md): concepts, guides, generated CLI, crate, contract and
  model reference, status and executable examples; [design and boundaries](docs/website-design.md).

Run the local website with `npm ci --allow-git=root` and `npm start` from `website/` after its
[toolchain setup](website/README.md#start-the-preview). The preview is served at
http://127.0.0.1:3100/connectors/. The `Documentation validation` workflow builds the site on
every pull request and push, and `Documentation site` deploys a build of `main` to
<https://beyond10x.github.io/connectors/>. Follow an illustrated GitLab
request walkthrough or inspect the advanced contract exercises. Both use fictional
Rust/WASM behavior, separately from the connector runtime; specified user authorization
is labeled separately from configured federation available today.

Agents making repository changes should start with [AGENTS.md](AGENTS.md).

## Licence

MIT OR Apache-2.0, as `Cargo.toml` declares.
