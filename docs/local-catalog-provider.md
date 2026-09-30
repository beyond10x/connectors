# GitLab through the catalog provider

The catalog provider runs ordinary HTTP operations straight from a pinned OpenAPI
document. There is no Rust per endpoint: the pinned source is compiled into a
digest-verified bundle, a selection set says which operations to expose and what
each one is allowed to do, and one engine binds, sends and classifies. Adding a
supported endpoint changes the selection; it changes no code.

This is the `provider` realization of
[the catalog design](../adapters/catalog/design.md) and applies
`architecture-decision-record:declarative-http-provider-runtime`. The host keeps
what it already owns: connection admission, approval policy and proofs, audit,
the attempt ledger, dispatch and recovery. The provider is one more adapter
executable behind the same private protocol as the Kubernetes adapter.

## Build the bundle

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider gitlab \
  --source adapters/gitlab/upstream/openapi_v3.yaml \
  --directory adapters/catalog/generated/bundles \
  --auth-profile gitlab.pat
```

The pipeline reads the source (JSON or YAML), records its SHA-256, extracts the
operation inventory and writes `gitlab.bundle.json` plus `index.json`. The
committed GitLab bundle carries every one of the 1,847 operations in the pinned
source with none unsupported; `adapters/catalog/tests/bundle_drift.rs` refuses a
committed bundle that a fresh run would not reproduce byte for byte. Nothing
here reaches the network.

## The shipped selection set

The repository reviews and ships one selection set per provider under
`adapters/catalog/providers/<provider>/operations.json`; Jira Cloud is described
in [the Jira guide](catalog-jira.md), Confluence Cloud in
[the Confluence guide](catalog-confluence.md) and HubSpot CRM in
[the HubSpot guide](catalog-hubspot.md). The GitLab set,
[operations.json](../adapters/catalog/providers/gitlab/operations.json), exposes
every operation the retired native GitLab adapter exposed, so one configuration
serves the provider from the pinned source alone, plus four repository reads:

| id | source operation | effect |
|---|---|---|
| `project.get`, `issues.list`, `file.get`, `branch.get` | the matching `getApiV4Projects…` | read |
| `merge_requests.list`, `merge_request.get` | `getApiV4ProjectsIdMergeRequests`, `…MergeRequestIid` | read |
| `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get` | the matching `getApiV4Projects…` | read |
| `job.trace` | `getApiV4ProjectsIdJobsJobIdTrace`, `"response": "text"` | read |
| `merge_request.create` | `postApiV4ProjectsIdMergeRequests`, guarded | write |
| `merge_request.update` | `putApiV4ProjectsIdMergeRequestsMergeRequestIid`, guarded | write |
| `merge_request.merge` | `putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge`, guarded | write |

The repository reads list one page per call. Each takes `page` and `per_page`;
a caller has walked the list when a page comes back shorter than `per_page`.
GitLab serves at most 100 items per page, so with a larger value every page
would be short and the walk would stop after page one. The pinned source
declares no range, so the shipped selection bounds `per_page` to 1 through 100
on every list read (`issues.list`, `merge_requests.list`, `pipelines.list`,
`pipeline.jobs` and these four): a value of zero or below never ends a walk on a
short page. A value outside that range, or one that is not an integer, is
refused as `invalid_input` before any request. The provider returns `status`, `body` and `provenance`, not GitLab's
`X-Next-Page` header. Every other query parameter the pinned source declares
is accepted by name.

| id | source operation | request | time filter | effect |
|---|---|---|---|---|
| `projects.list` | `getApiV4Projects` | `GET /projects`, e.g. `membership`, `simple=false`, `archived`, `order_by=last_activity_at` | `last_activity_after` | read |
| `tags.list` | `getApiV4ProjectsIdRepositoryTags` | `GET /projects/{id}/repository/tags` | none; each tag carries its commit id | read |
| `releases.list` | `getApiV4ProjectsIdReleases` | `GET /projects/{id}/releases` | none; each release carries `released_at` | read |
| `project.events` | `getApiV4ProjectsIdEvents` | `GET /projects/{id}/events` | `after`, `before` (dates) | read |

`projects.list` returns each project unchanged, including `archived`,
`created_at`, `last_activity_at` and `path_with_namespace`. All four need only
the `read_api` token scope.

`adapters/catalog/tests/shipped.rs` loads this file against the committed bundle
and pins the complete list of shipped ids, so a renamed, dropped or added id
fails the gate. The native
`merge_request.validate` has no entry: it was `merge_request.get` plus
`pipeline.get` and a comparison, which the merge guard now makes itself.

## Configure the provider

Build the executable and write an owner-only native configuration:

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-catalog-provider
sha256sum target/release/connectors-catalog-provider
```

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "gitlab-sandbox",
  "provider": "gitlab",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://gitlab.example/api/v4",
  "ca_file": "/absolute/private/ca.crt",
  "auth": {
    "profile": "gitlab.pat",
    "header": "PRIVATE-TOKEN",
    "bearer": false,
    "label": "GitLab personal access token",
    "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
    "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
    "minimum_scopes": ["api"]
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/gitlab/operations.json"
}
```

The complete configuration used against the sandbox is
[gitlab-catalog-2.json](evidence/gitlab-sandbox-20260913/gitlab-catalog-2.json).

- `auth` is the whole authentication profile: the header the token travels in,
  the read that names the credential's subject, an optional read that lists its
  granted scopes, and the scopes the profile requires. The token itself enters
  through the usual protected entry and custody; the file never holds it.
- `auth.scheme` is `token` when omitted: the protected entry is `{"token":"..."}`
  and travels in `header`, prefixed `Bearer ` when `bearer` is true. `basic` is
  HTTP basic for providers whose API tokens are used with an account name: the
  protected entry is `{"account":"...","token":"..."}` and every request carries
  `Authorization: Basic base64(account:token)`. A basic profile must state
  `"header": "Authorization"` and `"bearer": false`, and names the account
  prompt in `account_label`; the account may not contain a colon. The profile is
  offered to the host as `http_basic` with the fields `account` and `token`.

```json
"auth": {
  "profile": "tracker.api-token",
  "scheme": "basic",
  "header": "Authorization",
  "bearer": false,
  "account_label": "Account email",
  "label": "API token",
  "identity": {"path": "myself", "kind": "tracker.user", "subject_pointer": "/accountId"}
}
```
- `oauth2_refresh` is for providers that issue expiring OAuth access tokens
  from a refresh token that does not rotate, such as Google for installed apps
  (`architecture-decision-record:oauth-material-as-static-entry`). The protected
  entry is `{"client_id":"...","client_secret":"...","refresh_token":"..."}`,
  each field under the token rules, and nothing else. The provider posts it
  form-encoded (`grant_type=refresh_token`) to `token_url`, with no credential
  header, and sends the access token it gets back as
  `Authorization: Bearer <access token>`, so the profile must state
  `"header": "Authorization"` and `"bearer": true`. The access token lives in
  the provider process only, keyed by a digest of the entry, until 60 seconds
  before its `expires_in`; a read or write the API refuses as an invalid credential
  evicts it, and `validate` always exchanges afresh. A token answer carrying a
  different `refresh_token` is refused as an invalid credential and nothing is
  kept. `invalid_grant` and `invalid_client` are an invalid credential; on an
  `operations invoke` of a read on an existing connection the CLI reports it
  with `next_action: repair_connection`, and while connecting with
  `retry_explicitly`. A guarded write refused for its credential reports
  `retry_status` until `story:guarded-write-credential-refusal-says-repair`
  lands; a 429 is a provider
  rate limit and a 5xx is `unavailable`. `token_url` and `authorize_url` must
  be `https` URLs without credentials, query or fragment, written in canonical
  form. `authorize_url` and `requested_scopes` are never called or checked by
  the provider: they reach the host as the profile's `acquisition`, next to
  `token_url`, for the CLI to obtain the entry by consent
  ([Google OAuth guide](catalog-google-oauth.md)).
  `token_ca_file` names trust roots for the token host only; omit it to use
  the platform roots. Like `ca_file`, its bytes enter the configuration
  revision and its path does not. It applies to the provider's refresh and
  validation exchanges, not to the code exchange the CLI makes during consent,
  which uses the platform roots only. Each `minimum_scopes` and
  `requested_scopes` entry is one scope: an empty entry or one holding
  whitespace is refused at load. The profile is offered to the host as
  `http_bearer` with the fields `client_id`, `client_secret` and
  `refresh_token`.
- `identity.source` is `api` when omitted, the read shown above. `id_token`,
  for `oauth2_refresh` only, takes the subject from the `sub` of the token
  answer's `id_token`, after checking that its `iss` is
  `https://accounts.google.com` or `accounts.google.com` and its `aud` is the
  entry's `client_id`, that its `exp` is after now and that its `iat`, if
  present, is at most five minutes ahead, and the granted scopes from the answer's
  space-separated `scope`, which `minimum_scopes` is checked against. The
  `id_token` came straight from the token endpoint over verified TLS, so its
  signature is not checked. When the answer has no `id_token`, the provider
  asks `tokeninfo` on the token host for the fresh access token and reads the
  same `sub`, `aud` and `scope` there. A wrong issuer or audience, or an expired
  `id_token`, is refused as a protocol failure. An `id_token` identity names no `path`,
  `subject_pointer` or `scopes` read.

