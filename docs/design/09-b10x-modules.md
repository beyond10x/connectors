# B10x module admission

The B10x Integration admits exact code-first Work and Ontology contracts from the
`b10x/modules` collection. Hosted configuration can make a sorted subset available to every
Identity-verified member of its tenant:

```toml
[b10x]
tenant_member_modules = ["ontology", "work"]
work_origin = "http://b10x-work:8080"
ontology_origin = "http://b10x-ontology:8080"
ontology_bearer_file = "/run/secrets/ontology/bearer"
```

The list is an authorization ceiling. An omitted list preserves existing personal/hosted behavior;
an explicit empty list exposes no Work or Ontology operations to hosted tenant members. Entries
must be sorted, unique, supported, and backed by a configured private origin.

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
Connector state root. Consumers use the ordinary Search/Receive/Replay event protocol and never
receive an owner URL or credential. A 409 owner cursor refusal becomes a typed non-retriable
protocol error requiring resynchronization.

UI discovery remains outside Connectors. Agent loads trusted declarative contributions and admits
only those whose global required-operation IDs resolve against the current effective Connector
surface. Zwirn renders Agent's semantic `WidgetCatalog`/`RenderDocument`; it does not call modules
or inspect deployment configuration directly.
