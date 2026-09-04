---
format: aep.planning-md/1
id: story:state-becomes-a-port
kind: story
status: active
title: State becomes a port, with a SQLite backend
refs:
- provider: legacy
  reference: S-041
relations:
- derived_from: epic:local-product
scope:
- confidence: cited
  path: crates/service
revision: 4
---
## Acceptance

Verbatim from `docs/stories/S-041-state-becomes-a-port.md:85`. **read**

- [x] A `StateStore` port with a shared key grammar and error vocabulary.
- [x] An in-memory backend, a SQLite backend (in-memory and file), and PostgreSQL behind it.
- [x] One conformance suite, run by every backend, pinning byte-exactness, atomic bounded append,
      bounded reads, delete idempotence and key validation on **every** operation including reads.
- [x] The suite passes against a live PostgreSQL, so backend equivalence is measured rather than
      claimed.
- [x] The five Integrations drop their private state shapes and take `Arc<dyn StateStore>` on the
      hosted seam, and `hosted-vault`'s prepared-transaction journal with them.
- [ ] `integration-jira` composes locally, which today it cannot. It no longer *requires* a
      database — its constructor takes the port — but no personal-posture constructor wires it.
- [ ] `slack` and `gitlab` collapse their owner-only file branch into the port as well.

## Context

Make where an Integration's durable state lives a **port**, the way secrets and egress already are,
so a deployment chooses a backend instead of the code choosing a branch — and give it a SQLite
implementation that is in-memory for tests and a file for a workstation.

Source frontmatter: pillar Platform · areas [service, state, testing] · priority 2 · design `../design/12-one-owner-for-every-outside-connection.md`. **read**

Source `note:` field, quoted: “the port, its three backends and the shared conformance suite have landed, and every hosted state seam now takes Arc<dyn StateStore> — serve-hosted runs on SQLite with no database server. What remains is the personal posture: slack and gitlab still keep an owner-only file branch beside the port, and jira has no personal constructor at all.”

## Status

`in-progress` in the source. Quoted from `docs/stories/S-041-state-becomes-a-port.md:5`: `status: in-progress`. **read**

## Provenance

Migrated from `docs/stories/S-041-state-becomes-a-port.md`, which is not deleted and now names this artifact.

- First written 2026-08-20 · last touched 2026-08-21 · 2 revision(s)
- Legacy id `S-041`, recorded as the reference `legacy:S-041`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
