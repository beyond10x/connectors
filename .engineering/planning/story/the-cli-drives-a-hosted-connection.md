---
format: aep.planning-md/1
id: story:the-cli-drives-a-hosted-connection
kind: story
status: draft
title: The CLI drives a hosted connection
refs:
- provider: legacy
  reference: S-074
relations:
- derived_from: epic:subscription-custody
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-074-the-cli-drives-a-hosted-connection.md:20`. **read**

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

## Context

Let the same `connectors connect` a person runs on their laptop drive a deployment, so the web UI
is a convenience rather than the only way to connect and the flow is testable without a browser.

Source frontmatter: pillar Clients · areas [connectors-cli, connectors-console, connectors-client] · design `../design/16-subscription-credential-custody.md`. **read**

Source `note:` field, quoted: “HostedClient exists but no connectors verb reaches it. Also: the CLI is at exactly its 856-line architecture-fence cap, so any new arm needs the cap raised with a dated justification.”

## Status

`backlog` in the source. Quoted from `docs/stories/S-074-the-cli-drives-a-hosted-connection.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-074-the-cli-drives-a-hosted-connection.md`, which is not deleted and now names this artifact.

- First written 2026-08-25 · last touched 2026-08-25 · 1 revision(s)
- Legacy id `S-074`, recorded as the reference `legacy:S-074`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
