---
format: aep.planning-md/3
id: review-result:connector-vocabulary-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, connector-vocabulary stories
relations:
- reviews: story:catalog-connector-declarations
- reviews: story:cli-connector-instance-selectors
- reviews: story:cli-text-without-adapter
- reviews: story:connections-reconnect-verb
- reviews: story:connector-config-dual-read
- reviews: story:connector-instance-derived-fields
- reviews: story:connection-stale-state
- reviews: story:setup-migrate-adapter-entries
- reviews: story:docs-catalog-guides-vocabulary
- reviews: story:docs-google-one-instance-per-account
- reviews: story:docs-local-guides-vocabulary
- reviews: story:docs-vocabulary-readme-website
revision: 1
---
needs-revision

story:catalog-connector-declarations — the acceptance joins two independent outcomes with "and" (the bootstrap requests both Gmail scopes; a derived configuration prints the same revision as the legacy JSON), so one can pass while the other fails — .engineering/planning/story/catalog-connector-declarations.md:71
story:cli-connector-instance-selectors — the acceptance joins three independent outcomes (equal connection ids, exactly one deprecation line, `adapter`/`connector`/`instance` in both JSON results) and checks only `connections list` while the title says "Every command" — .engineering/planning/story/cli-connector-instance-selectors.md:78
story:cli-text-without-adapter — "every rendered error in the CLI conformance fixtures" names no fixture set, and `contracts/cli/v1alpha1/fixtures` holds 117 case-insensitive "adapter" matches, including wire fields the Change keeps, so two reviewers cannot get the same answer — .engineering/planning/story/cli-text-without-adapter.md:71
story:connections-reconnect-verb — the acceptance joins three independent outcomes (reconnect publishes, repair is an alias with one deprecation line, `instances list` equals `adapters list`), and the `adapters` alias's deprecation line is checked nowhere — .engineering/planning/story/connections-reconnect-verb.md:68
story:connector-config-dual-read — the acceptance joins two independent outcomes with "and" (equal id, revision and selection digest; a duplicate-id file refused), and says nothing of `default_instance` or its refusal when both defaults are set — .engineering/planning/story/connector-config-dual-read.md:73
story:docs-catalog-guides-vocabulary — the acceptance says "no native JSON, revision or executable pin on the setup path" for every guide in scope, but the Change says Slack's guide "shows the pinned instance form" and `docs/catalog-slack.md` is in scope, so it cannot be satisfied — .engineering/planning/story/docs-catalog-guides-vocabulary.md:64 (contradicts :49)
story:docs-catalog-guides-vocabulary — the acceptance joins two independent outcomes with "and" (setup is connector plus instance; no guide uses "adapter" for the configured thing) — .engineering/planning/story/docs-catalog-guides-vocabulary.md:64
story:docs-google-one-instance-per-account — the acceptance joins two independent outcomes with "and" (each guide configures one instance per account; `catalog-google-oauth.md` states a personal-use app can be published without verification), and "each Google guide" is ambiguous about whether `catalog-google-oauth.md` counts — .engineering/planning/story/docs-google-one-instance-per-account.md:64
story:docs-local-guides-vocabulary — the acceptance joins two independent outcomes with "and" (no file uses "adapter" for the configured thing; each links to the `README.md` vocabulary section) — .engineering/planning/story/docs-local-guides-vocabulary.md:85
story:docs-vocabulary-readme-website — the acceptance joins four independent outcomes (README defines six terms; AGENTS.md and every hand-written page link to it; none uses "adapter" for the configured thing; `generate --check` and the website build pass) — .engineering/planning/story/docs-vocabulary-readme-website.md:81

**What I read:** 12 of 12 ids (whole bodies), with `aep plan artifact show <id>` for each, `aep plan artifact kinds` and `aep plan artifact lifecycle story`, plus `epic:connector-vocabulary`. Tree checks: `git grep` for the `generate --check` flag, `approvals policy-status`, `--print-local-bootstrap`, the `oauth2_refresh.rs` `invalid_grant` fixture, and the CLI conformance fixtures. I did not open `ess/`.

**What I could not establish:**
- Whether "adapter … for the configured thing" in the three docs stories and the epic can be checked mechanically. No grep allowlist is named, so each remaining occurrence is a judgement call. This is an unease, not a finding, because the epic uses the same phrase.
- Approved on acceptance: story:connector-instance-derived-fields, story:connection-stale-state, story:setup-migrate-adapter-entries. Each states one observable journey with a before/after.
- Out of my lane, and none of it set the verdict: the "ESS model … being drafted" relations cited but not yet present (design/scope); story:docs-catalog-guides-vocabulary and the other docs stories depending on story:docs-vocabulary-readme-website, which in turn depends on all the CLI stories (parallel-safety/design).

```findings
- file: .engineering/planning/story/catalog-connector-declarations.md
  line: 71
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes with \"and\" (the bootstrap requests both Gmail scopes; a derived configuration prints the same revision as the legacy JSON), so one can pass while the other fails"
- file: .engineering/planning/story/cli-connector-instance-selectors.md
  line: 78
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins three independent outcomes (equal connection ids, exactly one deprecation line, `adapter`/`connector`/`instance` in both JSON results) and checks only `connections list` while the title says \"Every command\""
- file: .engineering/planning/story/cli-text-without-adapter.md
  line: 71
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "\"every rendered error in the CLI conformance fixtures\" names no fixture set, and `contracts/cli/v1alpha1/fixtures` holds 117 case-insensitive \"adapter\" matches, including wire fields the Change keeps, so two reviewers cannot get the same answer"
- file: .engineering/planning/story/connections-reconnect-verb.md
  line: 68
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins three independent outcomes (reconnect publishes, repair is an alias with one deprecation line, `instances list` equals `adapters list`), and the `adapters` alias's deprecation line is checked nowhere"
- file: .engineering/planning/story/connector-config-dual-read.md
  line: 73
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes with \"and\" (equal id, revision and selection digest; a duplicate-id file refused), and says nothing of `default_instance` or its refusal when both defaults are set"
- file: .engineering/planning/story/docs-catalog-guides-vocabulary.md
  line: 64
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says \"no native JSON, revision or executable pin on the setup path\" for every guide in scope, but the Change (line 49) says Slack's guide \"shows the pinned instance form\" and `docs/catalog-slack.md` is in scope, so it cannot be satisfied"
- file: .engineering/planning/story/docs-catalog-guides-vocabulary.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes with \"and\" (setup is connector plus instance; no guide uses \"adapter\" for the configured thing)"
- file: .engineering/planning/story/docs-google-one-instance-per-account.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes with \"and\" (each guide configures one instance per account; `catalog-google-oauth.md` states a personal-use app can be published without verification), and \"each Google guide\" is ambiguous about whether `catalog-google-oauth.md` counts"
- file: .engineering/planning/story/docs-local-guides-vocabulary.md
  line: 85
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins two independent outcomes with \"and\" (no file uses \"adapter\" for the configured thing; each links to the `README.md` vocabulary section)"
- file: .engineering/planning/story/docs-vocabulary-readme-website.md
  line: 81
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance joins four independent outcomes (README defines six terms; AGENTS.md and every hand-written page link to it; none uses \"adapter\" for the configured thing; `generate --check` and the website build pass)"
```
