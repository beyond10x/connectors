---
format: aep.planning-md/3
id: story:fixtures-truncation-newline-retry-after
kind: story
status: draft
title: Fixtures for a truncated file read, a file update's trailing newline and 429 Retry-After
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The recorded-fixture suite covers three provider behaviours a live consumer met and the fixtures
did not: a file read over a size bound, a file update's trailing newline, and an HTTP 429 with
`Retry-After`.

## Today

From a downstream consumer's live test of 0.50.0, 2026-10-14: no fixture reads a file larger than
the read bound (so the truncation flag is untested), none checks that a file update keeps or adds
the trailing newline the caller sent, and none answers 429 with a `Retry-After` header.

## Change

Spec first where the operation contracts state these (the size bound and its truncation flag, the
update's byte-exact content, the retry hint); add the fixtures and the tests that read them. A
behaviour the contract does not state is added to it first.

## Acceptance

- A file read over the bound answers the bounded prefix with `truncated: true`.
- A file update round-trips its trailing newline byte for byte.
- A 429 with `Retry-After` answers `capacity` with the retry hint in milliseconds.
