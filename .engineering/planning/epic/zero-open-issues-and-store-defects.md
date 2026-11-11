---
format: aep.planning-md/3
id: epic:zero-open-issues-and-store-defects
kind: epic
status: active
title: Zero open GitHub issues and every defect story in the store fixed, in one pull request
summary: 'Operator goal 2026-11-09: 9 open issues and 56 defect stories, delivered by one pull request'
relations:
- serves: vision:independent-contract-adapters
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:57:46Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:57:46Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
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

## Single defect wave (in progress)

Approved by `approval-record:zero-issues-single-defect-wave` (non-interactive), which replaces
`approval-record:zero-issues-wave-b`. One wave, integration branch `wave/zero-issues-b` off origin/main
`23b9f1bc4`, one pull request. 48 stories: the 46 active stories of this epic and the two wave A
from-finding stories. `story:macos-release-provider-starts-through-the-private-copy` stays behind
`hardware-blocker:macos-adopter-provider-sigkill`; 47 are implemented.

Lanes follow the derived waves of `aep plan artifact waves --kind story --status active` at
`7f42c789b`: a story of a later derived wave starts only after the stories it collides with have merged.
At most five units run at once. Each unit: branch `impl/<story name>`, a managed worktree with the id
listed below, the shared build directory of the wave, a sonnet implementor, an opus adversary on guard,
emitted-bytes and auth or custody units, merge into the integration branch, push.

- Lane 1 (derived wave 1): the 29 implementable stories of derived wave 1, then
  `story:slack-file-read-checks-exact-path` and `story:tavily-crawl-cut-short-is-not-complete`.
- Lane 2 (derived wave 2): `story:catalog-validates-enum-and-date-time`,
  `story:clock-quorum-windows-unreachable-flake`, `story:definition-reader-refuses-what-the-feed-engine-refuses`,
  `story:exec-plugin-busy-reaches-the-operator`, `story:lease-early-recollection-relaunch-margin`,
  `story:owner-errors-are-readable`, `story:owner-path-tests-hold-under-load`,
  `story:owner-recovers-after-failed-acquisition`, `story:required-body-key-refuses-null`.
- Lane 3: `story:configuration-change-error`, `story:required-parameter-refuses-empty`,
  `story:windows-clock-check-bracket-flake`.
- Lane 4: `story:definition-connect-begin-carries-selection`. Lane 5: `story:owner-cancels-a-bounded-read`.
  Lane 6: `story:owner-invoke-frame-effects-boundary`.
- Joined by `approval-record:zero-issues-review-s2-join-the-wave`: the four S2 findings of the code
  review at `23b9f1bc4`, each red first: `story:mcp-tool-deadline-keeps-the-server` (after the stories
  touching the MCP server), `story:sql-wrapper-resolves-pg-catalog-first` (after
  `story:sql-length-units`), `story:reconnect-retained-bound-matches-refresh`,
  `story:read-use-bound-counts-live-reads`. The 21 S3 findings are draft stories tagged `from-review`,
  outside this wave.
- Joined by `approval-record:zero-issues-release-image-joins-the-wave`:
  `story:release-image-platform-check-tolerates-padding`, red first over captured inspect output.

Commits the approval authorises: one commit per unit on its branch, a correction commit per adversary
round, one merge per unit into `wave/zero-issues-b`, store commits on the integration branch, the
closing store commit, and the squash merge of the pull request. No tag, no release.

### Units

