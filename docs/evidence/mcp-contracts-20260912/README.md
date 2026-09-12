# The first two MCP contract stories

`epic:mcp-contracts` had twelve draft stories and no implementation. Two are now
`implemented`: the pinned specification revisions, and the adapter-owned domain
model that reads them.

## What ran

| | |
|---|---|
| command | `cargo run --locked -p connectors-build -- gate --msrv` |
| branch | `wave/mcp-contracts-20260912` |
| result | **34 gate steps, 74 test targets, every step exit 0** |
| log | [gate.log](gate.log), ending `GATE-EXIT:0` |

`CARGO_BUILD_JOBS=1`, `RUSTC_WRAPPER=""`, `TMPDIR` task-owned under the worktree's
`.local/tmp`, pinned ESS selected explicitly.

## What the pin establishes, and what it does not

54 upstream MCP specification files are archived with their source URL,
uncompressed SHA-256 and byte length, across both revisions the initiative names —
`2026-07-28` primary and `2025-11-25` for interoperability. Two adversary passes
could not break them: every digest re-derives from the archived bytes, and every
recorded URL re-fetches to exactly those bytes at the recorded commit.

It establishes the protocol documents and nothing else. **No MCP connection was
made, no server started, no credential persisted, no transport selected.** A
pinned revision is evidence about a specification.

## Three version-string traps, recorded because eleven stories read this pin

| where | says |
|---|---|
| `2025-11-25` schema, `LATEST_PROTOCOL_VERSION` | `DRAFT-2025-v3` |
| that revision's lifecycle examples | `2024-11-05` |
| its transport example | `MCP-Protocol-Version: 2025-06-18` |

The negotiable string `2025-11-25` is stated by the **primary** revision, not by
the revision it names. A model that reads the obvious constant carries the wrong
string.

## A gate step this wave added

Nothing in this repository had ever re-derived an archived upstream source digest,
for any adapter. Six manifests recorded 84 files and no step read one, so every
pinned-evidence record was evidence by assertion. Two agents found that
independently, from different directions, without seeing each other's work. The
gate now checks all 84; the step is in this run's log.

## A gate refusal that looks like a failure and is not

The first attempt on this branch exited 1 at its **first** step having checked
nothing: `.local/toolchains` is gitignored, so a fresh managed worktree has
neither the pinned ESS nor the pinned AEP binary. Only what the step *printed*
said so — the exit code alone reads as a red gate. Symlink the primary checkout's
directory into the worktree.

## Limits

The gate ran on `wave/mcp-contracts-20260912`, below the merge commits into
`main`. Ten of the epic's twelve stories remain `draft`, and two decision blockers
withhold `review` on the epic itself.
