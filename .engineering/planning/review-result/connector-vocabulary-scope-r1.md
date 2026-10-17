---
format: aep.planning-md/3
id: review-result:connector-vocabulary-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, connector-vocabulary stories
relations:
- reviews: epic:connector-vocabulary
revision: 1
---
needs-revision

epic:connector-vocabulary — "the CLI, the configuration file, the ESS model and the guides use the vocabulary" promises an ESS model change, but no drafted story's scope or change claims the Provider and Connector entities or the `Stale` transitions on `connectors.auth_bindings.Connection`, so each story cites that model as input "being drafted" with no artifact to depend on. The only ESS file a story claims is `ess/domains/cli.yaml`, and `ess/domains/auth_bindings.yaml` is in no story's scope. `story:connection-stale-state` is the natural owner of the `auth_bindings.yaml` lifecycle change, and the Provider/Connector/Instance entities need a story or a stated decision that this is out-of-band work — `.engineering/planning/epic/connector-vocabulary.md:14-16`

**What I read.** 13 artifacts: the epic and the 12 stories, read in full. Also `migration-plan:connector-vocabulary`, `decision-blocker:instance-connection-cardinality`, `docs/design/connector-vocabulary.md`, and the working-tree diff of `ess/domains/` (read-only). Commands: `aep plan artifact show` for each id, `aep plan artifact graph`, `aep plan artifact kinds`, plus `git diff` and `grep` over `docs/`, `website/` and the contract READMEs.

I extracted 10 promises from the parent and traced 9 to a story. The 10:
- the enable, add-instance and connect-once flow;
- vocabulary in the CLI;
- vocabulary in the configuration file;
- vocabulary in the ESS model;
- vocabulary in the guides;
- the five epic acceptance bullets that do not repeat the outcome (Gmail by name, reads and writes under one instance, reconnect and selectors, `Stale`/`Live`, migration);
- the "no adapter in guides, help or errors" bullet.

The ESS model promise is the untraced one. No story claims either out-of-scope item, and none reaches into them.

**What I could not establish.**
- I have no drafter's report, so I cannot tell whether the ESS work was meant as out-of-band. The `auth_bindings.yaml` comment "the `Stale` state … is a later story" suggests a story was intended.
- Docs coverage. I traced the operator's README, AGENTS.md, website and user-facing docs request through `story:docs-vocabulary-readme-website` and the three docs stories, and found no gap. The remaining `docs/*.md` hits for "adapter" (`development.md`, `live-e2e.md`, `verification.md`, `fluxplane-plugin-parity.md`) use it for the implementation or are dated records. `apps/connectors/spec/README.md:4` lists the `adapters` command group, which `story:connections-reconnect-verb` renames. It is contributor-facing, so I did not file it.
- `story:connector-config-dual-read` takes the design's open question 3 as "no" while the epic's Open section lists it as open. I judged this a recorded, reversible assumption and not reach.
- Out of my lane:
  - `contracts/cli/v1alpha1/semantics.md` and `ess/domains/cli.yaml` are edited by three stories at once, which belongs to parallel-safety.
  - `story:cli-text-without-adapter` and `story:docs-vocabulary-readme-website` both regenerate `website/docs/reference/cli.md`, which belongs to parallel-safety and design.
  - Whether the acceptances are checkable belongs to the acceptance critic.

```findings
- file: .engineering/planning/epic/connector-vocabulary.md
  line: 14
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"the CLI, the configuration file, the ESS model and the guides use the vocabulary\" promises an ESS model change, but no drafted story's scope or change claims the Provider and Connector entities or the Stale transitions on connectors.auth_bindings.Connection (ess/domains/auth_bindings.yaml is in no story's scope), so each story cites that model as input being drafted with no artifact to depend on; connection-stale-state would most naturally take the auth_bindings.yaml lifecycle change, and the Provider/Connector/Instance entities need a story or a stated out-of-band decision"
```
