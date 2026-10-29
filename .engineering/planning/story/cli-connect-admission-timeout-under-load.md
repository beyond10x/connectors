---
format: aep.planning-md/3
id: story:cli-connect-admission-timeout-under-load
kind: story
status: draft
title: connections connect times out at admission on a heavily loaded host
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Defect

On a heavily loaded host, `connectors connections connect` can fail with
`{"code":"timeout","stage":"admission"}`. Tests that call it in fixture setup
(`apps/connectors/tests/support/mounted.rs:423`) then fail before the code they test runs.

Observed on 2026-10-29 on Linux, tree `wave/d215-mcp-long-read`, while running
`adversary_a_read_longer_than_one_lease_is_answered_and_the_session_stays_ready` pinned
(`taskset -c 5,6`, `nice -n 19`, 4 `yes` workers on the same CPUs):

- At edb385bb3, 1 of 50 runs failed this way, with the host load average around 71 from other
  sessions' builds.
- At cb6527c56, 11 runs of an interrupted batch failed this way, with the load average around 64.

No run failed at the MCP server itself (story:mcp-long-read-exits-under-load).

## Outcome

The cause is named: which deadline expires in admission, and what it waits on. The decision is
recorded: either the CLI or the owner is wrong under load, or the deadline is the documented
contract and the timeout is the right answer.

## Acceptance

- A reproduction that names the deadline and the wait behind the `admission` timeout.
- Either a server fix with a failing test first, or a recorded reason why the timeout is correct.
- No `#[ignore]` and no retry.

## Further observations

At edb385bb3, a second series of 50 runs pinned to 2 CPUs also had 1 failure of this kind. The
load average was 13 to 34. The two series together failed 2 of 100 runs, and none of the 100 logs
shows an MCP server exit.

Likely mechanism (inferred from the code, not traced):

- `connect` (`apps/connectors/src/local/definition.rs:1078`) calls `owner::Client::connect(paths, true)`.
- Its deadline is a fixed 10 s (`crates/connectors-host/src/local/owner/transport.rs:213`). The
  10 s covers the authority read under the store's `metadata.lock` and any owner start.
- With 4 busy workers on the same 2 CPUs, the owner and the CLI each get about two fifths of a CPU.
