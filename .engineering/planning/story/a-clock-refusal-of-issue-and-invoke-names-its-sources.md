---
format: aep.planning-md/3
id: story:a-clock-refusal-of-issue-and-invoke-names-its-sources
kind: story
status: draft
title: A clock refusal of approvals issue and invoke names its sources
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:standing-grant-binds-a-pattern-or-a-value-set
- depends_on: story:an-approved-write-survives-one-refused-clock-acquisition
revision: 1
---
## Defect

A clock refusal of `approvals issue` and of `invoke` is a bare `unavailable`: `issue` at
`stage` `readiness`, `invoke` at `stage` `admission`
(story:an-approved-write-survives-one-refused-clock-acquisition, "What happened"). The caller
cannot tell a clock refusal from another `unavailable`, and cannot see which source did not
answer.

- `Configuration::acquire()` drops the detail `acquire_explained()` has
  (`crates/connectors-host/src/local/clock.rs:377`): the combined `Refusal` and one
  `SourceReport` per source (`clock.rs:225-231`).
- `Error::clock` maps every refusal that is not `invalid_configuration` to `Code::Unavailable`
  with no member (`crates/connectors-host/src/local/owner.rs:261-272`).
- Only `approvals clock-check` builds `clock_refusal` and `clock_sources`
  (`apps/connectors/src/local.rs:549-570`).
- The CLI contract limits both members to `stage = observation`:
  `ess/domains/cli.yaml:946-953` and `contracts/cli/v1alpha1/semantics.md:123-130`.

## Change (draft)

1. A clock refusal of `approvals issue`, of the standing commands and of `invoke` carries
   `clock_refusal` and `clock_sources`, as `approvals clock-check` does. `stage` stays
   `readiness` and `admission`.
2. The presence rule of the contract becomes: on an approval clock's `unavailable` at `stage`
   `observation`, `readiness` or `admission`, when the quorum formed no answer.
3. Files the change reaches: `clock.rs`, `owner.rs` (`Error::clock` and a member beside
   `clock_format_retired`, `owner.rs:191-195`), `apps/connectors/src/local.rs`,
   `ess/domains/cli.yaml`, `contracts/cli/v1alpha1/semantics.md`,
   `contracts/cli/v1alpha1/fixtures/values.json`, `apps/connectors-cli-contract/binding.json`,
   the generated pages under `website/docs/reference/contracts`, `CHANGELOG.md`.

## Open before it is proposed

- The consumers that read the two members, named one by one (repository rule for a changed
  answer member): the callers' post script, the inbox bridge, the runbook program.
- `clock_sources` holds each source's numeric address. Through `invoke` it reaches a caller that
  has no access to the configuration file. Whether that is wanted is the operator's call.
- Acceptance statements: one test per command and stage, one statement per member.

## Ordering

After story:standing-grant-binds-a-pattern-or-a-value-set: both regenerate
`apps/connectors-cli-contract/binding.json`, the contract digest and the generated contract
pages, and both edit `contracts/cli/v1alpha1/semantics.md`. Lines added at
`semantics.md:123-130` move every later line that `ess/domains/cli.yaml` cites.
