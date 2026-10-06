---
format: aep.planning-md/3
id: review-result:probe-epic-scope-r1-20261006
kind: review-result
status: active
title: Scope critic, round 1, epic:connector-probe-20261006
relations:
- reviews: story:dependency-refresh-20261006
- reviews: story:registry-clock-floor-growth
- reviews: story:owner-memory-bounded
- reviews: story:cli-json-answers-as-json
- reviews: story:upgrade-adapter-identity-mismatch-names-new-connection
- reviews: story:forge-issue-create
- reviews: story:expired-evidence-invoke-advises-revalidate
- reviews: story:catalog-honours-retry-after
- reviews: story:service-failure-carries-upstream-reason
- reviews: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
needs-revision

story:cli-json-answers-as-json — the acceptance requires "the examples triage demo, the MCP adapter" to read the new form, which is work in another repository (`beyond10x/examples`) and in an MCP adapter the story's own scope found no trace of, and the parent promises only that `describe` and `invoke` answer JSON as JSON (#105) — .engineering/planning/story/cli-json-answers-as-json.md:52

**What you read:** 11 artifacts (the epic and its 10 `decomposes` stories). I ran `aep plan artifact show` on the epic and on every story, `aep plan artifact list --format json` filtered on the `decomposes` edge, and `aep plan artifact graph`. I also read issues #101, #103, #105 and #81 with `gh issue view`. I extracted 9 promises from the epic and traced all 9 to a story. Each is claimed once.

| Epic promise | Claimed by |
|---|---|
| Store growth per command, the registry clock floor (#101) | `story:registry-clock-floor-growth` |
| Owner memory (#103) | `story:owner-memory-bounded` |
| Expired evidence advises the wrong action | `story:expired-evidence-invoke-advises-revalidate` |
| Identity mismatch during an upgrade | `story:upgrade-adapter-identity-mismatch-names-new-connection` |
| Dispatch `unauthorized` without its upstream reason | `story:service-failure-carries-upstream-reason` |
| `429` honours `Retry-After` | `story:catalog-honours-retry-after` |
| `operations describe` and `invoke` answer JSON as JSON (#105) | `story:cli-json-answers-as-json` |
| GitLab issue creation (#81) | `story:forge-issue-create` |
| Crates.io updates, ESS 0.53.0, AEP 0.68.0 | `story:dependency-refresh-20261006` |

`story:metadata-invoke-cost-flat-in-store-size` is the tenth story. The epic's own exclusions name it as blocked on beyond10x/entity-runtime#55, so it is not reach beyond the parent.

**What you could not establish:**
- The epic's Outcome says per-command metadata cost "stop[s] growing with the store". Only the blocked metadata story carries that fully, and the epic excludes it. #101's other asks are not claimed by any story: CLI-side work that avoids a full verify per command, and snapshot or compaction of settled history. I did not call this a gap because the epic's In scope narrows #101 to the clock floor, and its Not in scope names the upstream open.
- `story:registry-clock-floor-growth` is titled "no longer records an event on every command", but its acceptance only halves the clock-floor events per invoke. That is acceptance wording, outside my lane.
- `story:cli-json-answers-as-json` has two acceptance-lane issues. It names an MCP adapter while its Scope says "no MCP code in this repository or `~/beyond10x/mcp` reads this output". It also decides "a new output version or a documented compatible addition" in the story itself.
- `story:expired-evidence-invoke-advises-revalidate` ends its first alternative with "or: invoke revalidates transparently". That may be a design-lane question, and it does not affect scope.
- Whether the six stories that touch `local.rs`, `er.rs` or `Cargo.lock` can run at once belongs to parallel-safety.

```findings
- file: .engineering/planning/story/cli-json-answers-as-json.md
  line: 52
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the acceptance requires "the examples triage demo, the MCP adapter" to read the new form, which is work in another repository and in an MCP adapter the story's own scope found no trace of, and the parent promises only that describe and invoke answer JSON as JSON (#105)
```
