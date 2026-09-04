# Wave: CLI output readability — 2026-09-04

Skill version 0.6.1 (`.claude-plugin/plugin.json`). Status: **stage 2, approved and running.**

## Unit

| | |
|---|---|
| story | `story:informative-command-readability` |
| status | draft |
| scope | `crates/connectors-console/src/output.rs`, `crates/connectors-console/src/doctor.rs` — both **cited** |
| blast radius | one package, `connectors-console` |
| objective | O5, the generic agent platform — the catalogue a platform tenant configures connectors from |
| implementor | `adp:implementor` |
| adversary | `adp:adversary` |
| branch | _not created_ |
| worktree | _not created_ |
| build directory | _not created_ |
| scratch root | _not created_ |
| stage | proposed |

## Acceptance, from the artifact

`connectors doctor`, `providers` and `auth status` each render in `text` as one aligned row per
record with the severity or readiness of a row distinguishable without reading its text, no output
format drops a field the value carries, and the bytes of `-o json` and `-o yaml` are unchanged.

## Selection

Computed, not judged: `aep artifact waves --kind story --status draft`, binary `protocol 0.50.0`.

```
wave 1
  story:informative-command-readability
1 wave(s), 0 collision(s), 24 unassessed
```

Collisions: **none returned.**

Unassessed, verbatim, all 24:

```
story:adopt-token-response-metadata
story:auth-as-tool-result
story:connect-session-oauth-custody-in-personal-posture
story:contract-bundles-are-versioned-and-pinned
story:coverage-regains-its-second-direction
story:declarative-webhook-routing-grammar
story:deployment-catalogs-are-external-packs
story:deployment-declared-destination-aperture
story:header-name-rate-limit-retry
story:kubernetes-joins-the-catalog
story:m2-the-platform-skeleton-serves
story:m3-connect-a-provider-and-invoke-it
story:m4-events-reach-a-client-by-push-and-by-pull
story:mint-source-entries-from-the-mined-catalogs
story:one-composed-local-placement
story:one-connection-config-shape
story:per-service-verification-probes
story:production-credential-custody-closes
story:raw-proxy-is-break-glass-not-a-model-capability
story:satellite-federation-has-a-threat-modeled-contract
story:the-anthropic-api-key-arrives-through-a-connect-session
story:the-claims-journal-survives-a-full-life
story:the-cli-drives-a-hosted-connection
story:the-hosted-posture-connects-a-catalogued-provider
```

Those 24 arrived in tonight's backlog migration and carry no typed scope, because scope was recorded
only for stories that were `active` at the time. They are **unassessed, not safe**. They were not
scoped for this wave because the operator named the unit rather than asking for a selection; scoping
them is the prerequisite for any future wave chosen by the store rather than by name.

N is 1. That is a wave.

## Pre-flight — PASSES

| check | value | verdict |
|---|---|---|
| working tree clean | only this page, untracked | pass |
| free disk | 84 G, 90% used | pass |
| previous waves' trees | none of this wave's; four unrelated trees from other sessions | pass |
| build cache | `~/.cache/sccache` was deleted in tonight's disk incident and rebuilds cold | note |
| model budget | not stated; N is 1 so the default of 4 does not bind | note |
| `AGENTS.md` | read; it declares no branch prefixes, so the skill's defaults are used | pass |

`web/` was 242 MB of `node_modules`, VitePress build output and one generated `catalog.json`, with
no hand-written source and no `package.json`. The operator confirmed it held nothing and directed
its removal; it was deleted rather than overridden, so the dirty-tree refusal is cleared rather
than waived. It was the sole cause of
`json_governance::every_repository_json_is_classified_and_valid` failing locally.

## Commits this wave's approval would authorise

One unit commit on `impl/informative-command-readability`; its merge into the integration branch;
one closing store commit recording evidence and moving the story; and the merge of the integration
branch into `main`. **Nothing else** — no push, no tag, no release, no second wave.
