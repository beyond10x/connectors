---
format: aep.planning-md/1
id: story:the-transport-refusal-names-its-class
kind: story
status: implemented
title: The transport refusal names its class
refs:
- provider: legacy
  reference: S-066
relations:
- derived_from: epic:endpoint-plane
revision: 5
---
## Acceptance

Verbatim from `docs/stories/S-066-the-transport-refusal-names-its-class.md:22`. **read**

- [x] The transport arm of the refusal log carries an error class derived from the transport
  error (timeout / connect / tls / body-read / other) — never the error's Display string,
  which can embed the URL.
- [x] The transport class distinguishes an oversized upstream body from unreachability; an
  executor cap breach refuses as "response too large", never "unreachable".
- [x] `grafana_dashboards_list` dispatches with a bounded page `limit` and follows `continue`
  server-side up to a hard budget, so the 4.6 MB namespace lists instead of refusing.
- [x] The alertmanager document resolves the v2 API sub-path; `alertmanager_alerts` returns live
  alerts on dev through the mediated route. (The document half is test-proven through the real
  resolve path; the live-dev half needs the next deploy to pick the rebuilt catalog pack up.)
- [x] The alertmanager 403 is IN scope — diagnosis below proved it a document defect, not
  provisioning.

## Context

S-065's live diagnosis (rev 16) pinned `grafana_dashboards_list` to an upstream-transport
failure that is specific to that operation's resolved request — `grafana-datasources-list`
succeeds on the same direct route, credential and origin every reconcile tick. The final
discriminator (timeout vs connect vs TLS vs body-read) is discarded at the transport mapping
(`integration-monitoring`, the `.map_err(|_| UpstreamFailure::Transport)` sites), so the last
step of the diagnosis is blind.

Source frontmatter: pillar Platform · areas [integrations] · design `../design/15-a-zero-configuration-endpoint-plane.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-066-the-transport-refusal-names-its-class.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-066-the-transport-refusal-names-its-class.md`, which is not deleted and now names this artifact.

- First written 2026-08-24 · last touched 2026-08-24 · 4 revision(s)
- Legacy id `S-066`, recorded as the reference `legacy:S-066`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
