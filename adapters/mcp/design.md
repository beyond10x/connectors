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

- Which transports and capability profiles this repository supports. The pinned revisions
  specify **four** transport bindings, not two:

  | Binding | Status in the pinned revisions | Cited in the archives |
  |---|---|---|
  | stdio | current | `2026-07-28 basic/transports/stdio.mdx`; `2025-11-25 basic/transports.mdx` §stdio (22) |
  | Streamable HTTP | current | `2026-07-28 basic/transports/streamable-http.mdx`; `2025-11-25 basic/transports.mdx` §Streamable HTTP (54) |
  | custom transports | permitted, MUST-level constraints | `2026-07-28 basic/transports/index.mdx` §Custom Transports (60): "Implementers who choose to support custom transports **MUST** preserve the … metadata model"; `2025-11-25 basic/transports.mdx` §Custom Transports (313) |
  | HTTP+SSE, from `2024-11-05` | **deprecated, not removed** | `2026-07-28 basic/transports/streamable-http.mdx` §HTTP+SSE Transport (2024-11-05) (695), "New implementations **SHOULD NOT** adopt it; existing implementations **SHOULD** migrate" (703-705) and the fallback procedure (710-737); `2026-07-28 deprecated.mdx` (31) lists it under `## Deprecated` (22) and above `## Removed` (37); `2025-11-25 basic/transports.mdx` §Backwards Compatibility (284-311), whose fallback ends "should use that transport for all subsequent communication" (309-311) |

  The initiative names stdio and Streamable HTTP as implementation targets; that is a
  target list, not the option space. HTTP+SSE matters precisely because `2025-11-25` is
  pinned for the legacy seam: that revision's documented fallback terminates in HTTP+SSE,
  so an outbound client that follows it lands on a binding the target list does not
  mention. Selecting, bounding and specifying transports — including an explicit refusal
  of custom transports or of HTTP+SSE, if that is the selection — together with framing,
  streaming, cancellation, progress, session loss and version mismatch, is a separate
  authoring obligation that reads this pin. A selection matrix built from this bullet has
  a row for each of the four.
- Any MCP noun, its identity, cardinality or lifecycle. No entity or relation is declared
  by this directory, and none may be inferred from the pinned documents without its own
  modelled and reviewed record.
- Any mapping onto shared service, records, session, auth, discovery or mutation
  contracts.

A capability listed in a pinned specification document is a protocol declaration. It is
not evidence that any MCP server implements it and it is not a support claim by this
repository. That distinction is the reason the revisions are pinned before anything is
authored.

## The boundary obligation this directory creates

Creating `adapters/mcp/` made `mcp` a forbidden term in **shared** ESS, silently and as a
side effect of the directory name. `crates/connectors-build/src/ess_boundary.rs` lines
300-302 insert every adapter owner's words into the boundary gate's forbidden term set
unless that name is listed in the reviewed `shared_protocol_names` policy, and
`crates/connectors-build/ess-boundary.json` line 3 lists exactly one name there: `sip`.

Nothing is red today — shared `ess/` and `contracts/` name MCP nowhere, and the gate
exits 0 with this directory present. The obligation is for later: a story in this epic
that needs *shared* vocabulary to say MCP will be refused by the boundary gate, and the
refusal will read as that story's own bug rather than as this one's consequence. `sip` is
the precedent for what the exception has to look like — an entry in the reviewed
`shared_protocol_names` policy, justified by the term being genuine shared protocol
vocabulary rather than native provider semantics. Per
[the adapter index](../README.md#boundary-gate), such an entry cannot suppress a
forbidden native term or an adapter-path dependency, and no per-file suppression exists.
Native MCP vocabulary stays in this directory and needs no exception at all.

## Sources and remaining obligations

The only retained source is the specification pin above; it is research provenance, not
adopted upstream source/license packaging, a build dependency or a vendored library.
`upstream/`, `spec/ess/`, `generated/` and `src/` do not exist here and are not implied.

Remaining obligations, all unstarted: the protocol contract documents under
`contracts/`, the transport and capability profile selection, the authored native model,
and every runtime, fixture and conformance artifact. No MCP connection has been made from
this repository and no executable has been added.
