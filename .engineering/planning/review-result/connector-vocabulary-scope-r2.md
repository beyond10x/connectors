---
format: aep.planning-md/3
id: review-result:connector-vocabulary-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, connector-vocabulary stories
relations:
- reviews: epic:connector-vocabulary
- reviews: story:docs-local-guides-vocabulary
- reviews: story:docs-google-publishing-advice
revision: 1
---
needs-revision

story:docs-local-guides-vocabulary — the operator guides `adapters/loki/README.md`, `adapters/prometheus/README.md` and `adapters/grafana/README.md` are in no story's scope or exclusions, so "No user-facing guide, CLI help text or error message says "adapter"" is not claimed for them. Each shows `[adapters.<alias>]` and `--adapter` (loki 11 hits, prometheus 9, grafana 9 of `--adapter|[adapters.`). `README.md:132` links "the Loki guide" and CHANGELOG calls each "the operator guide". The story already takes the sibling non-catalog guides (Kubernetes, SQL, MCP, Tavily) and its Not-in-scope note (line 98) does not exclude these. Add the three files to its Scope and Acceptance, or name them as excluded — `.engineering/planning/epic/connector-vocabulary.md:30`

story:docs-google-publishing-advice — its acceptance (the "personal-use app can be set to In production without verification" correction) is not traceable to any sentence of the epic. The epic promises vocabulary, the connect flow and migration, not a factual correction to Google publishing advice. The correction comes only from the design's § Consequences, and the epic's Outcome binds that design to vocabulary. Either the epic gains the promise, or the story keeps only the Repair-to-Reconnect and `Stale` text (which the epic does cover) and the correction is dropped or recorded as outside this epic — `.engineering/planning/story/docs-google-publishing-advice.md:44`

**What I read.** 17 artifacts: the epic read first, all 15 `decomposes` stories in full, `architecture-decision-record:connector-vocabulary`, `migration-plan:connector-vocabulary`, `decision-blocker:instance-connection-cardinality`, `review-result:connector-vocabulary-scope-r1`, and `docs/design/connector-vocabulary.md`. Commands: `aep plan artifact show` for each id, `aep plan artifact graph` and `list`. I also ran `rg` over the repository (excluding `docs/waves`, `docs/evidence`, `docs/plans`, `target`) for "adapter", `--adapter`, `connections repair` and `[adapters.` to compare stale-text locations with story scopes. I did not run `aep plan artifact kinds` or `relations`; the edge meanings were visible in `graph`.

**Promises.** I extracted 12 from the epic and the operator request. I traced 11 to a story and 1 only in part:
- the enable, add-instance and connect-once flow;
- vocabulary in the CLI;
- vocabulary in the configuration file;
- vocabulary in the ESS model;
- vocabulary in the guides;
- Gmail by name with no pins;
- reads and two guarded writes under one instance;
- `reconnect` and the selectors;
- `Stale`/`Live`;
- migration without re-consent;
- no "adapter" in guides, help or errors;
- README, AGENTS.md, website and docs updated.

The partial one is the guides promise (the first finding). The round 1 gap, the ESS model, is now claimed by `story:ess-connector-vocabulary-model`, and every state-dependent story depends on it. No story claims either out-of-scope item. I found no two stories claiming one outcome.

**What I could not establish.**
- Narrowing the drafter named, which I did not file. `setup migrate` leaves credential-binary, MCP, SQL and Kubernetes entries as `[adapters.<alias>]` because their connector mapping is UNMAPPED. The epic promises that "an existing configuration with `[adapters.<alias>]` entries migrates". The story states this, but no epic Out-of-scope or Open entry and no later story records who finishes it. Say whether it should be recorded in the epic.
- Contract documents that show the old flag and are not in any story's scope, so I did not file them. These are `contracts/service/{approval-issuers,local-mutations,clock}.md`, `contracts/cli/v1alpha1/{consumer-launch,scenarios}.md` and `adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md:65`. They stay true during the deprecation window, and I judged them specification rather than guides.
- The open `decision-blocker:instance-connection-cardinality` is the drafter's named omission, and the epic is already marked blocked on it.
- Out of my lane:
  - Acceptance: `story:docs-google-one-instance-per-account` claims one instance for Drive, Calendar and Slides, but only Gmail is exercised by any acceptance.
  - Acceptance: `story:cli-text-without-adapter` checks renderer output per variant, not host refusal strings.
  - Parallel-safety and design: shared files `ess/domains/cli.yaml`, `contracts/cli/v1alpha1/semantics.md` and `website/docs/reference/cli.md`.

```findings
- file: .engineering/planning/epic/connector-vocabulary.md
  line: 30
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:docs-local-guides-vocabulary: the operator guides adapters/loki/README.md, adapters/prometheus/README.md and adapters/grafana/README.md (which show [adapters.<alias>] and --adapter, and are linked from README.md:132) are in no story's scope or exclusions, so \"No user-facing guide, CLI help text or error message says adapter\" is not claimed for them; add them to that story's Scope and Acceptance or name them as excluded"
- file: .engineering/planning/story/docs-google-publishing-advice.md
  line: 44
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:docs-google-publishing-advice: its acceptance (a personal-use app can be set to In production without verification) is not traceable to any sentence of the epic, which promises vocabulary, the connect flow and migration; keep only the Reconnect and Stale text the epic covers, or add the promise to the epic"
```
