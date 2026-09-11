---
format: aep.planning-md/1
id: story:catalog-provider-bundle
kind: story
status: implemented
title: Write and load a provider bundle through a local index
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
revision: 4
---
## Outcome

A provider's ingested source and its operation inventory are written as one
bundle and loaded again on demand through a local index, with no central catalog
service. A bundle that does not match what the index recorded is refused, so a
stale or altered bundle cannot be read as current.

## Scope

`crates/connectors-catalog/` only. Writing a bundle records the provider name,
the bundle's own SHA-256 and the source digest it was built from into an index
beside it. Loading reads the index, then the bundle, then checks the bundle's
digest against the recorded one before returning anything.

Refused by name: a provider the index does not carry, a bundle file the index
names but the directory does not hold, a bundle whose digest differs from the
recorded one, and a second write for a provider already indexed unless it is
explicitly replacing it.

Compression, container format and cache behaviour are not claimed here. The
index is a file in a directory; this story does not make it a service.

## Acceptance

A bundle written and loaded returns the same source record and inventory it went
in with; altering one byte of the bundle file makes the load refuse with the
digest reason rather than returning data; an unknown provider and a missing file
each refuse with their own reason; replacing an indexed provider is refused
unless replacement is asked for, and the index lists its providers in a stable
order.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