```json
"auth": {
  "profile": "google.drive",
  "scheme": "oauth2_refresh",
  "header": "Authorization",
  "bearer": true,
  "label": "Google refresh token",
  "identity": {"source": "id_token", "kind": "google.user"},
  "minimum_scopes": ["https://www.googleapis.com/auth/drive.readonly"],
  "token_url": "https://oauth2.googleapis.com/token",
  "authorize_url": "https://accounts.google.com/o/oauth2/auth",
  "requested_scopes": ["openid", "https://www.googleapis.com/auth/drive.readonly"]
}
```
- `operations_file` names a shipped selection set; its `provider` must match. An
  inline `operations` list is accepted as well and comes first. The selection
  ids, in either place, are what the host permits and the approval policy names.
- Each selection exposes one `operationId` from the bundle under a local id.
  `effect` is declared, not inferred from the method: `read` is allowed only for
  GET, `write` only for POST, PUT, PATCH and DELETE, and a write is a
  required-approval mutation under private protocol two like any other.
- `response` is a reviewed exception for a read whose source declares JSON where
  the provider answers plain text, as GitLab does for a job trace. Without it the
  bundle decides: a 2xx declared only as `text/…` is read as text, anything else
  as JSON. A write never carries it.
- `bounds` is optional: `{"<parameter>": {"minimum": <n>, "maximum": <n>}}`
  narrows a query parameter the source declares, such as a provider's page-size
  cap; `minimum` may be omitted. The value, whether sent as a number or a
  string, must be a decimal integer no greater than `maximum` and no less than
  `minimum` (`-0` is zero); anything else is refused as `invalid_input` before
  any request. A bound on a parameter the operation does not declare as a query
  parameter, or with a `minimum` above its `maximum`, is refused when the
  selection loads. The declared input schema carries both limits as well.
