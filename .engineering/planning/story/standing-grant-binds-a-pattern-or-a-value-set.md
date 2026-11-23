---
format: aep.planning-md/3
id: story:standing-grant-binds-a-pattern-or-a-value-set
kind: story
status: draft
title: A standing grant binds a target member to a pattern or a value set
relations:
- depends_on: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
- informed_by: architecture-decision-record:standing-scoped-approval
revision: 2
---
## Outcome

A standing grant may bind a target member to a pattern or to a list of values, so one
long-lived grant per operation covers a consumer whose target changes on every call. For
release 0.78.0, after the exact-value grants of 0.77.0.

## Decision

D11, Timo's pick of 2026-11-23 about 13:25 CET, relayed by the operator: "A now, C next
(Recommended)". A: 0.77.0 keeps exact-value grants. C: a target member may be a pattern
(`/issueIdOrKey` matching `^DEV-[0-9]+$`) plus a value set (`/target_status`), one long-lived
grant per operation with `max_per_hour` and `not_after`, changing the accepted record
architecture-decision-record:standing-scoped-approval. The operator: "Timo's yes for the record
change is this pick."

The record today (Decision, point 1): "every target member the grant names (a JSON pointer into
the operation's input and its exact value, such as `/owner` and `/repo`) is present in the
input with that value". This story revises that point in both placements (served and local).

## Consumer

The Jira step of release-review (dev-operator), facts from its session on 2026-11-23, read from
`bindings/tools/release-stage.sh:790-870` of its own repository and
`~/beyond10x/connectors-definitions/connectors/jira.yaml:54-68`; not run, no route yet:

- Writes per DEV ticket of a revision: `issue.transition.run` `{issueIdOrKey, current_status,
  target_status, body.transition.id}`, one call per hop, 1 to 5 hops per ticket; and
  `issue.comment.add` `{issueIdOrKey, body: ADF document}`, one per ticket.
- No input member is the same between calls. The ticket set differs every week.
- `issueIdOrKey` matches `^DEV-[0-9]+$`.
- `target_status` and `current_status` carry status ids. The connector checks them
  (`jira.yaml:54-68`): `guard.before` `/fields/status/id == current_status`; `guard.also`
  the transition named by `body.transition.id` leads to `target_status` and is available;
  `guard.after` the issue read back has `/fields/status/id == target_status`.
- Status ids in project DEV, read by that session about 13:50 CET from 100 DEV issues of the
  last 90 days: 3 In Progress, 10234 Review, 10223 QA, 10001 Done, 10000 To Do, 10235
  Preparation; also seen and never a target: 10002 Backlog, 6 Abandoned. Paths pass through
  To Do and Preparation, so the target set is 10000, 10235, 3, 10234, 10223, 10001.
- Writes per run, from 64 prod-eu `release.json` files: tickets min 0, median 23, max 80; at
  most 6 writes per ticket; worst case 480, median case 138. `max_per_hour` 500 covers it.
- Every comment starts with `Release review of ` (`release-stage.sh:815`); the consumer wants
  that prefix bound. The text sits inside an ADF document, so the pointer goes into nested
  arrays.

Under 0.77.0 the step runs with one exact-value grant per ticket and operation, issued before
the run by a person or the operator session (up to 160 grants at 80 tickets; the file holds
256).

## Open, for planning

- The pattern language: a full regular expression, or a closed form (literal prefix plus a
  character class and length bound). A copied grant file then allows every write the pattern
  matches until `not_after`, up to `max_per_hour`.
- Whether a pattern may bind a member inside an array (the comment's prefix); the closed-input
  story treats arrays as leaves.
- How `input_keys` (story:standing-grant-closes-the-input-to-listed-members) and patterns
  combine.
- The ESS model (`ess/domains/standing_approvals.yaml`), the contract text of both placements,
  the grant file format version, and whether `grant-standing` asks for a second confirmation
  for a pattern grant (O6 of story:local-invoke-admits-a-keyed-write-under-a-standing-grant).

## Acceptance

Not written. Planned after 0.77.0 is merged; estimate for 0.78.0, not a measurement:
2026-11-24 about 18:00 CET.
