---
format: aep.planning-md/3
id: decision-blocker:catalog-first-run-source
kind: decision-blocker
status: cleared
title: Nobody has decided where a shipped binary gets its catalog connectors before any definitions install
relations:
- blocks: epic:catalog-connectors-only-in-definitions
- blocks: story:instances-resolve-through-locked-index
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-20T08:53:52Z", actor: "human:timo", revision: 3}
---
## Question

Where does a shipped `connectors` binary get its catalog connectors before any definitions install?

## Why it blocks

The epic deletes `adapters/catalog/providers` and `adapters/catalog/generated/bundles`. Today that
data is compiled into the binary (`adapters/catalog/build.rs:1-5,27-76`,
`adapters/catalog/src/local.rs:264-268,290-310,348-413`; `apps/connectors/src/local/migrate.rs:19-36`:
"the provider and this binary are released together"). The release archive carries only the two
binaries, README and CHANGELOG (`.github/workflows/release-binaries.yml:107,203-213`).
`story:catalog-connector-declarations` requires the binary to carry the data
(`.engineering/planning/story/catalog-connector-declarations.md:56-58`). D5 of
architecture-decision-record:provider-definitions-and-runtimes says "Nothing updates implicitly".

## Options

- A: a fresh install has no catalog connectors until the operator runs the D5 install of a named
  connectors-definitions tag; setup says so.
- B: the release archive carries the pinned connectors-definitions release next to the binary, and
  first run installs that exact, digest-checked release; nothing is fetched implicitly.
- C: the binary embeds the pinned release at build time from a fetched, digest-checked archive (not
  from the repository tree).

Raised by the decomposition of epic:catalog-connectors-only-in-definitions on 2026-10-20.

## Decided

Option B, 2026-10-20, by the b10x-chef-orchestrator session on the operator's delegation, reversible
if the operator answers otherwise: the release archive carries the pinned connectors-definitions
release next to the binary, and the first run installs exactly that, digest-checked; nothing is
fetched implicitly. Its reasons, verbatim: "A leaves a shipped binary with no catalog connectors and
C ties every definitions change to a rebuild." The connectors session had recommended B to the
operator; the operator has not answered.

## Decided by the operator: option C

2026-10-20, about 11:05, the operator answered "C", relayed to the connectors session by the
b10x-chef-orchestrator session; this replaces option B above. The build embeds the pinned
connectors-definitions release: the release named by the pin of story:definitions-pin-covers-catalog
is fetched and digest-checked before the build, and the binary carries it. The repository holds no
copy of a definition. A D5 install of another tag stays explicit.
