# Evidence precision audit

Story: `contracts-evidence-precision`. Findings: E15, E16, E17, E18,
E25, E26, E31 and E33 from the immutable external review of `db1c329`.
This is a textual/source audit; it executes no provider and proves no runtime
conformance. Coordinated media/Atlassian edits are verified on integration
before final disposition.

## Sources and reproducibility

The predecessor repository is read at immutable commit
`81459ac42ddd518d3942f4b079841e9e0ed6efc8`, using `git show <commit>:<path>`,
not a mutable web page or an inferred current line number.
[source-excerpts.txt](source-excerpts.txt) retains line-numbered excerpts, in
this order: providers/b10x.toml 741–771; providers/jira.toml 369–376;
docs/design/05-native-sip-and-rtvbp.md 315–318;
specs/grafana/http-api-2026-08-14.openapi.yaml 105–109.
The last excerpt is current core lib.rs 59–79 and 90–113 at common checkpoint
`8e1836cad8ae1b2127ce9ae306c6d8131960db4c`.
SHA256 of the excerpt file:
`37fae11fd3924227ac88e29b80c4c10f2023abc6192adb1ef6b1e6a64c1a2ecc`.

Docker evidence is the already preserved official Engine API v1.56 Swagger
and [endpoint audit](../restart-visibility-20260908/docker-endpoint-audit.json),
with provenance and limitations in
[provider-evidence.md](../restart-visibility-20260908/provider-evidence.md).
Those historical records are unchanged.

## Before/after scenarios

| Finding | Before / deciding observation | Selected correction and expected reading |
|---|---|---|
| E15 | The interrupt rule points at current design.md:315, a different subject. Old design 05:315–318 distinguishes playback clearing from Agent steering. | Cite the full predecessor design path and exact 315–318 range. Clearing playback does not cancel application work. |
| E16 | The cited SIP range ends at a request parameter, before the response schema. | Use 741–771 in shared sessions and native SIP evidence; call/session/state fields now lie inside the cited range. |
| E17 | jira.toml:376 is the operation id; effects are at 370. | Use full predecessor path and 369–376, covering metadata and id. |
| E18 | The original Grafana source is described as GET-only even though it includes POST /api/ds/query. | Explicitly distinguish selected GET operations from the unselected datasource_query POST. Source format does not imply selected runtime support. |
| E25 | A blanket “unpaged” statement implies completeness, and the earlier no-op claim lacked a pin. | State the verified API shape: the four selected list endpoints declare no page-offset/continuation parameter; container list declares a limit. Require separately specified bounded snapshot/completeness semantics. Native lifecycle text relies on the pinned 204/304 distinction and preserves concurrent-change limitations; no live daemon proof is claimed. |
| E26 | Bare configuration numbers look like selected defaults. | Mark outlines illustrative; trace log max_bytes 131072 and document max_body_bytes 262144 to their selected native defaults/ceilings. Other example selections cannot override normative contract limits. |
| E31 | An untyped Kubernetes item schema is treated as a guaranteed status projection. | State that current listing returns the object as listed, and require a declared status projection before claiming preserved status coverage. |
| E33 | Legacy struct ranges truncate fields or are obscured by proposed extended examples. | Link actual Operation (59–68), Invocation (90–98), Outcome (100–105), Response (107–113, custom reader follows). Explicitly distinguish the current structures from proposed extended codecs. |

These eight cases are manually checked source/semantic comparisons, not eight
executed tests. Final integration checks and independent reviewer findings
are recorded in the combined specification-completion evidence.
