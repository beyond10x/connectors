# Design 12: one owner for every connection to the outside

**Status:** accepted direction, partially implemented · **Date:** 2026-08-20

**Inputs:** [Design 01](01-domain-model.md) · [Design 02](02-architecture.md) ·
[Design 03](03-beyond-http.md) · [Design 07](07-credential-custody-topologies.md) ·
[VISION](../VISION.md)

## The goal, in one sentence

**Connectors is the single owner of every connection B10x makes to something outside itself —
the credential, its acquisition, its refresh, its storage, and the request it authorises — and it
works the same way on a laptop with no deployment as it does in a cluster.**

## What that means for a person

Today, to read your own GitLab from your own machine you install `dex`, or `fluxplane-plugin`, or
both, and each keeps its own copy of your token. Three tools know your credentials and none of them
agrees with the others about what you are allowed to do with them.

After: you run `connectors init`, name the provider once, and every consumer — the CLI, the Zwirn
workbench, an agent mid-conversation — reaches it through the same process, under the same grant,
with the same audit trail. **None of them ever sees the token.** Adding a provider is a row in a
configuration file, not a new program.

That is the whole product claim, and it only holds if one component owns all of it. A second
credential store anywhere — in Zwirn, in an Integration, in a helper CLI — breaks it.

## The two rules the layout follows

1. **Auth is separated from execution.** How a credential *arrives* — pasted, imported from a file,
   fetched by an OAuth2 authorization code, refreshed on expiry — is a different concern from how a
   request is *built and sent*. They meet at a store, addressed by
   [`connector-address`](../../crates/connector-address), and neither knows the other's shape.
   This is what makes OAuth2 Connectors' job rather than each provider's.

2. **Posture is which ports you bind, not which branch you take.** Personal, hosted and satellite
   differ in where secrets live, where state lives and what the network admits. Those are three
   ports. Everything above them is identical code, and an Integration's constructor names no
   posture at all.

## Where everything sits

```text
foundation/connectors/
  catalog/                     61 providers as reviewed data — the bytes a human read
  specs/ providers/            their sources and the declarations compiled from them

  crates/
    ── data, engine-free, opens no socket ──────────────────────────────
    connector-spec             the compiler IR
    catalog-build catalog-cli  the offline build
    catalog catalog-reader     the compiled table, embedded in the binary
    connector-address          credential addressing, pure
    connector-resolve          template + input + credential -> request, pure

    ── the three ports a deployment binds ──────────────────────────────
    connector-secrets          SecretStore:  memory | file | OS keyring | Vault
    connector-state    (new)   StateStore:   memory | SQLite | PostgreSQL
    service                    ConnectorBackend, planning, the egress port,
                               and the datasource scaffolding all backends share

    ── port implementations ────────────────────────────────────────────
    state-sqlite       (new)   rusqlite, bundled: `:memory:` for tests, a file locally
    hosted-state               PostgreSQL cells
    hosted-vault               Vault
    server                     egress transport, the owner-only local socket,
                               the Identity-authenticated hosted HTTP surface

    ── acquisition: how a credential arrives ───────────────────────────
    connector-auth     (new)   OAuth2 authorize/token/refresh driven by declared
                               catalogue data; loopback callback custody; token state.
                               One implementation, every provider that declares a flow.

    ── execution ───────────────────────────────────────────────────────
    integration-catalog        every declared HTTP provider. One adapter, 61 providers.
    driver-sip driver-audio    the non-HTTP drivers, closed set
    driver-cdp
    integration-sip            voice sessions
    integration-kubernetes     only what kubeconfig exec auth adds beyond HTTP
    integration-b10x     module sockets and signed module requests

    ── composition: the central place ──────────────────────────────────
    connectors-runtime         compose(config, ports) -> BackendRegistry
                               bind(registry, socket) -> the local daemon
                               serve_hosted(config)   -> the deployed service

    ── surfaces ────────────────────────────────────────────────────────
    connectors-console         operator flows as a library: init, connect, doctor,
                               providers, rendering
    connectors-cli             a thin frontend over the two above, and nothing else
```

## Who calls what

| caller | how |
|---|---|
| `connectors` CLI | links `connectors-console` + `connectors-runtime::compose` |
| Zwirn, local placement | links the **same two crates**, in the same process |
| Zwirn, hosted placement | the wire protocol, over Identity |
| An agent | the wire protocol, through its harness |
| The cluster | the **same binary**, `serve-hosted` |

`products ──▶ connectors` is the permitted dependency direction
([`dependency-rules.md`](../../../../architecture/architecture/dependency-rules.md)). The link is made from
Zwirn's own crates; **agent** must never compile connectors implementation, and does not.

## What this removes

| absorbed by `integration-catalog` | lines today |
|---|---|
| `integration-gitlab` | ~2,900 |
| `integration-jira` | ~3,200 |
| the dispatch and credential halves of `integration-slack` | of ~5,400 |
| the dispatch and credential halves of `integration-monitoring` | of ~2,700 |

What legitimately survives in a provider crate is what is **not** HTTP: Slack's Socket Mode event
stream and multi-identity registration, Grafana's federated datasource discovery, Kubernetes'
credential-plugin exec auth. Everything that is "build a declared request, place a declared
credential, send it inside a declared aperture" is one implementation.

Also removed: Zwirn's `attach_managed_slack`, `attach_managed_kubernetes` and
`submit_connect_credential` — a second copy of the connect flows, one of which is a weaker copy of a
security-critical check.

## How to tell whether it worked

- Adding a provider requires **no Rust**.
- `jira` can be tested without a database. Today it cannot be composed locally at all.
- Every outbound request passes through the egress port, so the destination aperture applies to all
  of them. Today four Integrations carry their own HTTP client and are not bounded by it.
- One credential store on the machine, not three.
- The same operation, invoked from the CLI and from Zwirn, takes the same code path.

## Sequence

S-035 CLI · S-036 OS keyring · S-037 datasource scaffolding into `service` · S-038 the generic
executor · S-039 the agent surface · S-040 Zwirn's library edge · S-041 the state port and SQLite ·
S-042 one composed local placement.

`integration-catalog` and the operations slice of S-038 exist and invoke live providers.
`connector-auth` is not written; OAuth2 acquisition is the largest remaining gap, and
[S-013](../stories/S-013-connect-session-oauth-custody-in-personal-posture.md) owns the one question
it cannot answer generically — where a callback lands on a machine with no public origin.
