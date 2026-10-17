---
format: aep.planning-md/3
id: story:ess-connector-vocabulary-model
kind: story
status: implemented
title: The ESS model declares Provider, Connector and the instance's connector, and the Stale connection state
relations:
- decomposes: epic:connector-vocabulary
- informed_by: architecture-decision-record:connector-vocabulary
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: inferred
  path: ess/domains/auth_bindings.yaml
- confidence: inferred
  path: ess/domains/cli.yaml
- confidence: inferred
  path: ess/domains/connection_admission.yaml
- confidence: inferred
  path: ess/domains/declarations.yaml
- confidence: inferred
  path: website/docs/reference/contracts/model
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T11:55:34Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-16T11:55:34Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-17T01:36:44Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

The epic promises that the ESS model uses the vocabulary (`epic:connector-vocabulary` § Outcome),
and review-result:connector-vocabulary-scope-r1 found that no story owned it. This story owns the
ESS model change for the whole epic; every story that reads a new noun or the `Stale` lifecycle
depends on it.

Two ess:author drafts exist, uncommitted, on branch `design/connector-vocabulary`:

- the nouns (Provider, Connector, and the instance as `connectors.declarations.ServiceConfiguration`
  gaining its connector and name) in this tree, in `ess/domains/declarations.yaml`, with vocabulary
  comments in `ess/domains/auth_bindings.yaml` and `ess/domains/cli.yaml`;
- the `Stale` state of `connectors.auth_bindings.Connection` in worktree `wt-446f3263bd2a`.

The wave implementor of this story verifies both drafts against `docs/design/connector-vocabulary.md`
and `architecture-decision-record:connector-vocabulary`, merges them, finishes what they leave
UNMAPPED where the design or the ADR answers it, and leaves the rest UNMAPPED with its question.

## Change

- `ess/domains/declarations.yaml`: `connectors.declarations.Provider` and
  `connectors.declarations.Connector`, and the instance (`ServiceConfiguration`) referencing its
  connector, as the drafts declare them.
- `ess/domains/auth_bindings.yaml`: `connectors.auth_bindings.Connection` lifecycle states
  `Live`, `Stale`, `Revoked`, with the transitions of design § Connection lifecycle: `Live -> Stale`
  on a provider refusal of the stored credential, `Stale -> Live` and `Live -> Live` on publication,
  `Live, Stale -> Revoked` on revoke. `Revoked` stays terminal.
- How `connections list` projects `Stale`: if it needs a variant of
  `connectors.connection_admission.ConnectionState` (`ess/domains/connection_admission.yaml`), this
  story adds it, so story:connection-stale-state edits no ESS file.
- `ess/domains/cli.yaml`: the header vocabulary comment only. Field changes to CLI types belong to
  story:cli-connector-instance-selectors, story:connections-reconnect-verb and
  story:setup-migrate-adapter-entries.
- Regenerated outputs: `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json`
  (`crates/connectors-build/src/metadata_entities.rs`) and the website model pages
  (`cargo run --locked -p connectors-docs -- generate`).
- Instance-connection cardinality is not modelled beyond today's; see
  `decision-blocker:instance-connection-cardinality`.

## Domain relations

This story is where the new relations are declared. In the nouns draft: Connector -> Provider is
`connectors.declarations.Connector` relation `provider` (references, cardinality one, via
`provider_id`); instance -> Connector is `connectors.declarations.ServiceConfiguration` relation
`connector` (references, cardinality one, via `connector_id`). Connector -> AuthProfile and
Connector -> Operation are UNMAPPED markers in the draft and stay so unless the design or ADR
answers them. Existing relations kept: `connectors.auth_bindings.Connection` relations `instance`
and `auth_profile`; `connectors.declarations.ServiceConfiguration` relation `adapter`.

## Scope

- `ess/domains/declarations.yaml`
- `ess/domains/auth_bindings.yaml`
- `ess/domains/connection_admission.yaml` (only if the `Stale` projection needs a variant)
- `ess/domains/cli.yaml` (header comment only)
- `crates/connectors-host/src/local/metadata/entity-runtime-definitions.json` (regenerated)
- `website/docs/reference/contracts/model/` (regenerated)

## Acceptance

For each of `connectors.declarations.Provider`, `connectors.declarations.Connector`,
`connectors.declarations.ServiceConfiguration` and `connectors.auth_bindings.Connection`,
`ess specify inspect --path ess --format json <name>` prints the declaration, and together they
show the entities `Provider` and `Connector`, the relations `Connector.provider` and
`ServiceConfiguration.connector`, and `Connection` lifecycle states `Live`, `Stale`, `Revoked` with
transitions `Live -> Stale`, `Stale -> Live`, `Live -> Live` and `Live, Stale -> Revoked`; on
`main` before this story the first two names are not found and `Connection` has no `Stale`.

## Notes

- The OpenRouter custody unit (adds `[adapters.<alias>.permissions.custody]` to
  `crates/connectors-host/src/local/config.rs` and touches `ess/domains/cli.yaml`) lands on `main`
  first: rebase this branch on it before branching the wave, and regenerate
  `entity-runtime-definitions.json` after the rebase, not before.