| Unit | Worktree id | Stage |
|---|---|---|
| `story:adapter-private-protocol-refusal-named` | `cb-protocol` | merged, tree finished (merge `1bfda4b51`) |
| `story:catalog-guard-typed-comparison` | `cb-guard-typed` | merged, tree finished (merge `18ae97dd1`) |
| `story:legacy-alias-sharing-an-instance-name` | `cb-alias` | merged, tree finished (merge `7be17a9aa`) |
| `story:sql-length-units` | `cb-sql-units` | merged, tree finished (merge `470e149db`) |
| `story:slack-move-names-four-writes` | `cb-slack-move` | merged, tree finished (merge `548cdd93d`) |
| `story:closed-body-load-refuses-ambiguous-pointers` | `cb-closed-body` | merged, tree finished (merge `a9d5c0270`) |
| `story:definition-import-merges-shared-auth` | `cb-import-auth` | merged, tree finished (merge `de978726b`) |
| `story:definition-refusal-keeps-deprecations` | `cb-refusal-depr` | not reproducible, no change, tree finished |
| `story:readme-back-under-its-size-limit` | `cb-readme` | merged, tree finished (merge `7ae9054fa`) |
| `story:mcp-projection-cases-carry-read-or-mutating` | `cb-mcp-cases` | merged, tree finished (merge `6489d43af`) |
| `story:sql-wrapper-resolves-pg-catalog-first` | `cb-sql-pgcat` | merged, tree finished (merge `9c4fdfd75`) |
| `story:reconnect-retained-bound-matches-refresh` | `cb-reconnect` | merged, tree finished (merge `63e774d11`) |
| `story:read-use-bound-counts-live-reads` | `cb-read-use` | merged, tree finished (merge `7d72ba5ec`) |
| `story:slack-file-read-checks-exact-path` | `cb-slack-path` | merged, tree finished (merge `62d0e3c18`) |
| `story:mcp-tool-deadline-keeps-the-server` | `cb-mcp-deadline` | merged, tree finished (merge `25cf2b02d`) |
| `story:catalog-oversized-body-one-classification` | `cb-oversized` | merged, tree finished (merge `09d1e03f6`) |
| `story:pool-idle-checks-wait-outside-the-pool-lock` | `cb-pool-idle` | merged, tree finished (merge `c3b1590c6`) |
| `story:owner-bounded-generic-read-inline-schemas` | `cb-inline-schemas` | merged, tree finished (merge `66a7c9bca`) |
| `story:metadata-conformance-emit-manifest-format` | `cb-meta-manifest` | already fixed, pin test merged, tree finished |
| `story:live-acceptance-without-serve` | `cb-live-accept` | merged, tree finished |
| `story:oauth-browser-empty-connection-fixture` | `cb-oauth-fixture` | merged, tree finished |
| `story:release-image-platform-check-tolerates-padding` | `cb-release-image` | merged, tree finished |
| `story:upgrade-refusal-test-port-reuse` | `cb-upgrade-port` | merged, tree finished |
| `story:tests-stop-the-owners-they-start` | `cb-stop-owners` | merged, tree finished (merge `dbdf32e26`) |
| `story:custody-keeps-out-of-the-session-keyring` | `cb-custody-keyring` | merged, tree finished (merge `7988b0427`) |
| `story:connection-lease-tests-pin-the-window` | `cb-lease-window` | merged, tree finished (merge `609da4250`) |
| `story:ignored-journeys-run-in-ci` | `cb-ignored-ci` | merged, tree finished (merge `0d27f9912`) |
| `story:authored-scenarios-known-failing` | `cb-authored-scen` | merged, tree finished (merge `32b7edf45`) |
| `story:cli-connect-admission-timeout-under-load` | `cb-cli-admission` | merged, tree finished (merge `b28bcf031`) |
| `story:clock-quorum-test-tolerates-a-slow-loopback-source` | `cb-clock-loopback` | merged, tree finished (merge `5f5d1e108`) |
| `story:docs-provider-pages-shared-guides` | `cb-docs-guides` | merged, tree finished (merge `bd8343ae3`) |
| `story:tavily-crawl-cut-short-is-not-complete` | `cb-tavily-crawl` | merged-partial (merge `5d76bbad9`) |
| `story:generated-output-regenerates-in-a-fresh-checkout` | `cb-fresh-regen` | merged, tree finished (merge `8b8a729d8`) |
| `story:legacy-alias-sharing-an-instance-name` | `cb-alias-contract` | merged, tree finished (merge `7be17a9aa`) |
| `story:docs-specification-sidebar` | `cb-docs-sidebar` | merged, tree finished (merge `e5e891798`) |
| `story:gmail-attachment-read-flaky-under-load` | `cb-gmail-attach` | not-reproduced |
| `story:host-gate-load-sensitivity-w3` | `cb-gate-w3` | implementing |
| `story:mcp-reply-after-hangup-owner-latency-under-load` | `cb-mcp-hangup` | merged, tree finished (merge `56067e56f`) |
| `story:catalog-validates-enum-and-date-time` | `cb-enum-date` | merged, tree finished (merge `e728524d0`) |
| `story:owner-recovers-after-failed-acquisition` | `cb-owner-recover` | merged, tree finished (merge `d3bd6fcd4`) |
| `story:owner-errors-are-readable` | `cb-owner-errors` | merged, tree finished (merge `13367cb99`) |
| `story:owner-path-tests-hold-under-load` | `cb-owner-paths` | merged, tree finished (merge `96212520e`) |
| `story:clock-quorum-windows-unreachable-flake` | `cb-clock-win` | implementing |
| `story:configuration-change-error` | `cb-config-change` | implementing |
| `story:lease-early-recollection-relaunch-margin` | `cb-lease-margin` | implementing |
