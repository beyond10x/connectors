---
format: aep.planning-md/3
id: specification:wave-20261008a-cli-audit
kind: specification
status: approved
title: 'Wave 20261008a: CLI audit findings and the expiry batch bound'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:explicit-service-commands-help-and-completion
- informed_by: story:pending-connection-refusal-names-its-remedy
- informed_by: story:aep-pin-0690
- informed_by: story:lift-expiry-batch-bound
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-08T07:54:09Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-08T07:54:09Z", actor: "human:timo", revision: 3}
---
## Selection

| unit | story | surface |
|---|---|---|
| U1 | story:explicit-service-commands-help-and-completion | `apps/connectors` |
| U2 | story:pending-connection-refusal-names-its-remedy | `crates/connectors-host` registry/owner |
| U3 | story:aep-pin-0690 | `crates/connectors-build`, development guide |
| U4 | story:lift-expiry-batch-bound | `crates/connectors-host` metadata/er.rs |

Integration branch `wave/20261008a`; one pull request. U2 and U4 share a crate, not a file.

## Audit re-run on 0.33.0 (2026-10-08 07:49–07:52Z)

The read-only CLI audit of 2026-10-07 ran on 0.31.0. Its failing checks were re-run with 0.33.0 against
the same local state, same arguments; outputs stay outside the repository.

| finding (0.31.0) | 0.33.0 | disposition |
|---|---|---|
| admission `timeout`: Jira changelog 42/46 s, GitLab issues 36 s and MR list 49 s | after revalidation: 1.3/1.4 s, 1.0 s, 1.1 s, all ok | does not reproduce; the store-cost fix of 0.33.0 is the likely cause (inferred) |
| admission `timeout`: Tavily crawl 37 s | 10.8/11.3 s ok (input differs: the original was not retained) | does not reproduce |
| revalidation `outcome_unknown` at publication: Slack ×2, Zendesk | 4.1–4.7 s, ok | does not reproduce |
| Confluence `lifecycle_conflict` on reads and `approvals key-status` | reproduces; revalidation `not_granted` (`create_connection`) | U2 |
| Slack advertises 0 operations | no Slack operation is bound yet | parity waves, not a defect |
| help dispatcher and completion omit `describe`/`invoke`/`serve` | reproduces | U1 |
| parse refusals carry only `cli_parse` | reproduces; the `ess-cli/1` presentation defines `{}` data | waits on `ess/story:cli-parse-refusal-names-the-declared-arguments` |
| installed skill documents family-only discovery | the CLI requires `--adapter` by contract | the skill is corrected in its own repository |

Before the re-run every saved connection was `pending` with lapsed validation; reads first refused
`not_granted` (`revalidate_connection`) and succeeded after one revalidation.

## Results

## Results

| unit | commit | package tests |
|---|---|---|
| U1 help and completion | `4414bb3234`, merged `60cc6dae82` | connectors 127 passed, 0 failed, 6 ignored; `cli --check` current |
| U3 AEP 0.69.0 | `e3d7c61514` | connectors-build `aep_toolchain` 2 passed; clippy, fmt clean |
| U4 expiry bound 32 -> 128 | `a8abcd7265`, merged `f0b7501817` | connectors-host lib 345 passed, 0 failed, 33 ignored; clippy clean |

- U1: `help COMMAND` routes only as the first word; `--output=json help describe` keeps refusing
  `cli_parse` (`adversary_legacy_edges.rs:98`). The ESS CLI binding applies its globals to every
  command, so the three compatibility commands stay outside the generated tree.
- U4, `first_owner_open_of_a_grown_store`, release, Entity Runtime 0.30.2, 2026-10-08, load about 21:

| store | bound | batches | largest batch | catch-up |
|---|---|---|---|---|
| 1,201 events (396 due) | none | 1 | 4.0 s | 6.5 s |
| 1,201 events | 128 | 4 | 1.4 s | 5.8 s |
| 6,000 events | none | 1 | 14.4 s | 20.9 s |
| 6,000 events | 32 (3 runs) | 32 | 0.5–1.3 s | 14.7–17.6 s |
| 6,000 events | 128 (4 runs) | 8 | 1.5–2.5 s | 13.2–20.0 s |

  An unbounded batch still grows with the backlog (14.4 s at 6,000 events), so the bound is raised,
  not removed. Every store this host opens is SQLite.
