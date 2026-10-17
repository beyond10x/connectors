---
format: aep.planning-md/3
id: story:openrouter-management-provider
kind: story
status: implemented
title: OpenRouter management keys read and created through the catalog provider, the created key held in custody
relations:
- serves: vision:independent-contract-adapters
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T06:55:07Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T06:55:07Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T02:29:03Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

An OpenRouter connection on a management key, through the catalog provider, reads the key's own
record, the account's workspaces and its API keys, and creates an API key whose one-time secret
never leaves Connectors: the created key goes straight into the existing credential custody as a
new connection, and the caller receives only that connection's reference and the key's hash. A
downstream gateway mints per-tenant inference keys without ever holding a key itself.

## Source

OpenRouter's OpenAPI document, <https://openrouter.ai/openapi.json>, retrieved 2026-10-16:
OpenAPI `3.1.0`, `info.version` `1.0.0`, licence MIT (`info.license`), 2,690,928 bytes. Pin it
under `adapters/openrouter/upstream/` with a source-hash record, as the Runpod source is pinned;
run the redaction check. Direct HTTPS to `https://openrouter.ai/api/v1`, like every other provider.

## Operations

| id | endpoint | effect |
|---|---|---|
| `key.get` | `GET /key` | read (the management key's own record; also the identity and access read) |
| `workspaces.list` | `GET /workspaces` | read |
| `keys.list` | `GET /keys` | read |
| `key.create` | `POST /keys` | write, required approval |

Nothing else is selected: no workspace creation, no key update or delete, no members or budgets.

## Write semantics

- `key.create`'s answer carries the new key's one-time secret (`key`). It is written into the
  existing custody as the credential of a new connection on an operator-configured target
  adapter and profile, and the invocation's output is that connection's reference and the key's
  `hash`, nothing else from the body that could carry the secret. The secret appears in no
  output, log, journal, evidence or error.
- A definite refusal (a documented 4xx) is `refused`. A timeout, a 5xx, a lost connection or a
  crash after the request was sent is `outcome_unknown`, and the create is never sent again for
  that invocation, by a retry, a restart or a recovery path.
- A created key whose custody write fails is reported by its hash with a distinct outcome, so the
  operator can delete it in OpenRouter; the secret is still not output.
- The management key is entered by the operator through the protected credential prompt, never
  in chat or argv. No Substrate dependency, no Kubernetes Secret delivery.

## Spec first

The shared ESS specification models the write outcome that carries a created credential into
custody (a generic member, not OpenRouter vocabulary) before any code; the specification
validates with the repository's pinned `ess` and its conformance suite runs green.

## Acceptance

- Reads: each of the three reads is invoked against a local fixture and answers the fixture's
  body; the connection's identity is the key record's subject.
- Create: against a fixture, `key.create` under approval returns a connection reference and a
  hash; the new connection is listed, and a read through it sends the created secret; no output,
  log or store file of the run contains the secret (searched for its bytes).
- Unknown outcome: a fixture that times out after receiving the request yields
  `outcome_unknown`, and the fixture records exactly one create across a retry and a restart.
- Custody failure: a forced custody write failure reports the hash and the distinct outcome, and
  the secret appears nowhere.
- Docs: `docs/catalog-openrouter.md` shows the configuration, the protected-prompt connect and
  each operation from a run in the tree.

## Status

- Reads: the pinned OpenRouter source (redaction rule `upstream-redaction/3`), the bundle,
  `key.get`, `workspaces.list` and `keys.list` with their fixture tests, and
  `docs/catalog-openrouter.md`.
- `key.create` under the current operator setting `[adapters.<alias>.permissions.custody]` (the
  connector-vocabulary migration reads it as a legacy name): the one-time key goes into custody
  as a new connection; the output is the connection ref and the key hash. `custody` and
  `created` are members of `connectors.cli.Failure`; the CLI contract is regenerated.
- Security review of the write, 2026-10-16: two must-fix findings, both fixed. A create whose
  operation declares custody with no configured target (and a target for an operation that
  creates nothing) is refused `invalid_configuration`, `not_attempted`, before it is sent
  (modelled in `ess/domains/mutations.yaml`). The owner and every adapter child set
  `RLIMIT_CORE` 0 and, on Linux, `PR_SET_DUMPABLE` 0. Checked and not faulted: the config
  refusals, the pool worker deadlock, the body overwrite and `Secret` wipe, the child's send.
- The ignored custody CLI journey cases were red locally; the guarded_merge journey with no
  custody failed the same way on `main`, from a test helper that missed the audit anchor's
  `access` (fixed in the test, aa1ee3437) and an intermittent `outcome_unknown` whose owner
  error is not captured (story:owner-errors-are-readable). These journeys run in no CI job
  (story:ignored-journeys-run-in-ci).
- Risks accepted for this release, each stated in the release notes:
  - a custody store that answers `outcome_unknown` is reported as custody `failed`, though the
    connection may have been published;
  - the HTTP client's serde_json buffers that held the answer are not wiped;
  - an owner crash between the provider's applied answer and the settle recovers the attempt as
    unknown with the key hash recorded nowhere; nothing is re-sent.
