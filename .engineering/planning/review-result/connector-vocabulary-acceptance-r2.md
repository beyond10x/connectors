---
format: aep.planning-md/3
id: review-result:connector-vocabulary-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, connector-vocabulary stories
relations:
- reviews: story:catalog-connector-declarations
- reviews: story:cli-connector-instance-selectors
- reviews: story:cli-text-without-adapter
- reviews: story:connection-stale-state
- reviews: story:connections-reconnect-verb
- reviews: story:connector-config-dual-read
- reviews: story:connector-instance-derived-fields
- reviews: story:docs-catalog-guides-vocabulary
- reviews: story:docs-google-one-instance-per-account
- reviews: story:docs-google-publishing-advice
- reviews: story:docs-local-guides-vocabulary
- reviews: story:docs-pinned-catalog-guides-vocabulary
- reviews: story:docs-vocabulary-readme-website
- reviews: story:ess-connector-vocabulary-model
- reviews: story:setup-migrate-adapter-entries
revision: 1
---
needs-revision

story:cli-connector-instance-selectors — the acceptance still joins two independent outcomes: the two runs are identical bar one deprecation line, and `adapter`, `connector` and `instance` are present in every JSON result. Two identical runs that both lack `connector`/`instance` satisfy the first and fail the second. The round-1 finding on this story is part-fixed: it now runs every `--adapter` command path (:87) and the three outcomes are down to two (:90-91). — .engineering/planning/story/cli-connector-instance-selectors.md:90
story:docs-vocabulary-readme-website — the title's outcome, "README introduces the connector vocabulary once", is observed nowhere. The acceptance is `connectors-docs vocabulary` exiting 0 over the listed files, and the check's four rules only forbid text (`adapter`, a table row in a file other than `README.md`, pins, extra instances). A `README.md` that is cleaned of "adapter" but defines none of the six terms passes, and nothing checks that `AGENTS.md` links to it. The acceptance needs a rule that requires the six term rows in `README.md`. — .engineering/planning/story/docs-vocabulary-readme-website.md:103
story:ess-connector-vocabulary-model — the acceptance is "`ess specify validate --path ess` exits 0 on an `ess/` that declares every entity, relation and lifecycle transition listed under Change". Validate also exits 0 on today's `ess/` without the new entities, so the exit status does not separate before from after (`crates/connectors-build/tests/mcp_domain_model_census.rs:325` records that validate accepts models it leaves open). The declared-and-listed half is carried by prose. It points at relations "listed under Change", but the relations (Connector -> Provider, instance -> Connector and the rest) are listed under Domain relations (:66-70) and not under Change. The acceptance should name the entities, relations and transitions and a command that prints them. — .engineering/planning/story/ess-connector-vocabulary-model.md:83
story:setup-migrate-adapter-entries — the acceptance names only what stays equal (connection ids and states, policy revision, no credential entered) after `--apply`. It never states that the catalog entries became `[connectors.<c>.instances.<alias>]` or that the other kinds stayed `[adapters.<alias>]`. An `--apply` that rewrites nothing therefore passes, and the title's outcome ("rewrites adapter entries as connector instances") is unobserved. — .engineering/planning/story/setup-migrate-adapter-entries.md:87

**What I read:** all 15 of 15 stories that decompose `epic:connector-vocabulary` (whole bodies, via `cat -n .engineering/planning/story/<id>.md`), plus `review-result:connector-vocabulary-acceptance-r1`. The ids came from `aep plan artifact graph`, filtered for `decomposes -> epic:connector-vocabulary`. I also ran `aep plan artifact kinds`, `git grep` on the tree (`--print-local-bootstrap`, the `FailureCode` renderer, `docs/catalog-google-oauth.md` headings) and `ess --version` (0.60.0). I did not open anything under `ess/` beyond `domains/cli.yaml:554-600`.

**What I could not establish:**
- **Round-1 findings:** all ten held against the old text; none of them recurs on the stories as revised, except the first finding above (cli-connector-instance-selectors), which is part-fixed. The acceptances now name checked sets, an allowlist or a command, and are one test each.
- **Acceptances approved this round:** catalog-connector-declarations, cli-text-without-adapter, connection-stale-state, connections-reconnect-verb, connector-config-dual-read, connector-instance-derived-fields, docs-catalog-guides-vocabulary, docs-google-one-instance-per-account, docs-google-publishing-advice, docs-local-guides-vocabulary and docs-pinned-catalog-guides-vocabulary. The last four are the ones judged only on their command or text, which I did not run.
- **`connectors-docs vocabulary`:** it does not exist yet. `crates/connectors-docs/src` has no such subcommand, so the docs-guide acceptances depend on `story:docs-vocabulary-readme-website` landing first. That ordering is a `depends_on` edge, not an acceptance defect.
- **cli-text-without-adapter, item 3:** "calling the renderer once per variant" assumes a callable human-failure renderer. In the tree, `render_reply` and `failure` live in the generated `apps/connectors-cli-contract/src/runtime.rs` and are private, so I could not confirm the test can call it per variant. This is an unease, not a finding.
- **Out of my lane, not counted:** `story:docs-google-publishing-advice` keeps its `connectors-docs vocabulary` check as an obligation outside the acceptance. The `ess/` working-tree edits are the other agent's in-flux work.

```findings
- file: .engineering/planning/story/cli-connector-instance-selectors.md
  line: 90
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance still joins two independent outcomes (the two runs are identical bar one deprecation line; adapter, connector and instance are present in every JSON result), since two identical runs both lacking connector and instance pass the first and fail the second"
- file: .engineering/planning/story/docs-vocabulary-readme-website.md
  line: 103
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is the vocabulary check exiting 0, whose four rules only forbid text, so a README.md that defines none of the six terms passes and the title's outcome (README introduces the vocabulary once; AGENTS.md links to it) is observed nowhere"
- file: .engineering/planning/story/ess-connector-vocabulary-model.md
  line: 83
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "ess specify validate exits 0 on the ess/ of today as well as after the work, so the exit status names no transition, and the relations it must declare are listed under Domain relations, not under Change, so the declared-and-listed half is ambiguous prose"
- file: .engineering/planning/story/setup-migrate-adapter-entries.md
  line: 87
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names only what stays equal after --apply (connection ids and states, policy revision, no credential), not that catalog entries became [connectors.<c>.instances.<alias>] or that other kinds stayed [adapters.<alias>], so an --apply that rewrites nothing passes"
```
