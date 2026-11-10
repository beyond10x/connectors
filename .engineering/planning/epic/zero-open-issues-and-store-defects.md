---
format: aep.planning-md/3
id: epic:zero-open-issues-and-store-defects
kind: epic
status: active
title: Zero open GitHub issues and every defect story in the store fixed, in one pull request
summary: 'Operator goal 2026-11-09: 9 open issues and 56 defect stories, delivered by one pull request'
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:57:46Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:57:46Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

The repository has no open GitHub issue, and every open story in this store that describes a defect
is fixed, shown already fixed, or stopped by a typed blocker that names what is missing. Everything
is delivered by one pull request from `wave/zero-issues`.

## Source

Operator goal of 2026-11-09, verbatim: "zero remaining issues on github, all bugs fixed from planning
store". Method, verbatim: "use /planning and /wave and /worktree:managing-worktrees - but in the end
provide 1 single PR to gh. make sure all are below 1 big epic for integration via bottoken from
./atlas". The run is non-interactive; each bypassed stop is an `approval-record` tagged
`non-interactive`.

## GitHub issues (9 open on 2026-11-09, `gh issue list -R beyond10x/connectors`)

| Issue | Carried by | State on `origin/main` `bd338daa0` |
|---|---|---|
| #180 | `story:approval-issue-slack-bot-post` (implemented) | fixed; proof re-run in this epic |
| #181 | `story:connection-next-actions-no-cycle`; Confluence half `story:confluence-token-kind-binding` (implemented) | the next-action loop is open |
| #183 | `story:ci-timing-flakes-20261018` (implemented) | fixed; proof from the macOS runs |
| #229 | `story:provider-programs-run-on-macos` (active) | open: no Grafana connect through the CLI runs on macOS CI |
| #230 | `story:catalog-provider-failure-is-named` (implemented) | fixed; proof re-run in this epic |
| #231 | `story:provider-programs-run-on-macos` | delivered in 0.65.0; proof from the release assets |
| #232 | `story:provider-programs-run-on-macos` | delivered in 0.65.0; proof from the macOS `programs` job |
| #233 | `story:program-start-failure-reason` (implemented) | fixed; proof re-run in this epic |
| #234 | `story:kubernetes-exec-plugin-on-macos` (implemented) | fixed; proof from the macOS `programs` job |

## Defects in the store

Found by reading all 150 open stories and tasks on `bd338daa0` (a read-only agent pass, 2026-11-09;
its classes are a reading, not a measurement): 34 describe a product defect, 16 a test defect, 6 a
wrong fact in documentation or a contract; 94 are features, parity items, refactors or decisions and
are not part of this epic. The store declares a `correctness-blocker` ladder and holds no artifact of
that kind. Every one of the 56 carries a `decomposes` edge to this epic.

## Out of scope

Releases (no tag, no version bump, no publication). The 94 open stories that describe no defect.

## Wave plan

Derived once, 2026-11-09, by `aep plan artifact waves --kind story --status active --format json` on the store of this epic (typed scope written for 42 stories, most `inferred`; 1 unassessed: `story:sustained-live-read-loop`, not part of this epic). Collisions printed: 65; cycles: 0. Units run at most two at a time, each in its own managed worktree forked from `wave/zero-issues`, and merge into it; the single pull request is opened from that branch.

`story:macos-release-provider-starts-through-the-private-copy` leaves every wave: `hardware-blocker:macos-adopter-provider-sigkill` is open.

### Wave 1

