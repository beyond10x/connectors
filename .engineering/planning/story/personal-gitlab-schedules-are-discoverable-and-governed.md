---
format: aep.planning-md/1
id: story:personal-gitlab-schedules-are-discoverable-and-governed
kind: story
status: draft
title: Personal GitLab schedules are discoverable and governed
summary: Close the CLI discovery and validation gaps, then add write-gated GitLab pipeline-schedule operations.
relations:
- derived_from: epic:local-product
- informed_by: story:explicit-target-never-implicit
revision: 1
---
## Defect

An end-to-end check of the personal-local GitLab path on 2026-09-04 showed that a healthy,
callable connection is not an operable schedule-management surface:

- `connectors doctor -o json` reported the personal posture healthy and `gitlab-user-get` plus
  `gitlab-project-list` both succeeded through the local socket, while `connectors connection list
  --query gitlab` returned no connections even though `operation search --query gitlab` named the
  admitted catalog-backed GitLab connection; the two discovery surfaces therefore disagree about
  the same live connection.
- `connectors operation search --query 'pipeline schedule'` returned no operation because
  `providers/gitlab.toml` declares pipeline inspection but no pipeline-schedule list, create,
  update, or delete operations, so an operator cannot configure the internal scheduled trigger
  needed for periodic Atlas reconciliation through Connectors.
- `connectors operation search --limit 50` was accepted by clap and then surfaced
  `connector-unreachable: Connector request was invalid: InvalidInput: search bounds are invalid`;
  the actual maximum is the undocumented `MAX_SEARCH_RESULTS = 25` in
  `crates/protocol/src/operation.rs`, while `crates/connectors-cli/src/lib.rs` declares only a
  default and no range, making caller input look like transport failure.

The separate implicit hosted-target defect is already governed by
`story:explicit-target-never-implicit`; this story is informed by it and does not duplicate its
target-selection scope.

## Scope

- Make a catalog-backed personal-local connection appear consistently in `connection list` and
  `operation search`, with tests proving the same opaque connection reference is reported by both.
- Add source-grounded GitLab pipeline-schedule list, create, update, and delete operations to the
  existing provider declaration, generated catalog artifacts, and consolidated catalog invariant
  coverage; keep the read useful to operators and curate model exposure separately from
  callability.
- Classify schedule mutations with truthful effects, risk, idempotency, OAuth/PAT scope, and
  approval requirements, and preserve the personal aperture: mutations remain absent or refused
  unless that configured GitLab placement explicitly enables writes.
- Put protocol search bounds into clap validation and help for every affected CLI search command,
  and render an out-of-range value as local invalid input before any socket or network request.
- Document and test the shortest personal-local workflow for finding the connection, describing
  the schedule operations, and invoking them after explicit write enablement.

## Out of scope

- Creating or changing a production GitLab pipeline schedule as part of implementation.
- Enabling writes in an operator's personal configuration.
- Reworking hosted-versus-local target selection, which belongs to
  `story:explicit-target-never-implicit`.

## Acceptance

With a healthy personal-local GitLab connection, the default local CLI reports one consistent connection across connection and operation discovery, documents and rejects search limits above the protocol maximum before transport, exposes source-grounded pipeline-schedule list/create/update/delete operations, and proves schedule mutations remain refused until writes are explicitly enabled and approval requirements are satisfied.
