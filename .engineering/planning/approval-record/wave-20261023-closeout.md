---
format: aep.planning-md/3
id: approval-record:wave-20261023-closeout
kind: approval-record
status: approved
title: 'Wave 20261023 closeout: verify and move the merged feed, token-kind and flake stories'
relations:
- decides: story:jira-feed-binding
- decides: story:confluence-feed-binding
- decides: story:confluence-token-kind-binding
- decides: story:ci-timing-flakes-20261018
- decides: story:list-tests-copied-cli-busy
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-10-23T21:46:15Z", actor: "human:timo", revision: 3}
---
## Stop

The directing session `b10x-chef-orchestrator` approved on 2026-10-23, under its grant: "Merge #207 when its 8 checks are green. Then one wave over the active stories that no open blocker in your store blocks: story:ci-timing-flakes-20261018, story:jira-feed-binding, story:confluence-feed-binding, story:confluence-token-kind-binding (open-work.sh connectors). Add story:sustained-live-read-loop only if it runs without a live credential. Skip any story that needs wave C's custody or MCP decision. When ESS 0.69.0 ships (estimated 2026-10-24 ~03:00Z), move the pin to it."

## Finding at selection

The three feed and token-kind stories were built and merged by wave 20261021 (approval-record:wave-20261021, units U2, U4 and U5) and shipped in 0.56.0; their feeds reached the shipped selections in 0.57.0. Neither release plan moved them, so they stayed `active`. story:ci-timing-flakes-20261018 had only its CI-run acceptance left. This wave therefore writes no product code: it checks each acceptance against `main` `0336f1381` and records the result.

## Units

| unit | story | work |
|---|---|---|
| V1 | story:jira-feed-binding | run its acceptance tests on `main`; record them |
| V2 | story:confluence-feed-binding | the same |
| V3 | story:confluence-token-kind-binding | the same, including the ignored CLI journey through the `disposable` family |
| V4 | story:ci-timing-flakes-20261018 | count the CI runs after the change from their logs |
| V5 | story:list-tests-copied-cli-busy | put the list tests' executables in place through a `cp` child; 200 runs of each target under load |

V5 was added on 2026-10-23 by a second approval of the directing session, under its grant: "A: put story:list-tests-copied-cli-busy into the current wave, approved under the grant. It is test-only and is done at 200/200 under load. The four stories: do the verification and the store moves, and cite the CI runs (30 macOS, 31 Linux, 0 failures) in each move. Record this in your store."

The same session relayed the operator's answer "A A" (2026-10-23) to decision-blocker:custody-file-delivery-widens-custody and decision-blocker:mcp-tool-call-without-revision; this wave records both decisions and clears the two blockers. Their stories go to wave C.

## Left out

| story | reason |
|---|---|
| story:sustained-live-read-loop | its acceptance is a 10-minute run against live Slack, Jira, GitLab, Kubernetes and SQL connections; it does not run without a live credential |

## Results

| unit | result | move |
|---|---|---|
| V1 | `cargo test --locked -p connectors-catalog-provider --test jira_feed` on `0336f1381`: 14 passed, 0 failed; feed conformance in `connectors-build` (ESS 0.67.0): 24 passed | active → implemented |
| V2 | `--test confluence_v1_feed`: 14 passed, 0 failed; the same 24 conformance tests | active → implemented |
| V3 | `--test confluence_binding`: 12 passed, 0 failed, 1 ignored; the ignored CLI journey passed in `connectors-build ignored --family disposable` | active → implemented |
| V4 | macOS "host and CLI" 30 runs, Linux repository gate 31 runs after `9201b1177`, 0 failures of the four tests; run ids in the story | active → implemented |
| V5 | under 20 busy cores: `adversary_list_connectors` 200 of 200, `list_connectors` 200 of 200 | active → implemented |

Each of V1–V4 cites the CI runs in its test_result evidence.

Not evidenced: story:ci-timing-flakes-20261018 carries from story:oauth-drain-macos-race an acceptance of 500 consecutive repetitions of the two oauth tests on the macOS runner. No record of such a run exists; the story's own list of what remained named only the CI runs. The 30 macOS CI runs passed both oauth tests each time.

Observed, outside this wave: the same `disposable` run reported 10 failed tests, 8 in `connectors-catalog-provider` `local_runtime::cli_journey` and 2 in `connectors-host` (`final_fence::withdrawal_at_final_dispatch_prevents_real_child_invocation`, `launch::tests::disposable_consumer_launch_delivers_fd3_and_refuses_by_code`). The runner's report records no output for them; the cause is not known.

## ESS 0.69.0

Not released at the time of this record. When it is, the pin moves the way approval-record:ess-pin-0680 records for 0.68.0.
