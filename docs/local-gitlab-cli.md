# GitLab through the local CLI

The current local binding supports project, issue and file reads plus exact-commit
CI pipeline, job and trace reads, plus merge-request reads, using a saved GitLab PAT. It requires Linux x86_64 and the
[qualified Secret Service binding](local-secret-service.md). The separately
installed older CLI is not upgraded by building this checkout.

Use the optimized build for the local owner. Startup copies and verifies its
executable within a bounded deadline; large unoptimized debug binaries can exceed
that budget on a busy host. Such a refusal grants no provider dispatch.

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-gitlab
target/release/connectors --output json setup init
target/release/connectors --output json setup check
```

An absolute `--config` and `--state-dir` can select a separate private placement.
Existing installations and credentials are not automatically imported.

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks.
Replace the example target, instance and allowlist with your sandbox coordinates:

```json
{
  "format": "connectors-gitlab-local/1",
  "instance": "gitlab-local",
  "api_base": "https://gitlab.example/api/v4",
  "allowed_projects": ["group/project"]
}
```

The optional `ca_file` names an owner-only PEM certificate file. Inspect the
native bootstrap and hash the built executable:

```sh
target/release/connectors-gitlab --local-config /absolute/path/gitlab.json --print-local-bootstrap
sha256sum target/release/connectors-gitlab
```

Add an adapter entry to the configuration created by setup. Use the bootstrap's
exact `configuration_revision`, the executable's SHA-256 and absolute paths:

```toml
[adapters.forge]
instance_id = "gitlab-local"
adapter_id = "gitlab"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
startup = "on-demand"
restart = "never"

[adapters.forge.permissions]
profiles = ["gitlab.pat"]
operations = ["project.get", "issues.list", "file.get", "pipelines.list", "pipeline.get", "pipeline.jobs", "job.get", "job.trace", "merge_request.get", "merge_requests.list", "merge_request.validate"]