- `story:adapter-private-protocol-refusal-named`
- `story:authored-scenarios-known-failing`
- `story:bundle-index-refuses-ambiguous-rows`
- `story:catalog-guard-optional-preflight-values`
- `story:catalog-oversized-body-one-classification`
- `story:cli-connect-admission-timeout-under-load`
- `story:clock-quorum-test-tolerates-a-slow-loopback-source`
- `story:closed-body-load-refuses-ambiguous-pointers`
- `story:connection-lease-tests-pin-the-window`
- `story:custody-keeps-out-of-the-session-keyring`
- `story:definition-import-merges-shared-auth`
- `story:definition-refusal-keeps-deprecations`
- `story:docs-provider-pages-shared-guides`
- `story:docs-specification-sidebar`
- `story:endpoint-publish-holds-one-writer`
- `story:feed-cursor-unadvanced-is-unavailable`
- `story:generated-output-regenerates-in-a-fresh-checkout`
- `story:gmail-attachment-read-flaky-under-load`
- `story:host-gate-load-sensitivity-w3`
- `story:http-empty-query-no-trailing-question-mark`
- `story:ignored-journeys-run-in-ci`
- `story:legacy-alias-sharing-an-instance-name`
- `story:live-acceptance-without-serve`
- `story:mcp-projection-cases-carry-read-or-mutating`
- `story:mcp-reply-after-hangup-owner-latency-under-load`
- `story:metadata-conformance-emit-manifest-format`
- `story:mysql-query-read-provenance`
- `story:oauth-browser-empty-connection-fixture`
- `story:owner-bounded-generic-read-inline-schemas`
- `story:owner-read-answer-size-envelope`
- `story:pool-idle-checks-wait-outside-the-pool-lock`
- `story:provider-programs-run-on-macos`
- `story:readme-back-under-its-size-limit`
- `story:slack-move-names-four-writes`
- `story:tests-stop-the-owners-they-start`
- `story:upgrade-refusal-test-port-reuse`
- `story:websearch-contract-states-host-deadline`

### Wave 2

- `story:catalog-guard-typed-comparison`
- `story:clock-quorum-windows-unreachable-flake`
- `story:configuration-change-error`
- `story:definition-reader-refuses-what-the-feed-engine-refuses`
- `story:exec-plugin-busy-reaches-the-operator`
- `story:lease-early-recollection-relaunch-margin`
- `story:required-body-key-refuses-null`
- `story:sql-length-units`

### Wave 3

- `story:catalog-validates-enum-and-date-time`
- `story:connection-next-actions-no-cycle`
- `story:owner-path-tests-hold-under-load`
- `story:windows-clock-check-bracket-flake`

### Wave 4

- `story:definition-connect-begin-carries-selection`
- `story:required-parameter-refuses-empty`

### Wave 5

- `story:owner-cancels-a-bounded-read`
- `story:owner-errors-are-readable`

### Wave 6

- `story:owner-invoke-frame-effects-boundary`

### Wave 7

- `story:owner-recovers-after-failed-acquisition`

### Delivery

Nothing delivered yet.

## Wave A (delivered 2026-11-10)

Approved by `approval-record:zero-issues-wave-a` (non-interactive). Ten stories, one managed worktree
each, an adversary pass each (two for four of them), merged into `wave/zero-issues`:
`story:http-empty-query-no-trailing-question-mark`, `story:bundle-index-refuses-ambiguous-rows`,
`story:provider-programs-run-on-macos` (#229), `story:endpoint-publish-holds-one-writer`,
`story:connection-next-actions-no-cycle` (#181), `story:catalog-guard-optional-preflight-values`,
`story:feed-cursor-unadvanced-is-unavailable`, `story:mysql-query-read-provenance`,
`story:websearch-contract-states-host-deadline`, `story:owner-read-answer-size-envelope`.

Nine are implemented on the local repository gate at `74030787c`. `story:provider-programs-run-on-macos`
stays active until the pull request's macOS jobs prove it. Integration fixes the gate found: the
metadata Entity Runtime definitions and the operation-type digests regenerated after two `cli.yaml`
changes, two `cli.yaml` line citations re-pointed, and the Google Drive configuration revisions
re-pinned. `local_mcp_server` failed under load in two gate runs; an implementor measured 3 of 22
failing with the owner-read change and 3 of 23 without it, the known
`story:mcp-reply-after-hangup-owner-latency-under-load`.

From-finding drafts: `story:slack-file-read-checks-exact-path`, `story:tavily-crawl-cut-short-is-not-complete`.
