---
format: aep.planning-md/3
id: epic:fluxplane-plugin-parity
kind: epic
status: draft
title: Connectors replaces fluxplane-plugin in agent sessions
relations:
- serves: vision:independent-contract-adapters
- informed_by: specification:recent-agent-adapter-usage-20260909
revision: 3
---
## Outcome

The Connectors local CLI replaces `fluxplane-plugin` in the operator's Claude Code and Codex
sessions: every fluxplane operation those sessions use has a Connectors operation with the same
capability against the same provider.

## Evidence

`docs/fluxplane-plugin-parity.md` (2026-10-07): 301 declared fluxplane operations, 109 used,
6,356 calls since 2026-09-09. Covered 29 operations (2,659 calls), partial 23 (1,273), missing
249 (2,424). The used partial and missing operations form 25 units (3,788 calls with the mapped
undeclared names), planned as 7 waves after the feed-binding wave.

## Scope

One story per unit; a story's operations, calls, surface and planned wave are in its body and
in the parity page. Operations with 0 calls are not planned until a session uses them.

## Spec first

Each story models what it adds in this repository's ESS specification (the adapter's native
model, or the catalog provider's pinned OpenAPI source and selection), validates it with the
newest `ess`, regenerates, then implements. A provider with no model yet (Slack, Homer,
Alertmanager) gets its domain drafted and validated before the story is scheduled.

## Order

The cross-cutting reliability items come first, then the per-operation rows. They gate unattended use of every operation, which call-count order cannot reach: a downstream consumer's live test of 0.50.0 passed 15 of 71 operations (2026-10-14).

1. story:connection-lease-holds-unattended: a revalidated connection keeps serving reads; concurrent revalidations finish within the admission bound.
2. story:upgrade-moves-catalog-adapters: an upgraded CLI does not leave catalog adapters on the old provider.
3. story:slack-user-token-history-reads: Slack history on a user token.
4. story:kubernetes-eks-exec-auth-and-kinds: EKS exec authentication.
5. story:slack-ok-false-is-a-refusal: a Slack `ok: false` answer is read as a refusal.
6. story:refusals-name-the-field: `invalid_input` names the field.
7. story:sustained-live-read-loop: accepted by a 10-minute live read loop with no manual revalidate.

The per-operation rows follow in call-count order, highest first.

## Plan 2026-10-19

Written 2026-10-19 against `main` at `c15004d81` (release 0.54.0 in preparation). Inputs: the
recount of every gap unit against that commit and the old plugins' operation lists, both read-only;
the page itself is refreshed by story:parity-page-recount-20261019.

### Target

Parity, for an engineering team that starts using Connectors in its agent sessions, means three
things: every fluxplane operation with calls in the baseline has a Connectors operation of the same
capability; unattended use holds (a read keeps working across lapsed evidence, an upgrade, and a
refusal names its field); and demand from the team's own sessions is measured, so the next recount
is computed from the execution audit rather than counted by hand.

Baseline: `docs/fluxplane-plugin-parity.md` (301 declared operations, 109 used, 6,356 calls,
2026-09-09 to 2026-10-07). Recount 2026-10-18 against `c15004d81`: covered 112 operations /
5,957 calls, partial 6 / 5, missing 183 / 394. 17 of the 25 gap units are closed. 8 are open, 407
calls: U06 Jira writes 326, U17 Homer 33, U13 Slack upload and delete 31, U02 Slack single-user
reads 5, U03 `jira.issue.create_meta` 5, U19 port-forward 4, U24 Alertmanager 2, U25 Secret read
1. The old `flux` GitLab plugin declares the same 64 operations as the fluxplane one, so the page
already counts it; 19 of them (15 writes, 4 reads) are missing and all have 0 calls.

### Phases

Each phase is a wave of at most four stories on different surfaces; `aep plan artifact waves`
derives the set from the recorded scopes.

- **P0, announce-ready.** story:definition-format-write-members (the 0.54.0 release gate's one red
  seam), story:parity-page-recount-20261019, story:definition-authoring-guide,
  story:remove-unshipped-material-20261019 (the architecture review's scope A; #2 and #3 deferred to
  story:upstream-sources-gzip-only and story:projections-derived-in-temp). Outside this repository: the Claude Code plugin skill
  `connectors:integrating` (the agentplugins repository) describes `operations invoke
  --definition`, `--connector` and `--instance`, and `setup migrate`; no cached skill version up
  to 0.23.1 mentions `--definition` (checked 2026-10-18).
- **P1, unattended use holds** (the Order above, items 1, 2, 6, 7, plus measurement).
  story:sustained-live-read-loop (active), story:upgrade-moves-catalog-adapters,
  story:connection-follows-configuration-upgrade (proposed), story:refusals-name-the-field,
  story:ci-timing-flakes-20261018, story:operations-usage-report, story:operations-list-text-match.
- **P2, Jira writes** (326 of the 407 open calls). story:parity-jira-issue-create-and-comment
  first; then story:parity-jira-issue-writes and story:parity-jira-create-meta-per-type, which
  land on the same selection file and run after it. Beside them story:slack-single-user-lookups.
- **P3, a new auth scheme and composed writes.** story:auth-profile-login-exchange, then
  story:parity-homer-sip-reads. story:catalog-composed-write, then story:parity-slack-files.
- **P4, on first demand.** Scheduled when the usage report shows a `not_found` refusal naming one
  of their operations: story:parity-gitlab-issue-notes-approvals-ci-variables (0 baseline calls),
  story:parity-kubernetes-exec-portforward (port-forward, 4), story:parity-alertmanager-alerts (2),
  story:parity-kubernetes-secret-read (1; blocked by decision-blocker:kubernetes-secret-disclosure),
  and the zero-call plugin drafts (asterisk, aws, docker, duckduckgo, git, ollama, openai, sleep,
  system, vision), which keep the page's rule: not planned until used.

### Exit criteria

- E1: release 0.54.0 published with its four archives; P0 merged; the plugin skill updated.
- E2: after P2, 6,283 of the 6,356 baseline calls are covered (98.9 %); after P3, 6,347 (99.9 %).
- E3: the acceptance of story:sustained-live-read-loop and story:upgrade-moves-catalog-adapters
  passes on a released binary.
- E4: two consecutive weekly usage reports (story:operations-usage-report) show no `not_found`
  refusal naming an operation the old plugins declared.

### Decisions taken 2026-10-19

Each is recorded at its story and reversible there.

- D1: a Jira text body (comment, description) is an Atlassian Document Format document the caller
  sends as given; a plain-text wrapper needs array-index pointers in the closed-body engine and is
  story:plain-text-comment-bodies; Markdown conversion is not built until the usage report shows a
  session sending Markdown.
- D2: Slack `files.delete` ships as a plain selection before the upload; only the upload needs the
  composed write.
- D3: the Alertmanager route stays native, as story:parity-alertmanager-alerts decided; it is not
  re-routed through Grafana.
- D4: the 19 GitLab operations the old plugins declare with 0 calls are one story held for first
  demand, not one story per operation.

### Not covered

The fluxplane verbs `lookup`, `context build-all`, `blob put` and `index build` (20 calls) have no
Connectors equivalent because Connectors keeps no index and no blob store; `endpoint import` and
`endpoint save` (12 calls) have no verb that writes one instance into the configuration. They are
verbs, not provider operations, and get no story until a session asks for one. Guardrail 7 (a new
noun is modelled first) is met at story level: each story that introduces one names the ESS domain
it lands in and models it before implementation.
