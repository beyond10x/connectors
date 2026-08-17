# B10x module admission

The B10x Integration admits exact code-first Work and Ontology contracts from the
`b10x/modules` collection. Hosted configuration can make a sorted subset available to every
Identity-verified member of its tenant:

```toml
[b10x]
tenant_member_modules = ["ontology", "planner", "work"]
work_origin = "http://b10x-work:8080"
ontology_origin = "http://b10x-ontology:8080"
planner_origin = "http://b10x-planner:8080"
module_signing_key_file = "/var/run/b10x-module-auth/private.pem"
module_signing_key_id = "developer-1"
module_signing_issuer = "b10x-connectors"
```

The list is an authorization ceiling. An omitted list preserves existing personal/hosted behavior;
an explicit empty list exposes no Work or Ontology operations to hosted tenant members. Entries
must be sorted, unique, supported, and backed by a configured private origin.

Personal-local composition can route the same module operations to owner-only Unix sockets instead
of hosted origins:

```toml
[b10x.connection]
connection_ref = "connection:module:work"
label = "Work (local)"
grant_ref = "grant:module:work"
initiation = "b10x"

[b10x.module_sockets]
work = "/absolute/private/runtime/work.sock"
```

One module cannot declare both a socket and an origin. The local route must be an absolute Unix
socket owned by the current effective user beneath an owner-only real directory. It uses the
module's fixed local caller admission, so Connector sends neither hosted request JWS nor a bearer;
the kernel peer/filesystem boundary replaces only transport authentication. The Connection,
operation description, Grant, approval, and audit boundaries remain Connector-owned. Zwirn's
managed module composition generates one such value-free deployment per module and reads only the
exact `b10x` descriptive catalog provider, without enumerating unrelated Integrations.

**2026-08-16 authority amendment.** An origin is invalid unless the three module-signing settings
are present, and the retired `ontology_bearer_file` is refused. After Identity authority and exact
operation admission, Connectors signs every Work/Ontology request—including owner event polls—with
the dedicated Ed25519 key. The compact JWS binds tenant, initiating subject, immediate actor,
operation, method, path/query, body and idempotency digests, authority snapshot, module audience,
a maximum 30-second lifetime, and a one-time identifier. Modules receive only the corresponding
public key. See [design 11](11-hosted-module-request-authority.md).

Identity-verified tenant members may invoke the fixed read-only Work/Ontology subset without the
deployment operator group. The hosted receiver checks this exact subset before dispatch; writes,
other B10x operations, and external-provider invocations remain operator-only. Module global
operation IDs such as `work/request.list` and `ontology/claim.query` are accepted by
Describe/Invoke in addition to established Connector compatibility names. This lets Agent resolve
a declarative UI contribution's `required_operations` without guessing provider names. It remains
the controller's responsibility to reacquire a fresh description lease and check grants/approval
at every invocation.

Work's owner event feed is polled through `/api/work/v2/events`. Connectors validates the module
envelope and event allowlist, translates the opaque deployment cursor into its own decimal sequence
space, deduplicates by owner event ID, and durably stores the envelope/checkpoint under the
Connector state root partitioned by the verified tenant. A legacy unpartitioned checkpoint can be
opened only when configuration admits exactly one explicit tenant; ambiguity fails startup.
Consumers use the ordinary Search/Receive/Replay event protocol and never
receive an owner URL or credential. A 409 owner cursor refusal becomes a typed non-retriable
protocol error requiring resynchronization.

UI discovery remains outside Connectors. Agent loads trusted declarative contributions and admits
only those whose global required-operation IDs resolve against the current effective Connector
surface. Zwirn renders Agent's semantic `WidgetCatalog`/`RenderDocument`; it does not call modules
or inspect deployment configuration directly.