- `ess specify validate --path ess` stays green throughout; it is a gate, not the acceptance, because
  it also passes on today's model.

## Decisions

## Decisions

Answers to every `UNMAPPED:` marker the two drafts left in `ess/domains/declarations.yaml`,
`ess/domains/auth_bindings.yaml` and `ess/domains/cli.yaml`. Taken by the coordinator on
2026-10-16 from the design, the accepted ADR and `decision-blocker:instance-connection-cardinality`;
the implementor writes them into the model and removes the markers. The two markers that predate
this epic (`ess/domains/cli.yaml`, `ess/domains/feed.yaml` at `74d8d857d9`) are not in scope.

Nouns (`declarations.yaml`):

1. Provider is a declaration shipped with its connectors (the catalog), never a configuration key.
2. `provider_id` and `connector_id` follow `connectors_core::valid_id`
   (`crates/connectors-core/src/lib.rs:360-366`: ASCII alphanumerics and `-_.`, at most 128 bytes),
   lowercase by convention (`google`, `gitlab`; `gmail`, `drive`, `gitlab`).
3. `connector_id` is the short user-facing name (`gmail`). A catalog connector also carries
   `catalog_provider` (today's `provider` value, `google-gmail`), which is what enters the
   configuration revision digest (`adapters/catalog/src/local.rs:929-960`), so migrated revisions
   are unchanged.
4. Connector references the implementation it runs on: `AdapterSpecification` by `adapter_id`,
   cardinality one; one implementation serves many connectors (`catalog` serves every catalog
   connector).
5. Auth profiles and operations stay declared on the implementation and its selection; Connector
   names the auth profiles it accepts (`profile_ids: List<String>`, references the profile
   declarations) and the selection its operations come from (`selection: Optional<String>`, the
   operations file name for a catalog connector). Every operation of that selection is available
   (ADR decision 5).
6. Every declared connector exists as a record whether or not it is enabled; `enabled` is set from
   configuration.
7. `default_endpoint` is Optional. An instance whose connector has none must state `endpoint`;
   recording such an instance without one is refused (an outcome on `RecordServiceConfiguration`).
8. `connector_id` on an instance stays Optional during the deprecation window (legacy
   `[adapters.*]` entries have none); it becomes required in the release that ends the window
   (migration step 9), which is a later change, not this story.
9. (`connector_id`, `name`) is unique among instances: a second instance with both equal is refused.
10. `RecordServiceConfiguration` carries the new instance fields (`connector_id`, `name`,
    `endpoint`, `description`). A new instance's `instance_id` is `<connector>.<instance>`
    (`gmail.private`); a migrated instance keeps its old `instance_id`.

Connections (`auth_bindings.yaml`):

11. At most one non-revoked connection per instance (cleared blocker, option A):
    `AllocateConnection` is refused while the instance holds a `Live` or `Stale` connection. The
    host's bound of 1000 connections per `instance_id` (`registry/lifecycle.rs:20-22`) stays as
    the storage bound for revoked history.
12. `contracts/auth/connection/v1alpha1/semantics.md` §4.1-§4.2 are amended in this story to name
    `Stale` and its transitions; the contract and the model say the same thing.
13. Host code that calls `MarkConnectionStale` is `story:connection-stale-state`, not this story;
    the marker is replaced by a note naming that story.
14. Any `PublishBinding` leaves `Stale`: a publication means the provider accepted a credential
    (reconnect, or a revalidation the provider now accepts).
15. Every credential the provider refuses as unauthorized makes a connection `Stale`, for every
    profile scheme (OAuth refresh token, personal access token, basic, bearer).
16. A refused cached OAuth access token alone does not: the provider drops it and refreshes once;
    only a refused refresh makes the connection `Stale`.
17. Revalidation outcomes: `invalid` (provider refusal), `insufficient` (scope below the minimum)
    and `missing` (lost keyring entry) make the connection `Stale`, because reconnect is the fix
    for each. Another identity on a repair or reconnect is refused and leaves the connection as it
    was.

Scenarios:

18. Every new command outcome and guard gets a synthesized scenario; where synthesis cannot
    arrange one (it names the refusal), an authored scenario (`ess verify conform author`) covers
    it: the cardinality refusal (11), the missing-endpoint refusal (7), the duplicate
    (`connector_id`, `name`) refusal (9), `Live → Stale → Live` through `MarkConnectionStale` and
    `PublishBinding` (14), and `MarkConnectionStale` on `Revoked`.

## Obligations

## Obligations

Checked by the coordinator before the unit merges, beside the acceptance:

- `git grep -c UNMAPPED -- ess/domains/declarations.yaml ess/domains/auth_bindings.yaml` prints
  nothing (zero markers), and `ess/domains/cli.yaml` holds only the one marker it held at
  `74d8d857d9`.
- `ess verify conform synthesize --path ess` (pinned `ess`, `crates/connectors-spec/toolchain.json`)
  reports more than 501 scenarios, and the suite holds one scenario for each case named in
  Decisions 18; each authored scenario passes `ess verify conform author`.
- `cargo run --locked -p connectors-build -- metadata-entities --check` and
  `cargo run --locked -p connectors-docs -- generate --check` exit 0 after regeneration.