[adapters.forge.executable]
path = "/absolute/path/connectors-gitlab"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/gitlab.json"]
```

Omitted permission sets deny access. Credentials never belong in TOML, native
configuration, executable arguments or environment variables. An optional top-level
`secret_service_socket` selects an explicit private local bus; it does not waive
custody qualification. Replacing a running artifact/configuration requires exact
stop before a subsequent admitted launch.

## Connect and read

Use the hidden native token prompt on your foreground controlling terminal:

```sh
target/release/connectors --output json connections connect --adapter forge --profile gitlab.pat --credential-prompt
target/release/connectors --output json operations describe --adapter forge --operation project.get
```

For automation, `--credential-file /absolute/private/file.json` or
`--credential-stdin` carries the complete native `{"token":"..."}` document.
Files must be singly linked, regular, owner-only (0400 or 0600), with admitted
ancestors. Stdin accepts a deliberate same-session anonymous pipe or admitted
private regular file. Sources are mutually exclusive. Protected values are not
accepted as argv or printed in results.

Retain the connection reference returned by connect, then the descriptor revision
and schema identity returned by operation describe:

```sh
target/release/connectors --output json operations invoke --adapter forge --connection CONNECTION --operation project.get --schema SCHEMA --revision REVISION --input-json '{"project":"group/project"}'
```

Business JSON also supports `--input-file` and `--input-stdin`. The owner validates
the exact original document and current authority before provider dispatch. The
result contains a bounded native JSON text carrier in `result`.

Each CLI invocation may exit independently; the owner and its exact adapter child
remain available. If the owner exits, a later admitted invocation can start it and
reuse the saved exact credential version. Lists, descriptions and status start
nothing. The cache always reports `stale: true` and does not grant permission.

`adapters status --adapter forge` returns incarnation coordinates. Pass those exact
values to `adapters stop` with `--expected-revision`, `--host-incarnation` and
`--child-incarnation`. Stop remains suppressed across owner restart until a later
admitted connect/repair/revalidate/invoke explicitly resumes it. A stale stop cannot signal a
replacement process.

`connections repair` requires `--adapter`, `--connection`, `--expected-revision`
and a protected credential source. A changed GitLab identity is refused; failed
repair preserves the still-valid existing credential. `connections revoke` requires
the same connection/revision selectors and commits terminal local revocation even
when custody or the provider is unavailable. It does not revoke the PAT at GitLab.

## Inspect CI for one commit

Use `operations describe` for each operation's current schema and descriptor
revision, then invoke with the saved connection as above. Select an exact full
40- or 64-character lowercase SHA. The request shapes are:

| Operation | Example input |
|---|---|
| `pipelines.list` | `{"project":"group/project","sha":"0123456789abcdef0123456789abcdef01234567","limit":10}` |
| `pipeline.get` | `{"project":"group/project","pipeline_id":11,"sha":"0123456789abcdef0123456789abcdef01234567"}` |
| `pipeline.jobs` | `{"project":"group/project","pipeline_id":11,"sha":"0123456789abcdef0123456789abcdef01234567","limit":20}` |
| `job.get` | `{"project":"group/project","job_id":42,"pipeline_id":11,"sha":"0123456789abcdef0123456789abcdef01234567"}` |
| `job.trace` | `{"project":"group/project","job_id":42,"max_bytes":10000}` |

Poll the same pipeline ID and SHA with fresh admitted reads within your workflow's
deadline. Preserve native pending/running/failure states; only literal `success`
means pipeline success. A response for another SHA or pipeline fails. Unknown
native statuses remain visible without being interpreted as success.

List results contain `items`, `next_cursor` and `complete`. Pass a returned cursor
alongside unchanged selectors, limit and connection. Pages are observations of a
mutable collection. Pipeline jobs contain current attempts in descending ID order;
retried and trigger-job histories are not part of that collection.

Select the failed job from the verified pipeline's jobs. A trace request uses its
project/job coordinates and makes no independent SHA assertion. Its `item`
contains `job_id`, UTF-8 `content`, retained `bytes` and `complete`. The transport
reads at most 512000 retained bytes; `max_bytes` may narrow this further. Omitted
bytes or a cut UTF-8 suffix keep `complete: false`. Completeness describes this
response, not whether a running job's trace will grow. Missing/erased traces and
permission failures are errors. Trace bytes stay JSON data, including escaped
terminal control characters.

## Inspect merge requests and an update window

Use the same saved connection and current operation description:

| Operation | Example input |
|---|---|
| `merge_request.get` | `{"project":"group/project","iid":42}` |
| `merge_requests.list` | `{"project":"group/project","state":"all","updated_after":"2026-09-01T00:00:00Z","updated_before":"2026-09-10T00:00:00Z","limit":20}` |

The get selector is the project-local IID. List selects an inclusive UTC update
window, sorts by update time ascending and accepts `all`, `opened`, `closed`,
`locked` or `merged`. Follow `next_cursor` with unchanged selectors and connection
until it is null and `complete` is true. This proves traversal exhaustion, not a
snapshot or lossless change feed. The consumer owns overlap, deduplication and
checkpoint policy; a failed or partial traversal must not advance a complete
collection checkpoint.

Results retain bounded native fields, including nullable source project and SHAs.
Unknown merge status stays visible. An observed head or merge status does not
validate a future merge, supply an approval or prove the outcome of a lost write.
Read the [MR contract](../adapters/gitlab/contracts/merge-requests/v1alpha1/semantics.md)
for exact projection and refusal rules. Governed MR writes remain unimplemented.

## Validate a pinned MR head

Discover `merge_request.validate` and invoke it with the same saved connection:

```json
{"project":"group/project","iid":42,"sha":"0123456789abcdef0123456789abcdef01234567","pipeline_id":17}
```

The returned `item.checks_passed` is true only for an opened, non-draft MR with
the exact requested head, `mergeable` status and the selected successful head
pipeline for that SHA. Otherwise `item.blockers` explains the negative result,
including changed head, missing checks or a different pipeline. Inspect this
field even when the CLI exits successfully: a negative validation is a completed
read. Unknown statuses cannot pass, and malformed or inaccessible responses
produce normal typed errors.

`item.merge_performed` is always false. A passing observation neither reserves
the head nor authorizes a later write; asynchronous provider checks and mutable
MR state still require current checks at dispatch. See the
[validation contract](../adapters/gitlab/contracts/merge-requests/v1alpha1/validation.md)
for the exact predicates and supported pipeline semantics.

## Current limits

Native validation evidence lasts at most 60 seconds. After expiry, the connection
reports `pending` and reads refuse. Explicitly revalidate the saved credential:

```sh
target/release/connectors --output json connections revalidate --adapter forge --connection CONNECTION --expected-revision CONNECTION_REVISION
```

Use the connection revision returned by connect or connection status. This command
needs no credential source; it has a 30-second budget including startup and checks
the exact saved version through the native identity/token reads. Success preserves
the connection identity, revision and credential version while replacing evidence.
Transient provider failure preserves any still-valid evidence. A known invalid,
expired or missing credential requires repair; revalidation cannot revive it.
Concurrent repair/revoke or another successful revalidation refuses stale results.
An unknown acknowledgement is observed through connection status without replay.
Disposable HTTPS/keyring fixtures cover restart reuse and revalidation after real
evidence expiry. Dedicated GitLab sandbox acceptance remains open.

Kubernetes and PostgreSQL retain their existing explicit service operations but
do not yet have this local connection/owner binding. GitLab MR and changed-record
workflows and their governed write controls come first, followed by Kubernetes,
PostgreSQL, MCP and the remaining providers. Writes, OAuth onboarding, complete management pagination
and reproducible distribution acceptance remain outside this increment.
