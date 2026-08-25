---
id: S-074
title: "The CLI drives a hosted connection"
pillar: Clients
status: backlog
priority:
design: ../design/16-subscription-credential-custody.md
epic: subscription-custody
areas: [connectors-cli, connectors-console, connectors-client]
note: "HostedClient exists but no connectors verb reaches it. Also: the CLI is at exactly its 856-line architecture-fence cap, so any new arm needs the cap raised with a dated justification."
---

# The CLI drives a hosted connection

## Goal

Let the same `connectors connect` a person runs on their laptop drive a deployment, so the web UI
is a convenience rather than the only way to connect and the flow is testable without a browser.

## Acceptance

- [ ] A hosted target is reachable from the `connectors` verbs. `HostedClient`
      (`crates/connectors-client/src/lib.rs:459-543`) already speaks operation, connection and
      event; today no verb constructs it.
- [ ] `connectors connect anthropic` and `connectors connect claude-code` work against a deployment
      and against a local state root, by the same code path.
- [ ] Credential input keeps the existing discipline: hidden `rpassword` prompt, or
      `--credential-file` for scripting. **No stdin path and no argument** — `connect` deliberately
      has neither, because an argument lands in `ps` output and shell history
      (`crates/connectors-console/src/enrol.rs:59-64`).
- [ ] `connectors auth status` reports both providers' presence from `SecretStore::exists` over
      declared credentials, without enumerating the store, and remains structurally value-free.
- [ ] Behaviour lives in `connectors-console`; the CLI crate gains clap arms and nothing else.
- [ ] `CLI_TOTAL_LINE_LIMIT` at `crates/catalog-build/tests/main/architecture_fence.rs:24` is raised
      with a dated justification comment in the style of the three prior raises, stating what moved
      and why it is packaging rather than behaviour. The pinned dependency list
      (`architecture_fence.rs:299-321`) does not grow.

## Progress
- (not started)

## Notes

- The CLI is at **exactly** 856 of 856 lines (`src/main.rs` 8 + `src/lib.rs` 848). Adding one line
  fails the gate, so the cap raise is part of this story rather than a surprise during it.
- Parser tests stay in-crate (`crates/connectors-cli/src/lib.rs:776-848`, `Cli::try_parse_from`);
  behaviour tests go one layer down in `connectors-console`, matching the existing split.
- Argv is parsed with clap's derive API. Hand-rolled parsing is banned repo-wide.