- `guard` is optional and declarative. The preflight reads another GET from the
  bundle, binding its parameters from the write's input, and refuses before any
  request unless every check holds. A check compares the scalar at a JSON
  `pointer` in the observed body with `{"input": "<key>"}` (a top-level input or
  `body.<key>`) or `{"literal": "<value>"}`. The postflight applies its checks to
  the write's own response; a difference there leaves the outcome uncertain and
  never refused, because the provider may already have applied the write. No
  corrective request is ever issued. This is the C14 boundary the operator
  accepted for create and update, written as data.

The merge guard in the shipped set reads the merge request and requires, before
the one PUT: `/sha` equal to the pinned `body.sha`, `/state` literally `opened`,
`/detailed_merge_status` literally `mergeable`, `/head_pipeline/id` equal to the
input `pipeline_id` and `/head_pipeline/status` literally `success`. After it:
`/state` literally `merged` and `/sha` still the pinned head. Those are the
checks the retired native `merge_request.validate` made in Rust, as five lines of data.

## Bind the provider to the local CLI

The local CLI requires Linux x86_64 and the
[qualified Secret Service binding](local-secret-service.md). Use the optimized
build for the local owner: startup copies and verifies the executable within a
bounded deadline, and a large debug binary can exceed that budget on a busy host.

```sh
target/release/connectors --output json setup init
target/release/connectors --output json setup check
target/release/connectors-catalog-provider --local-config /absolute/path/gitlab-catalog.json --print-local-bootstrap
sha256sum target/release/connectors-catalog-provider
```

Add an adapter entry to the configuration created by setup, with the bootstrap's
exact `configuration_revision`, the executable's SHA-256 and absolute paths. The
operation ids are the selection ids; omitted permission sets deny access.

