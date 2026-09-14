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
executable behind the same private protocol as `connectors-gitlab`.

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
`adapters/catalog/providers/<provider>/operations.json`. The GitLab set,
[operations.json](../adapters/catalog/providers/gitlab/operations.json), exposes
every operation the native GitLab adapter exposes, so one configuration serves
the provider from the pinned source alone:

| id | source operation | effect |
|---|---|---|
| `project.get`, `issues.list`, `file.get`, `branch.get` | the matching `getApiV4Projects…` | read |
| `merge_requests.list`, `merge_request.get` | `getApiV4ProjectsIdMergeRequests`, `…MergeRequestIid` | read |
| `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get` | the matching `getApiV4Projects…` | read |
| `job.trace` | `getApiV4ProjectsIdJobsJobIdTrace`, `"response": "text"` | read |
| `merge_request.create` | `postApiV4ProjectsIdMergeRequests`, guarded | write |
| `merge_request.update` | `putApiV4ProjectsIdMergeRequestsMergeRequestIid`, guarded | write |
| `merge_request.merge` | `putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge`, guarded | write |

`adapters/catalog/tests/shipped.rs` loads this file against the committed bundle
and checks that every native read and write id is present. The native adapter's
`merge_request.validate` has no entry: it is `merge_request.get` plus
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
checks the native `merge_request.validate` made in Rust, as five lines of data.

Add the adapter to the host configuration exactly as for
[the native GitLab binding](local-gitlab-cli.md), with `adapter_id = "catalog"`,
the bootstrap's `configuration_revision` from
`connectors-catalog-provider --local-config <file> --print-local-bootstrap`, the
executable's SHA-256 and the selected operation ids in its permissions. Select
`private_protocol = "connectors-private/2"` for writes and publish an approval
policy naming them.

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
- One authentication profile per configuration, a token in one header. OAuth,
  basic and signing profiles are not offered by this provider yet.
- Pagination and error envelopes are not declared; a paged read returns one page
  as the provider answers it.
- A guard compares scalars for equality. It cannot express "any of", ordering or
  a value the preflight must not have.
- Only GitLab has run live. A second provider through the same engine is still
  required by `specification:catalog-http-runtime-handoff`.
