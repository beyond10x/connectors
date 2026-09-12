# mcp adapter design

**Status:** specification-only owner directory. No adapter service, package, ESS model,
generated projection or runtime declaration exists, and nothing here is implemented.

## Scope and dependencies

`adapters/mcp/` owns this repository's native Model Context Protocol material: the pinned
upstream specification revisions, and later the authored protocol contracts that cite
them. `epic:mcp-contracts` owns the contract programme; this directory is where its
authored output lands. Both directions the epic names — outbound use of a remote MCP
server, and inbound MCP exposed by this product — are authored against the same pinned
revisions, but neither is specified yet.

The owner directory is a sibling of every other adapter and depends on none of them. It
carries no shared-contract selection yet, because no operation, profile or binding has
been authored to select one.

## Pinned specification revisions

| Revision | Role | Upstream tag | Commit |
|---|---|---|---|
| `2026-07-28` | primary | `2026-07-28` | `5f5440bb26a62e2cf3440b92da5a667efa03b267` |
| `2025-11-25` | interoperability | `2025-11-25` | `38c84e9f93ad191d9eb26d92b945d17bd0efcaf3` |

[The pinned-source record](contracts/protocol/v1alpha1/evidence/20260912/specification-sources.md)
holds the exact URLs, uncompressed SHA-256 values, byte lengths and archived bytes, the
command that reproduces them, and the sections later contract statements will cite. The
authority is the upstream specification repository `modelcontextprotocol/modelcontextprotocol`,
not the sibling `../mcp` repository, which is a consumer of these revisions and is not
pinned here. The two revisions are named by `initiative:complete-local-connectors` line
33; they are not selected by this document.

## What is deliberately not decided here

- Which transports and capability profiles this repository supports. stdio and Streamable
  HTTP are the transports the pinned revisions define and the initiative names as
  implementation targets; selecting, bounding and specifying them — framing, streaming,
  cancellation, progress, session loss and version mismatch — is a separate authoring
  obligation that reads this pin.
- Any MCP noun, its identity, cardinality or lifecycle. No entity or relation is declared
  by this directory, and none may be inferred from the pinned documents without its own
  modelled and reviewed record.
- Any mapping onto shared service, records, session, auth, discovery or mutation
  contracts.

A capability listed in a pinned specification document is a protocol declaration. It is
not evidence that any MCP server implements it and it is not a support claim by this
repository. That distinction is the reason the revisions are pinned before anything is
authored.

## Sources and remaining obligations

The only retained source is the specification pin above; it is research provenance, not
adopted upstream source/license packaging, a build dependency or a vendored library.
`upstream/`, `spec/ess/`, `generated/` and `src/` do not exist here and are not implied.

Remaining obligations, all unstarted: the protocol contract documents under
`contracts/`, the transport and capability profile selection, the authored native model,
and every runtime, fixture and conformance artifact. No MCP connection has been made from
this repository and no executable has been added.
