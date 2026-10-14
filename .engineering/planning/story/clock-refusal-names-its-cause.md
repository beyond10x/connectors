---
format: aep.planning-md/3
id: story:clock-refusal-names-its-cause
kind: story
status: active
title: The approval clock refusal names whether the source was silent, unverified or too imprecise
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T09:25:51Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T09:25:51Z", actor: "human:timo", revision: 3}
---
## Outcome

`connectors approvals clock-check`, and every approval path that acquires the clock, says which of
three things went wrong when no observation results: the source did not answer, the answer did not
verify against the configured key or protocol, or the signed radius makes the interval wider than
the contract admits. Each has its own refusal detail and its own next action.

## Today

Observed 2026-10-14 against public Roughtime sources with `connectors` 0.50.0:

- a source that never answers (roughtime.se, unreachable on both its addresses) answers
  `unavailable`, `next_action: check_configuration`, after the 2 s query timeout
  (`crates/connectors-host/src/local/clock.rs:25`);
- a source whose reply is well formed and verifies, but whose signed radius is 5 s, answers the same
  `unavailable`, `check_configuration`, because the interval exceeds the 4,000 ms width of
  `contracts/service/clock.md:5`;
- a wrong configured key answers the same again (`apps/connectors/tests/local_cli.rs`, the
  clock-check test's refused case).

Every failure inside `protocol::verify` and the socket path maps to `Failure::Unavailable`
(`crates/connectors-host/src/local/clock/protocol.rs`), so an operator cannot tell a dead source
from a misconfigured key from a source this contract cannot use.

## Change

Spec first: the clock contract and `ess/domains/cli.yaml` model the three refusal reasons and
their next actions before the code changes. The contract's bounds, the trusted-source model and the
configured source list do not change; only the refusal names what happened.

- no reply within the query timeout, or a send/receive error: the source is unreachable; next
  action retry later or check the address;
- a reply that does not parse, carries the wrong nonce or version, or fails either signature or the
  Merkle path: the reply did not verify; next action check the configured key and the source's
  protocol version;
- a verified reply whose radius, with the measured delay and timer-rate error, is wider than the
  contract's width: the source is too imprecise for approvals; next action choose a source with a
  smaller radius. The refusal carries the signed radius in milliseconds.

The refusal never carries the observation's bounds, the request or the reply bytes.

## Acceptance

- Three CLI tests against the clock fixture server: no reply, a reply signed by another key, and a
  verified reply with a radius over the width each answer their own refusal detail and next action.
- The existing clock tests and the contract's bounds hold unchanged; `cli --check` holds.
- docs/local-clock.md lists the three refusals and what to do about each.
