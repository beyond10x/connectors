# Three MCP contract stories, gated together

`story:mcp-profile-selection-matrix`, `story:mcp-inbound-local-binding` and
`story:mcp-outbound-connection-lifecycle` were built in three separate worktrees
and merged onto one branch before any of them closed.

## What ran

| | |
|---|---|
| command | `cargo run --locked -p connectors-build -- gate --msrv` |
| branch | `wave/mcp-w3-20260912` at `34d3704` |
| result | **35 gate steps, 85 test targets, every step exit 0** |
| log | [gate.log](gate.log), ending `GATE-EXIT:0` |

## What integrating first was worth

Each unit was green alone. The merge was not, twice, and neither defect was
reachable from inside a single tree.

**The matrix moved under the binding.** The selection matrix's own correction
re-dispositioned inbound Streamable HTTP from `deferred` to `supported` — the
blocker it had been deferred against never named a transport. That made the matrix
support two inbound transports while the local binding specifies one, and the
binding's check asserted the supported count was exactly one. The check now
requires every supported inbound transport to be either specified or attributed to
the story whose placement owns it; an exact count silently claimed them all.

**Two units answered one question differently.** Both needed to mark which story
owns a scenario file. The inbound unit used a `# Owner:` comment, which ESS
ignores. The outbound unit used a `story:` YAML key, which ESS **refuses** — "a
key it does not know is refused rather than ignored". The first merged gate run
was 33 steps and 85 targets green with nothing failing, and then refused all 23
outbound scenarios at the synthesis step.

## A gap this run closes, and one it states

Three units independently found that neither MCP scenario directory reached any
gate step: 9 inbound and 23 outbound files compiled in their own package lanes and
nowhere else. The one-line fix was right for one directory and wrong for the other.

The gate now collects the inbound set, which is `ess-scenario/1` and which the
shared IR synthesises. It does not collect the outbound set, which is
`mcp-outbound-lifecycle/1`, a format this repository defined and ESS does not
know. Those 23 files are held by their own package-scoped cases — **a weaker
guarantee**, stated in the gate's own source so the absence does not read as an
oversight. Whether that format should become `ess-scenario/1` is a real question
and no part of this run answers it.

## What these three stories do not establish

Contracts only. No MCP connection is made, no server starts, no credential
persists, and the scenarios are compiled rather than executed — ESS compiles
obligations and does not execute a trace. Nine of the epic's twelve stories remain
undrafted work, and two decision blockers withhold `review` on the epic itself.
