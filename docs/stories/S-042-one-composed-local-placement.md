---
id: S-042
title: "One composed local placement, called by both the CLI and Zwirn"
pillar: Platform
status: backlog
priority: 3
design: ../design/12-one-owner-for-every-outside-connection.md
epic: local-product
areas: [runtime, cli, zwirn]
note: "depends on S-041; splits compose from bind so the CLI one-shot, `connectors serve` and Zwirn's local placement share one entry point"
---

# One composed local placement, called by both the CLI and Zwirn

## Goal

One function that composes a local Connector placement from configuration, called by everything that
needs one — a one-shot CLI command, `connectors serve`, and Zwirn's local placement — so there is
exactly one composition path and exactly one owner of auth, secrets, token state and dispatch.

Timo, 2026-08-20: *"since connectors-cli AND zwirn both need a 'local' variant of those seams, we
should put the main composed thing in a shared place inside connectors. Then cli just calls the
constructor with config, and zwirn does the same thing."*

## Why

`PersonalRuntime::bind` already composes every configured Integration. It also binds a Unix socket,
which is why neither of the other two callers can use it:

- a one-shot `connectors operation search` wants the composed registry in-process, with no daemon;
- Zwirn's local placement wants the same, in its own process;
- only `connectors serve` wants the socket.

So the seam is one split serving three callers:

```text
compose(config, state_root, ports) -> BackendRegistry      ← CLI one-shot, and Zwirn in-process
bind(registry, socket_path)        -> LocalOperationDaemon ← `connectors serve` only
```

[S-035](S-035-the-cli-runs-without-being-configured-by-hand.md) already needs this split for its
in-process one-shot. The discovery is that it is not a CLI convenience: it is the shared entry point,
and splitting it once serves all three.

## Approach

- `Ports { secrets: Arc<dyn SecretStore>, state: Arc<dyn StateStore>, egress: Arc<dyn EgressTransport> }`.
  Every Integration takes one constructor with no posture in its signature. Hosted binds Vault plus
  PostgreSQL; local binds the OS keyring plus SQLite; a satellite binds whatever it has.
  `integration-catalog` is already written this way, which is why wiring it into the hosted arm is
  one block rather than a port.
- The two hand-written composition ladders in `connectors-runtime/src/composition.rs` collapse into
  one list driven by configuration.
- Zwirn links `connectors-runtime` and `connectors-console` from its own crates
  (`products/zwirn/crates/agent-app`), which is the permitted `products ──▶ connectors` direction.
  **Never from `runtime/agent`** — `architecture/dependency-rules.md:45-46` bars agent from
  compiling connectors implementation, and Zwirn embeds agent.
- Record the edge as a build dependency in an ADR. The arrow is already permitted; what needs
  writing down is that this edge compiles source rather than speaking a protocol.

## Acceptance

- [ ] `compose` and `bind` are separate, and `connectors serve` is the only caller of `bind`.
- [ ] A one-shot CLI command answers with no daemon running.
- [ ] Zwirn's local placement is a call to `compose`, not a spawned process plus reimplemented flows.
- [ ] `attach_managed_slack`, `attach_managed_kubernetes` and `submit_connect_credential` are deleted
      from `products/zwirn/crates/agent-app/src/connectors.rs`.
- [ ] A declared instance still materialises at open without a human typing a token, so a restart
      costs no hands. This property came from the concurrent session's Slack work and must survive.
- [ ] An ADR records the build-dependency edge.

## Depends on

[S-041](S-041-state-becomes-a-port.md), because a uniform `Ports` struct needs state to be a port.
