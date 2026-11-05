---
format: aep.planning-md/3
id: approval-record:connectors-wave-2026-11-05b
kind: approval-record
status: approved
title: Wave 2026-11-05b (0.68.0) go from the orchestrator
relations:
- decides: story:owner-bounded-read-rechecks-semantic-effects
- decides: story:mcp-mutating-tool-call-revision-rule
- decides: story:claude-mcp-add-registration-is-recorded
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T17:30:48Z", actor: "human:timo", revision: 2}
---

## Go (orchestrator, 2026-11-05)

The directing session `b10x-chef-orchestrator` gave the go for connectors wave 2026-11-05b, release 0.68.0, as proposed:

- Units run U2, then U1, then U3, one build at a time, with the default limits (tell at 8G or 20 cargo minutes, stop at 10G or 30). trees-budget and the disk pre-check run once per unit.
  - U2: story:owner-bounded-read-rechecks-semantic-effects.
  - U1: story:mcp-mutating-tool-call-revision-rule.
  - U3: story:claude-mcp-add-registration-is-recorded.
- U3's one `claude mcp add` run happens only in a rig with an empty HOME. If anything asks for a login or writes outside the rig, the run stops and is reported.
- The staged gate runs S1 to S5c with the proposal's terms. Each stage starts with its estimated peak +40G free, never under 45G, and trees-budget passing. A stage stops at its estimated peak +2G, at twice its estimated cargo minutes, or under 40G free, and the cache is discarded between stages.
- On green: one bot PR; PUT merge after its checks; tag v0.68.0 once main is green. Then the release page, the archive sums, verify-release, and the announcement.
- The three other active stories stay as they are: the two macOS stories and the sustained live read loop.

## ESS pin to 0.87.0 (orchestrator, 2026-11-05)

ESS 0.87.0 was released and verified after the go. The workspace rule moves a repository's ESS pin to the newest release when a wave touches that repository anyway, and the directing session `b10x-chef-orchestrator` applied it to this wave:

- One ride-along commit on the wave branch, after U3 and before the gate, moves the seven ESS git dependencies in `Cargo.toml` and the lockfile from tag 0.86.0 to tag 0.87.0.
- The staged gate uses the `ess` binary at 0.87.0.
- S1 may rebuild in full. If its peak passes the estimate (5G) by more than 2G, S1 stops with its stop line, and the directing session decides whether to raise the term.
