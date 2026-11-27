---
format: aep.planning-md/3
id: story:landed-write-is-not-unknown-after-the-owners-budget
kind: story
status: draft
title: A write that reached the provider is not reported unknown when the owner's 20 s budget ran out
relations:
- depends_on: story:invoke-retries-a-write-that-was-not-attempted-and-never-sends-one-twice
revision: 1
---
## Outcome

A write whose provider request succeeded is reported `applied` with its result, also when the
host is loaded so that the owner's work around the request takes longer than 20 s.

## Observed

2026-11-23, connectors 0.74.0, a side install that posts Slack messages
(`slack/slack-bot`, `chat.postMessage`), host load average 110 to 118:

- Four writes ended with `write failed code=timeout` in the owner's `owner.log`
  (15:43:59, 15:55:37, 16:07:55 and 16:10:18 CET).
- For two of them the caller read the channel back: both messages were in Slack. The log lines
  are 0.3 s and 1.2 s later than the Slack `ts` of those messages.
- The CLI reported `outcome_unknown`, classification `unknown`, no `result.result.body.ts`.
  One caller read that as a failure.
- A write through the same install 8 minutes later, at the same load, took 20 s for describe,
  prepare, issue and invoke together and ended `applied`.

## Cause as far as read

- `mutation::Deadline::start` (`crates/connectors-host/src/local/owner/mutation.rs`) gives every
  write 20 s; `invoke` has no flag for it.
- `exchange_write` (`crates/connectors-host/src/local/owner/transport.rs`) logs `write failed`
  when `write_action` returns an error, and `WriteClient::invoke` maps `Timeout`,
  `Interrupted` and `Unavailable` from the owner to `OutcomeUnknown`.
- Not measured: which part of `write_action` used the 20 s (preflight, the provider request, or
  the metadata work after the provider answered).

## Open, to settle before this story is proposed

- Whether a replay of the same idempotency key after such a timeout returns the stored result
  (`contracts/service/local-mutations.md`: an admitted existing result is observed without new
  dispatch) or `outcome_unknown` again. Not tested: the timeout cannot be caused on demand.
- Whether the fix is a caller-named budget for writes (like `--deadline-seconds` for reads in
  story:invoke-admission-queues-under-load), a result that the owner settles after the
  caller's budget and a same-key observation returns, or both.

## Acceptance

- A test in which the provider answers within the budget and the owner's metadata work after it
  is delayed past the budget: the caller gets `applied` with the provider result, either in the
  first reply or by one observation of the same key, and the provider received one request.
- The test runs within 60 s.

## Workaround in use

The side install's post script reads the channel or thread back after `unknown` and prints the
landed message's `ts`; it sends no second write.
