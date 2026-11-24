---
format: aep.planning-md/3
id: story:owner-observe-request-is-modelled-in-ess
kind: story
status: draft
title: The owner request observe has an ESS type
relations:
- decomposes: epic:review-20261123-findings
- serves: vision:independent-contract-adapters
- depends_on: story:connections-list-answers-through-the-owner
scope:
- confidence: inferred
  path: contracts/cli/v1alpha1/owner.md
- confidence: inferred
  path: crates/connectors-build/generated/metadata-emissions
- confidence: inferred
  path: crates/connectors-core/generated/operation-types
- confidence: inferred
  path: crates/connectors-host/src/local/metadata/entity-runtime-definitions.json
- confidence: inferred
  path: ess/domains/cli.yaml
- confidence: inferred
  path: website/docs/reference/contracts/model
revision: 7
---
## Outcome

The owner request `observe` has a type in `ess/domains/cli.yaml` beside `LocalOwnerConnections`,
and `contracts/cli/v1alpha1/owner.md` names that type where it describes the request.

## Source

story:connections-list-answers-through-the-owner adds the request in Rust and describes it in
`owner.md` in prose. Its critics (design and scope, round 1, 2026-11-23) moved the model out of
it: nothing in code or tests reads the ESS type of an owner request (`Request` is hand-written
serde, `crates/connectors-host/src/local/owner.rs:1307-1445`), 5 of about 14 request kinds are
modelled (`ess/domains/cli.yaml:232-339`), and the precedent c58fdb5f82f modelled `connections`
in a commit of its own after the request had landed.

## Acceptance

1. `ess/domains/cli.yaml` declares the request and its answer; `ess specify validate` exits 0.
2. `owner.md` names the two types in the `observe` paragraph (`git grep` of each name).
3. The drift checks of `AGENTS.md:149-153` and `cargo test -p connectors-build --tests` exit 0
   with the regenerated files committed: operation types, entity-runtime definitions, the
   contract model pages and the metadata emissions.

## Shares files with

- `crates/connectors-build/generated/metadata-emissions/inputs.json`:
  story:entity-runtime-0330-records-name-definition-by-digest changes its `digest` line through
  `Cargo.lock`. This story regenerates on a base that holds the pin.
- `ess/domains/cli.yaml`: story:timeout-refusal-says-how-long-it-waited,
  story:definition-files-in-user-config and other drafts. Stories that edit `ess/**` do not run
  in one wave: each regenerates the same files.

## Out of scope

The nine owner request kinds that have no ESS type today.
