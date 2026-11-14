# Basecamp through the catalog provider

A Basecamp personal access token is read-write at Basecamp: it acts as you, in every account and
project you can reach, although this version of the connector offers only reads. Create it, and
revoke it when you no longer use it, at <https://app.basecamp.com/my/access_tokens> (the page
Basecamp's own CLI names in its README, `basecamp/basecamp-cli`; Basecamp's API reference,
`basecamp/bc3-api`, does not name it). Enter it only at the host's hidden prompt
([Connect](#connect)); it is never an argument, and no example here holds one.

The catalog provider reads Basecamp projects, their people, message boards, messages, comments,
to-do sets, to-do lists and to-dos from Basecamp's pinned OpenAPI document. Nothing here is
Basecamp-specific code: the document is lowered from the definition `connectors/basecamp.yaml`
into a bundle and a selection of thirteen reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them. This page configures
the `basecamp` connector and covers what differs for Basecamp. Connector, instance and connection
are defined in [the README](../README.md#vocabulary).

## Source and bundle

The pinned source is
[`adapters/basecamp/upstream/openapi.json`](../adapters/basecamp/upstream/README.md), SHA-256
`6f309fd546710c53747ff546e0459083fd565902df07932aed49d9ba1a2603bb`, Basecamp's OpenAPI 3.1
document from `basecamp/basecamp-sdk` (MIT), retrieved on 2026-11-13, unmodified. It declares 300
operations, every path starting with `/{accountId}/`, and no `servers` or `securitySchemes`.

The definition, `connectors/basecamp.yaml` of
[connectors-definitions](https://github.com/beyond10x/connectors-definitions) (copied here as
`crates/connectors-catalog/tests/fixtures/definitions/connectors/basecamp.yaml`), names the base
URL `https://3.basecampapi.com/{accountId}` with `accountId` in its `config`. Because that URL
ends in a configured variable, the lowering drops `/{accountId}` and its parameter from every
path, and records it in the connector declaration as `endpoint_path`: no read takes `accountId`,
and the instance endpoint's path names the account. The bundle and selection are the lowering:

```sh
cargo run --locked -p connectors-build -- definition \
  --file <definitions>/connectors/basecamp.yaml --root <definitions> --out <bundles>
```

`<definitions>` is a checkout of connectors-definitions and `<bundles>` a scratch directory; its
`basecamp.bundle.json`, index entry, `basecamp/operations.json` and `basecamp/connector.json` are
the committed ones. `adapters/catalog/tests/bundle_drift.rs` lowers the committed definition
again and refuses a committed bundle or index that differs by a byte, and
`crates/connectors-catalog/tests/definition_import.rs` holds the selection and declaration equal
to the lowering. All 300 operations are in the bundle, none unsupported; the selection exposes
thirteen.

## The shipped selection set

[`adapters/catalog/providers/basecamp/operations.json`](../adapters/catalog/providers/basecamp/operations.json)
exposes thirteen reads and no write. `adapters/catalog/tests/basecamp.rs` pins the id list, each
id's `operationId` and its path in the bundle. Each request is sent under the account:
`GET /projects.json` goes to `https://3.basecampapi.com/<account id>/projects.json`.

| id | request | `operationId` | paging | end condition |
|---|---|---|---|---|
| `projects.list` | `GET /projects.json` | `ListProjects` | `page`, `Link` | no `next` |
| `project.get` | `GET /projects/{projectId}` | `GetProject` | single item | n/a |
| `project.people.list` | `GET /projects/{projectId}/people.json` | `ListProjectPeople` | `page`, `Link` | no `next` |
| `message_board.get` | `GET /message_boards/{boardId}` | `GetMessageBoard` | single item | n/a |
| `messages.list` | `GET /message_boards/{boardId}/messages.json` | `ListMessages` | `page`, `Link` | no `next` |
| `message.get` | `GET /messages/{messageId}` | `GetMessage` | single item | n/a |
| `comments.list` | `GET /recordings/{recordingId}/comments.json` | `ListComments` | `page`, `Link` | no `next` |
| `comment.get` | `GET /comments/{commentId}` | `GetComment` | single item | n/a |
| `todoset.get` | `GET /todosets/{todosetId}` | `GetTodoset` | single item | n/a |
| `todolists.list` | `GET /todosets/{todosetId}/todolists.json` | `ListTodolists` | `page`, `Link` | no `next` |
| `todolist.get` | `GET /todolists/{id}` | `GetTodolistOrGroup` | single item | n/a |
| `todos.list` | `GET /todolists/{todolistId}/todos.json` | `ListTodos` | `page`, `Link` | no `next` |
| `todo.get` | `GET /todos/{todoId}` | `GetTodo` | single item | n/a |

- **Finding the ids.** `project.get` answers the project's `dock`: each tool with its `name`
  (`message_board`, `todoset` and others) and `id`. That `id` is the `boardId` of
  `message_board.get` and `messages.list`, or the `todosetId` of `todoset.get` and
  `todolists.list`. `comments.list` takes the id of any recording that has comments, such as a
  message, a to-do or a to-do list, as `recordingId`.
- **Following a page.** Basecamp names the next page of a list only in the `Link` response
  header's `rel="next"`. Every list read answers that URL as `next`, beside `status`, `body` and
  `provenance`; send the `page` number it carries, with the other parameters unchanged, to the
  same read. A page without `next` is the last. The URL is answered only when it is on the
  instance's own origin and under its account; any other `rel="next"` refuses the read as
  `upstream_protocol` (`adapters/catalog/tests/basecamp.rs`). Basecamp sizes pages itself
  (15 on the first, growing to 100); no page size is sent.
- **Filters.** `projects.list` and `todolists.list` take `status` (`archived` or `trashed`;
  active by default), `todos.list` takes `status` and `completed` (`true` for completed to-dos),
  and `messages.list` takes `sort` (`created_at` or `updated_at`) and `direction` (`asc` or
  `desc`). Every other parameter the document does not declare is refused before any request.
- **Paths without `.json`.** Basecamp's API reference writes a single resource as
  `/projects/1.json`; the pinned document writes `GetProject` and the other single-item reads
  without `.json`, and they are sent as the document writes them. Basecamp's own SDKs are
  generated from this document and send the same paths, which their nightly live check against
  Basecamp reads ([upstream README](../adapters/basecamp/upstream/README.md)). This connector has
  not been run against a live Basecamp account.

## Authentication

A Basecamp personal access token, sent as `Authorization: Bearer <token>`, under the profile
`basecamp.pat`. The identity read is `GET /<account id>/my/profile.json`, the token's own person
in that account: its `id` is the subject and `basecamp.person` the kind. Connecting reads it, so a
token Basecamp refuses under the account the endpoint names does not connect. The profile declares
no scope check: Basecamp's authorization document names a token's `scope`, but this version does
not read it.

Every request carries the host's `User-Agent`,
`connectors/<version> (https://github.com/beyond10x/connectors)`, which names the project and a
contact as Basecamp requires; Basecamp answers a request without one `400`.

OAuth through Basecamp's Launchpad is not offered in this version.

## Configure, connect and read

The `basecamp` connector has no default endpoint: the instance's endpoint names the account, as
`https://3.basecampapi.com/<account id>`. The account id is the number after the host in the
address of any page of the account in a browser, and the `id` of the account in
`https://3.basecampapi.com/authorization.json`. Add to the host configuration
(`~/.config/connectors/config.toml`, format `connectors-local/5`):

```toml
[connectors.basecamp]
enabled = true

[connectors.basecamp.instances.work]
endpoint = "https://3.basecampapi.com/999999999"
```

An endpoint without the account (`https://3.basecampapi.com`) is refused when the instance is
derived. A second account is a second instance.

### Connect

Enter the token at the hidden prompt:

```sh
connectors connections connect basecamp/work --profile basecamp.pat --credential-prompt
```

`--endpoint https://3.basecampapi.com/<account id>` on the same command writes the instance's
endpoint first. For automation, `--credential-env <NAME>` names an environment variable holding
the bare token, set by a secret manager for that one command, such as
`op run --env-file basecamp.env -- connectors connections connect basecamp/work --profile basecamp.pat --credential-env BASECAMP_TOKEN`;
its value is never printed.

### A first read

```sh
connectors invoke basecamp/work projects.list --input-json '{}'
connectors invoke basecamp/work projects.list --input-json '{"page":2}'
```

### The configuration it derives

The bundle, the thirteen reads, the profile and the provider executable are derived from the
shipped connector. The instance above is this `connectors-catalog-local/2` configuration, with
the account path as `request_prefix` in front of the bundle's paths
(`adapters/catalog/tests/connector_declarations.rs` holds the two equal):

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "basecamp.work",
  "provider": "basecamp",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://3.basecampapi.com/999999999",
  "request_prefix": "/999999999",
  "auth": {
    "profile": "basecamp.pat",
    "header": "Authorization",
    "bearer": true,
    "label": "Basecamp personal access token",
    "identity": {"path": "my/profile.json", "kind": "basecamp.person", "subject_pointer": "/id"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/basecamp/operations.json"
}
```

## Through the MCP server

`connectors server --transport stdio` offers the thirteen reads as tools of one instance and
connection, configured as [the stdio guide](local-mcp-stdio.md) describes: the exposures in
`CONFIG.mcp-stdio.json` and the declarations of their operations in
`CONFIG.operation-curation.json`, each read declared `generic` with network effects only.
`apps/connectors/tests/basecamp_mcp_tools.rs` runs the production server with a Basecamp
instance and checks that `tools/list` lists the thirteen reads and that a listed tool reads a
page with its `next`. In order:

1. Add the instance with the account id in its endpoint, as under
   [Configure, connect and read](#configure-connect-and-read).
2. Connect it, entering the token at the hidden prompt:
   `connectors connections connect basecamp/work --profile basecamp.pat --credential-prompt`.
3. Write `CONFIG.mcp-stdio.json` with one exposure per read, each naming the connection the
   previous step printed, and `CONFIG.operation-curation.json` with the thirteen declarations.
   The curation entry also pins the instance's executable selection and the digest of its cached
   description; no `connectors` command prints either yet, so this step is not yet possible from
   the command line alone.
4. Start the server, or name this command in the MCP client's configuration:
   `connectors server --transport stdio`.

## Limits

- Verified against a local HTTPS stand-in for Basecamp only (`adapters/catalog/tests/basecamp.rs`):
  the account path, the bearer token and the `User-Agent` of each read, a three-page `Link` walk to
  the last page, `429` with `Retry-After` refused as `rate_limited` with that delay, `404` as not
  found, and an `accountId` from a caller refused. No live Basecamp account has been read.
- Reads only: no to-do, message or comment is created, and no approval is involved. Writes are a
  later version.
- Only personal access tokens; OAuth through Launchpad is not offered.
- The provider does not walk pages itself. It sends a read answered `429` once more after the
  delay its `Retry-After` names, when that wait leaves the second request time of its own before
  the invocation deadline, and otherwise returns it as a `rate_limited` refusal carrying that
  delay as `retry_after_seconds` ([Limits](local-catalog-provider.md#limits)).
- The engine parses and re-serialises the body, so it is returned as equal JSON, not as
  Basecamp's exact bytes.
