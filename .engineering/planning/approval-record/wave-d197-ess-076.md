---
format: aep.planning-md/3
id: approval-record:wave-d197-ess-076
kind: approval-record
status: approved
title: 'Wave D197: the five named stories are done or blocked; the ESS pin move is withdrawn'
relations:
- decides: story:jira-feed-binding
- decides: story:confluence-feed-binding
- decides: story:confluence-token-kind-binding
- decides: story:ci-timing-flakes-20261018
- decides: story:sustained-live-read-loop
revision: 5
transitions:
- {from: "draft", to: "approved", at: "2026-10-29T10:28:35Z", actor: "human:timo", revision: 3}
---
## Approval

The directing session `b10x-chef-orchestrator` approved item D197 on 2026-10-29, under its grant: "one connectors wave of at most 4 units, chosen from the accepted or active stories in .engineering/planning/story/: ci-timing-flakes-20261018, confluence-token-kind-binding, confluence-feed-binding, jira-feed-binding, sustained-live-read-loop. Re-check each against origin/main first. Drop and record in the store any that is already done or blocked. Also drop, recording why, any whose acceptance needs live provider credentials or a live tenant you don't have; never create, request or touch credentials." Ride-along: "if connectors pins ESS, move it to the newest release."

## Selection, checked against `origin/main` `f4df1c4f8`

| story | status on main | decision | reason |
|---|---|---|---|
| story:jira-feed-binding | implemented | dropped | done: shipped in 0.56.0 (wave 20261021), verified by approval-record:wave-20261023-closeout |
| story:confluence-feed-binding | implemented | dropped | done: shipped in 0.56.0, verified by approval-record:wave-20261023-closeout |
| story:confluence-token-kind-binding | implemented | dropped | done: merged in wave 20261021 (`7e22c9e65`), verified by approval-record:wave-20261023-closeout |
| story:ci-timing-flakes-20261018 | implemented | dropped | done: 30 macOS and 31 Linux CI runs with 0 failures recorded 2026-10-23 |
| story:sustained-live-read-loop | active | dropped | blocked by credential-blocker:live-read-loop-no-configured-connections; its acceptance is a 10-minute run against live Slack, Jira, GitLab, Kubernetes and SQL connections, which this wave does not have and may not create |

## ESS pin withdrawn (D198)

The ESS 0.75.0 to 0.76.0 move was built and gated in a tree but not committed when the same session sent D198 on 2026-10-29: "do not move the ESS pin in the connectors wave unless already committed (then leave it)", quoting the operator: "I don't see a point in upgrading all our libraries every time a new ESS version is there ... maybe do it once when all GitHub issues are solved". The move was discarded. The pin stays at ESS 0.75.0.

## Result

No unit, no pin move, no pull request of its own and no release. The full repository gate run on the discarded pin move failed in one test that also fails on `main` (2 of 5 runs); it is recorded as story:local-cli-capacity-probe-flake. This record and that story ride the repository's next pull request.
