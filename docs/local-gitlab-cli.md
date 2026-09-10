# GitLab through the local CLI

The current local binding supports `project.get`, `issues.list` and `file.get`
using a saved GitLab PAT. It requires Linux x86_64 and the
[qualified Secret Service binding](local-secret-service.md). The separately
installed older CLI is not upgraded by building this checkout.

```sh
CARGO_BUILD_JOBS=2 cargo build --locked -p connectors -p connectors-gitlab
target/debug/connectors --output json setup init
target/debug/connectors --output json setup check
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
target/debug/connectors-gitlab --local-config /absolute/path/gitlab.json --print-local-bootstrap
sha256sum target/debug/connectors-gitlab
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
operations = ["project.get", "issues.list", "file.get"]

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
target/debug/connectors --output json connections connect --adapter forge --profile gitlab.pat --credential-prompt
target/debug/connectors --output json operations describe --adapter forge --operation project.get
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
target/debug/connectors --output json operations invoke --adapter forge --connection CONNECTION --operation project.get --schema SCHEMA --revision REVISION --input-json '{"project":"group/project"}'
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

## Current limits

Native validation evidence lasts at most 60 seconds. After expiry, the connection
reports `pending` and reads refuse. Explicitly revalidate the saved credential:

```sh
target/debug/connectors --output json connections revalidate --adapter forge --connection CONNECTION --expected-revision CONNECTION_REVISION
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
do not yet have this local connection/owner binding. They are next, before MCP and
the remaining providers. Writes, OAuth onboarding, complete management pagination
and reproducible distribution acceptance remain outside this increment.
