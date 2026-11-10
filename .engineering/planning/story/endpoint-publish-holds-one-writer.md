---
format: aep.planning-md/3
id: story:endpoint-publish-holds-one-writer
kind: story
status: implemented
title: Concurrent connects with different endpoints on one instance publish at most one of them
summary: endpoint.rs publish() checks the file and then renames with no lock between; two writers can both go on (run 41742146209)
tags:
- from-finding
relations:
- informed_by: story:connect-names-a-failed-provider-start
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/src/local/endpoint.rs
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: docs/local-runtime-foundation.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:25Z", actor: "human:operator", revision: 4, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:25Z", actor: "human:operator", revision: 5, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-10T06:20:09Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":6,"verification":1}}, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

`apps/connectors/tests/adversary_d218_u2.rs:408` (`concurrent_endpoints_on_one_instance_agree_with_the_file`) failed
once on PR 237, repository gate run 41742146209, job 126286036566, 2026-11-04. Of 12 concurrent `connections connect
gitlab/ref-1 --endpoint https://forge-N.example.test`, 10 ended with `revision_conflict` at `publication`. Two went on
to `protected_entry`, forge-0 and forge-5, and the file held forge-5. The wave's diff does not touch the success path
(`git diff 413c60efc -- apps/connectors/src/local/endpoint.rs`: three error-path hunks only). 50 local runs at
`3689ae9dc` did not fail.

Inferred, not reproduced: `apps/connectors/src/local/endpoint.rs:492-495` (`publish`) compares the file with the text
it read, then calls `filesystem::publish_replace_keeping` with no lock between the check and the rename. Two writers
can both pass the check and both rename. Each reads back its own text before the other's rename lands, so both go on.
Found by wave D430 (D440).

## Acceptance

- At most one of several concurrent connects that write different endpoints into one instance goes on to its
  credential, and the file holds that one's endpoint: the check and the replacement are one step under a lock, or
  every writer that goes on re-reads the file after the last replacement.
- A test that widens the window between the check and the rename is red before the change and green after it.

## Scope

Landed on, from `git diff 5087593c1...impl/endpoint-publish-holds-one-writer` (merged into wave/zero-issues at 43135990d); the earlier scoper lines marked inferred are replaced by this list. Generated reference pages are left out.

- `apps/connectors/src/local/endpoint.rs`
- `contracts/cli/v1alpha1/semantics.md`
- `docs/local-runtime-foundation.md`