```toml
[adapters.forge]
instance_id = "gitlab-sandbox"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[adapters.forge.permissions]
profiles = ["gitlab.pat"]
operations = ["project.get", "issues.list", "file.get", "branch.get", "merge_requests.list", "merge_request.get", "pipelines.list", "pipeline.get", "pipeline.jobs", "job.get", "job.trace", "merge_request.create", "merge_request.update", "merge_request.merge"]

[adapters.forge.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/gitlab-catalog.json"]
```

Credentials never belong in TOML, native configuration, executable arguments or
environment variables. Connect with the hidden token prompt on your controlling
terminal, or `--credential-file` / `--credential-stdin` carrying the protected
`{"token":"..."}` document (`{"account":"...","token":"..."}` for a basic
profile) from an owner-only source:

```sh
target/release/connectors --output json connections connect --adapter forge --profile gitlab.pat --credential-prompt
target/release/connectors --output json operations describe --adapter forge --operation project.get
```

Connect runs the declared identity and scope reads and refuses a token below
`minimum_scopes`. Validation evidence lasts `evidence_lifetime_ms` (60 seconds by
default); after expiry the connection reports `pending` and reads refuse until
`connections revalidate --adapter forge --connection CONNECTION --expected-revision REVISION`
renews it with no credential re-entry. `connections repair` replaces an invalid
credential and refuses a changed identity; `connections revoke` is terminal local
revocation and does not revoke the token at GitLab. Lists, descriptions and status
start nothing. Writes need `private_protocol = "connectors-private/2"` and an
approval policy naming them; see [the guarded merge guide](local-gitlab-merge.md).

## Invoke

Input is one property per declared path or query parameter, named as the source
names it, a `body` object where the operation takes one, and any extra value a
guard reads. Output is the provider's status, its body unchanged (JSON, or a
string for a text read), and provenance whose `source_revision` is the pinned
source's SHA-256.

```sh
connectors operations invoke --adapter forge --connection "$connection" \
  --operation merge_request.get --schema "$schema" --revision "$revision" \
  --input-json '{"id":"group/project","merge_request_iid":9}'
```

```json
{"id":"group/project","merge_request_iid":10,"pipeline_id":19,
 "body":{"sha":"<source branch head>"}}
```

Prepare, issue and invoke a write with that file exactly as
[the guarded merge guide](local-gitlab-merge.md) describes. A read that the
provider refuses is an error with a safe code; a write is classified
`not_attempted` when the guard refuses in preflight, `refused` on a documented
definite refusal (400, 401, 403, 404, 405, 409, 410, 412, 415, 422), `applied` on
a 2xx whose postflight holds, and `unknown` for everything else.

## What the sandbox showed

Against the live GitLab, through this provider and the shipped selection set:
all eleven reads answered, the job trace as text; a merge with a stale pinned
`sha` and a merge naming the wrong pipeline were both refused before dispatch
with no request to the merge endpoint; the merge at the pinned head with the
successful pipeline merged request 10 with exactly one PUT; and an update of the
now-merged request was refused by the literal `/state` check with no request
sent. Before that, with the selection typed into the configuration: a create that
opened merge request 10 at its pinned head, the same create with a stale pin
refused with no request sent, a second create for the same branch refused by
GitLab's 409, an update that retitled 10, the same update at a stale pin refused
before dispatch, and a create whose branch was moved the moment the preflight GET
appeared, which opened merge request 11 at the moved head and was classified
`unknown`. The full record is in
[the sandbox evidence](evidence/gitlab-sandbox-20260913/README.md).

## Limits

- The bundle records parameters, media types and response statuses; it does not
  carry request or response schemas. A `body` is passed through as the caller
  supplies it and validated only by the provider.
- Header and cookie parameters are not carried; a selection whose operation
  requires one is refused at load.
- One authentication profile per configuration: a token in one header, a
  basic profile (`"scheme": "basic"`) sending an account and API token as HTTP
  basic, or an OAuth refresh profile (`"scheme": "oauth2_refresh"`). Signing
  profiles and rotating refresh tokens are not offered by this provider. The
  basic and OAuth refresh profiles have run only against the local fixture, not
  a live provider, and the provider does not acquire the OAuth entry itself.
- Pagination and error envelopes are not declared; a paged read returns one page
  as the provider answers it.
- A guard compares scalars for equality. It cannot express "any of", ordering or
  a value the preflight must not have.
- Only GitLab has run live. A second provider through the same engine is still
  required by `specification:catalog-http-runtime-handoff`.
