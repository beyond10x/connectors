# GitLab through the catalog provider

The catalog provider runs ordinary HTTP operations straight from a pinned OpenAPI
document. There is no Rust per endpoint: the pinned source is compiled into a
digest-verified bundle, a configuration file selects which operations to expose
and what each one is allowed to do, and one engine binds, sends and classifies.
Adding a supported endpoint changes the selection; it changes no code.

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

## Configure the provider

Build the executable and write an owner-only native configuration:

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-catalog-provider
sha256sum target/release/connectors-catalog-provider
```

```json
{
  "format": "connectors-catalog-local/1",
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
  "operations": [
    {"id": "merge_request.get", "operation_id": "getApiV4ProjectsIdMergeRequestsMergeRequestIid", "effect": "read"},
    {"id": "branch.get", "operation_id": "getApiV4ProjectsIdRepositoryBranchesBranch", "effect": "read"},
    {"id": "merge_request.create", "operation_id": "postApiV4ProjectsIdMergeRequests", "effect": "write",
     "guard": {"preflight": {"operation_id": "getApiV4ProjectsIdRepositoryBranchesBranch",
                             "values": {"id": "id", "branch": "body.source_branch"},
                             "pointer": "/commit/id", "expect": "sha"},
               "postflight": {"pointer": "/sha", "expect": "sha"}}},
    {"id": "merge_request.update", "operation_id": "putApiV4ProjectsIdMergeRequestsMergeRequestIid", "effect": "write",
     "guard": {"preflight": {"operation_id": "getApiV4ProjectsIdMergeRequestsMergeRequestIid",
                             "values": {"id": "id", "merge_request_iid": "merge_request_iid"},
                             "pointer": "/sha", "expect": "sha"},
               "postflight": {"pointer": "/sha", "expect": "sha"}}}
  ]
}
```

The complete configuration used against the sandbox is
[gitlab-catalog.json](evidence/gitlab-sandbox-20260913/gitlab-catalog.json).

- `auth` is the whole authentication profile: the header the token travels in,
  the read that names the credential's subject, an optional read that lists its
  granted scopes, and the scopes the profile requires. The token itself enters
  through the usual protected entry and custody; the file never holds it.
- Each `operations` entry exposes one `operationId` from the bundle under a local
  id. `effect` is declared, not inferred from the method: `read` is allowed only
  for GET, `write` only for POST, PUT, PATCH and DELETE, and a write is a
  required-approval mutation under private protocol two like any other.
- `guard` is optional and declarative. The preflight reads another GET from the
  bundle, binding its parameters from the write's input, and refuses before any
  request when the value at `pointer` is not the input value `expect` names. The
  postflight compares the write's own response the same way; a difference there
  leaves the outcome uncertain and never refused, because the provider may already
  have applied the write. No corrective request is ever issued. This is the C14
  boundary the operator accepted for create and update, written as data.

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
guard reads. Output is the provider's status, its JSON body unchanged, and
provenance whose `source_revision` is the pinned source's SHA-256.

```sh
connectors operations invoke --adapter forge --connection "$connection" \
  --operation merge_request.get --schema "$schema" --revision "$revision" \
  --input-json '{"id":"group/project","merge_request_iid":9}'
```

```json
{"id":"group/project","sha":"<source branch head>",
 "body":{"source_branch":"feature/x","target_branch":"main","title":"Open me"}}
```

Prepare, issue and invoke a write with that file exactly as
[the guarded merge guide](local-gitlab-merge.md) describes. A read that the
provider refuses is an error with a safe code; a write is classified
`not_attempted` when the guard refuses in preflight, `refused` on a documented
definite refusal (400, 401, 403, 404, 405, 409, 410, 412, 415, 422), `applied` on
a 2xx whose postflight holds, and `unknown` for everything else.

## What the sandbox showed

Against the live GitLab, through this provider: a read of merge request 9 and of a
branch; a create that opened merge request 10 at its pinned head; the same create
with a stale pin refused with no request sent; a second create for the same branch
refused by GitLab's 409; an update that retitled 10 with its pinned head; the same
update at a stale pin refused before dispatch; and a create whose branch was moved
the moment the preflight GET appeared, which opened merge request 11 at the moved
head and was classified `unknown`. The provider made exactly one request per
attempt that reached dispatch and none for the two the guard refused. The full
record is in [the sandbox evidence](evidence/gitlab-sandbox-20260913/README.md).

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
- Only GitLab has run live. A second provider through the same engine is still
  required by `specification:catalog-http-runtime-handoff`.
