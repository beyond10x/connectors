---
format: aep.planning-md/1
id: story:mcp-specification-pin
kind: story
status: active
title: Pin the authoritative MCP specification revisions with exact source digests
relations:
- decomposes: epic:mcp-contracts
- serves: vision:independent-contract-adapters
- informed_by: initiative:complete-local-connectors
scope:
- confidence: inferred
  path: adapters/README.md
- confidence: inferred
  path: adapters/mcp/contracts/protocol/v1alpha1/evidence
- confidence: inferred
  path: adapters/mcp/design.md
revision: 5
---
## Acceptance

`adapters/mcp/contracts/protocol/v1alpha1/evidence/<date>/` holds the exact MCP
specification revisions this repository authors against — the primary revision and
the one explicit interoperability revision — each recorded with its source URL,
uncompressed SHA-256 and byte length in a `specification-source-hashes.json`, its
archived bytes under `vendor/`, and a prose record naming which sections later
contract statements will cite, so that every MCP claim in this epic resolves to a
source line rather than a recollection.

## Scope

- `adapters/mcp/design.md` — new; the owner document, following the shape of
  `adapters/loki/design.md`
- `adapters/mcp/contracts/protocol/v1alpha1/evidence/` — new; the pinned-source
  record, hashes and vendored archive
- `adapters/README.md` — one row in the `## Owners` table (shared file; no other
  story in this decomposition touches it)

## Domain relations

None. This story declares no entity and states no relation between one noun and
another; it pins the document the remaining eleven stories read. That ordering is
`aep plan artifact waves`-visible: every other story in this decomposition
depends, directly or transitively, on this one.

## Why this comes first

`epic:mcp-contracts` requires it in its own words: "Pin the authoritative MCP
specification revision and select supported transport/capability profiles at
authoring." It also states the failure this guards against — "Do not equate a
protocol capability listing with implemented support" — which is only checkable
against a fixed revision.

The two revisions are already named, by the operator's recorded plan rather than
by any judgement made here. `initiative:complete-local-connectors:33` reads: "Pin
that exact published MCP revision here and implement outbound and inbound
tools/resources/prompts over stdio and Streamable HTTP, with version-specific
2026-07-28 and explicit 2025-11-25 interoperability, caller isolation and
authenticated local verification of the cloud-capable server profile." This story
pins both; it does not choose them.

*(This citation read `:35` at revisions 1 to 4. The implementor of this story
grep-verified the sentence at line 33 and reported the error; corrected here. The
companion citation `:32` below was checked at the same time and is right.)*

`initiative:complete-local-connectors:32` names the source: the sibling `../mcp`
repository, whose expansion and publication is that initiative's milestone 4
("Expand ../mcp with resources, prompts, servers, progress/cancellation,
consumer-owned stdio and explicit OAuth one-use exchange/publication seams"). This
story pins a revision; it does not wait for that publication, and it does not
publish anything. If the revision that gets pinned is an upstream specification
document rather than a `../mcp` commit, the record says which, because the two are
different authorities and a later reviewer must be able to tell them apart.

**That question is now answered, by the implementation.** The two dates are
upstream specification revisions, not `../mcp` releases: upstream
`modelcontextprotocol/modelcontextprotocol` carries release tags `2026-07-28` at
`5f5440bb26a62e2cf3440b92da5a667efa03b267` and `2025-11-25` at
`38c84e9f93ad191d9eb26d92b945d17bd0efcaf3`, and
`/home/timo/beyond10x/mcp/crates/b10x-mcp-types/src/lib.rs:15,17` declares those
same two strings as `CURRENT_PROTOCOL_VERSION` and `LEGACY_PROTOCOL_VERSION` —
a consumer's selection of them, not their source. Upstream is what is pinned.

## The discipline this follows

`adapters/kubernetes/contracts/logs/v1alpha1/evidence/20260909/provider-sources.md`
is the repository's existing standard and this story copies it: "retains the exact
URL, uncompressed SHA-256 and byte length of core/v1/types.go from Kubernetes
v1.35.0, commit `66452049f3d692768c39c797b21b793dce80314e`. The new `gzip -n`
archive was decompressed and compared byte-for-byte with the read-only research
cache; its original manifest hash was independently checked." The same document
also states the limit that applies here: "These are provider declarations, not
evidence that a particular cluster implements them." A pinned MCP revision is
evidence about the protocol, never about any server.

`adapters/loki/contracts/logs/v1alpha1/evidence/20260909/` shows the same
arrangement with `provider-source-hashes.json` beside `vendor/*.gz`.

## What it does not cover

Selecting which transports and capability profiles this repository supports —
that is `story:mcp-profile-selection-matrix`, which reads this pin. Declaring any
MCP noun — that is `story:mcp-domain-model`. No connection is made to any MCP
server, and no executable is added.
