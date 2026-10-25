---
format: aep.planning-md/3
id: story:oauth2-refresh-metadata-authority
kind: story
status: active
title: The metadata authority owns the refresh ledger and the OAuth2 domain
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-build/src/metadata_entities.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: cited
  path: ess/components.yaml
- confidence: inferred
  path: website/docs/reference/contracts/model
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:54Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:54Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

The local metadata authority owns `connectors.refresh` and `connectors.oauth2`: their entities are lowered
into the host's committed Entity Runtime definitions, and the synthesized `local-metadata-authority`
conformance suite executes and passes their scenarios. The host opens an existing metadata store with the
new definitions. No host code calls the new commands yet.

## Model

- `ess/domains/refresh.yaml`: `RefreshAttempt` and its 8 commands (`ReserveRefresh`, `AuthorizeExchange`,
  `FenceReserved`, `StoreResponse`, `QuarantineRefresh`, `RecoverStoredResponse`, `PublishRefresh`,
  `DiscardCandidate`).
- `ess/domains/oauth2.yaml`: `AuthorizationServer`, `AccessTokenIssue`, `DeclareAuthorizationServer`,
  `IssueAccessToken`.

## Design (probed 2026-10-25 on d2842557d)

- `ess/components.yaml`: add both domains to `local-metadata-authority.owns.domains` and the 10 commands
  to `accepts.commands`.
- `crates/connectors-build/src/metadata_entities.rs` `DEFINITIONS`: add
  `connectors.refresh.RefreshAttempt`, `connectors.oauth2.AuthorizationServer`,
  `connectors.oauth2.AccessTokenIssue`. Without them the lowering refuses with `MissingDefinitionVersion`
  for each of the three; with them `cargo run --locked -p connectors-build -- metadata-entities`
  regenerates `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json` with no
  diagnostic (probe, reverted).
- The host registers the committed definitions (`crates/connectors-build/src/metadata_conformance.rs:57`
  mirrors `registry()`); a store recorded before this change must still open.
- Regenerate the model reference (`cargo run --locked -p connectors-docs -- generate`).
- `story:spec-models-operation-admission` (draft) also edits `DEFINITIONS` and regenerates the same
  definitions file; whichever lands second regenerates the file on top of the first, never merges it by
  hand.
- Any construct the pinned ESS 0.68.0 lowering or synthesis refuses is reported as an ESS gap, with the
  workaround marked in the source with the ESS version; the `ESS-LIMIT` notes in `refresh.yaml` stay true.

## Acceptance

- `cargo run --locked -p connectors-build -- metadata-entities --check` exits 0 on the merge.
- The gate's metadata conformance step (`crates/connectors-build/src/gate.rs:312-314`, synthesized by
  `metadata_conformance::gate`) runs and passes the scenarios of every outcome of the 10 commands above,
  among them `ReserveRefresh` `reserved` and `source-unavailable`, `AuthorizeExchange` `authorized` and
  `wrong-state`, `QuarantineRefresh` `uncertain`, `PublishRefresh` `published` and
  `publication-refused`, `IssueAccessToken` `issued-current`, `issued-refreshed` and
  `reauthorization-required`. The executed scenario count before and after is in the report, and it rises.
- `ess verify conform mutate` over the component (the gate's emission, `metadata_conformance::emit`)
  leaves no surviving mutant whose site lies in `connectors.refresh` or `connectors.oauth2`. If the pinned
  ESS cannot reach a site, the story stays active and that site is filed as an ESS gap; a named survivor
  does not pass.
- The existing fixture store test (`crates/connectors-host/tests/fixtures/metadata-store-er-0.26.0`)
  passes unchanged.
- `cargo run --locked -p connectors-docs -- generate --check` exits 0.

## Out of scope

Any host call of these commands (`story:owner-rotating-refresh`, `story:connect-runs-an-oauth2-grant`,
`story:connections-token-issues-an-access-token`); the authored refresh scenarios under
`contracts/auth/acquisition/v1alpha1/scenarios/`, which stay on the gate's known-failing list
(`gate.rs:484-496`) until the owner implements them.
