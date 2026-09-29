# Wave 2026-09-29 (second): metadata replay, GitLab repository reads, setup init format, Jira reads

Skill version: aep implementing 0.18.0. Approved by the operator ("dispatch next wave"); a release
(0.16.0) follows a green close.

## Units

| story | serves | scope | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|---|
| story:metadata-open-is-not-a-full-replay | vision:independent-contract-adapters | inferred | aep:implementor, aep:adversary, aep:security-reviewer | impl/metadata-open-is-not-a-full-replay | `<managed-trees>`/wave0929b-replay | `<tree>`/target | `<wave-scratch>`/replay | merged (cccc8ebea), target deleted |
| story:catalog-gitlab-repository-reads | vision:independent-contract-adapters | inferred | aep:implementor, aep:adversary | impl/catalog-gitlab-repository-reads | `<managed-trees>`/wave0929b-gitlab | `<tree>`/target | `<wave-scratch>`/gitlab | merged (987ab2bd1), target deleted |
| story:setup-init-writes-current-config-format | vision:independent-contract-adapters | inferred | aep:implementor, aep:adversary | impl/setup-init-writes-current-config-format | `<managed-trees>`/wave0929b-setup | `<tree>`/target | `<wave-scratch>`/setup | merged (fd76f70e7), target deleted |
| story:catalog-jira-cloud-reads | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/catalog-jira-cloud-reads | `<managed-trees>`/wave0929b-jira | `<tree>`/target | `<wave-scratch>`/jira | merged (361d4dbc7), target deleted |
| story:catalog-confluence-reads | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/catalog-confluence-reads | `<managed-trees>`/wave0929b-confluence | `<tree>`/target | `<wave-scratch>`/confluence | green, held: decision-blocker:confluence-openapi-redistribution (not merged, worktree kept) |
| story:catalog-selection-parameter-bounds | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/catalog-selection-parameter-bounds | `<managed-trees>`/wave0929b-bounds | `<tree>`/target | `<wave-scratch>`/bounds | merged (a778e179a), target deleted |
| story:configuration-refusal-names-the-entry | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/configuration-refusal-names-the-entry | `<managed-trees>`/wave0929b-refusal | `<tree>`/target | `<wave-scratch>`/refusal | merged (6902cfef3), target deleted |
| story:provider-refusal-reports-dispatch-stage | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/provider-refusal-reports-dispatch-stage | `<managed-trees>`/wave0929b-stage | `<tree>`/target | `<wave-scratch>`/stage | merged (b69ead46c), target deleted |

Integration branch: `wave/20260929-replay-gitlab-setup-jira` off `main` 41d2d9222.

Build directories sit inside each worktree (`target/`): a build under `/dev/shm` cannot run the
host's child-process tests, which refuse a world-writable parent directory.

## `aep plan artifact waves --kind story --status active`

```
wave 1
  story:catalog-gitlab-repository-reads (inferred)
  story:catalog-jira-cloud-reads
  story:metadata-open-is-not-a-full-replay (inferred)
  story:setup-init-writes-current-config-format (inferred)
1 wave(s), 0 collision(s), 0 unassessed
```

## Pre-flight

- `/` had 18G free; one unit build measured 2.4–3.1G in the previous wave. Three units build at
  once; the Jira unit starts when a finished unit's build directory is deleted.
- `wt-d90bbee1cfa0` is an older retained worktree (staged, unpublished index content, already
  archived); not part of this wave, left in place.

## Decisions taken by the coordinator

- metadata: the counter sits in `er::complete_snapshot`; the count test is in-crate; state kept
  across opens must catch up on appends by other handles; the persist receipt check reads per subject.
- GitLab: paging stops on a page shorter than `per_page` (no response headers reach the caller);
  fixture bodies compare as equal JSON.
- setup init: writes `connectors-local/2`; the named refusal moved to
  story:configuration-refusal-names-the-entry.
- Jira: fixture bodies compare as equal JSON, as for GitLab.

## Commits approval authorises

One commit per unit (4), the merges into the integration branch, the closing planning-store commit,
the merge into `main` through a pull request once the single wave gate is green, then the 0.16.0
release through the repository's release process.

## Extension (operator: "dispatch /aep:wave", while the replay unit was still running)

Three stories disjoint from the running replay unit join this wave: catalog-confluence-reads
(depends on Jira, merged here), catalog-selection-parameter-bounds and
configuration-refusal-names-the-entry. owner-idle-exit, configuration-change-error and
revision-conflict-wire-code stay out: they share owner and registry files with the replay unit.
Confluence: the pipeline takes several pinned sources for one provider (v1 search + v2 pages).
The commit grant covers these units the same way (one commit each, the merges).
- Added 2026-09-29 on the operator's request ("pipelines cannot be read"): provider-refusal-reports-dispatch-stage. The pipeline read works; GitLab's 403 on one project was reported as a connectors admission refusal.

## Close

- Gate: `cargo run -p connectors-build -- gate` at `b69ead46c`, `CONNECTORS_ESS` = ESS 0.40.0:
  `gate: all checks passed`, exit 0; local metadata authority conformance 285 scenarios,
  24 synthesis refusals.
- Implemented: metadata-open-is-not-a-full-replay, catalog-gitlab-repository-reads,
  setup-init-writes-current-config-format, catalog-jira-cloud-reads,
  catalog-selection-parameter-bounds, configuration-refusal-names-the-entry,
  provider-refusal-reports-dispatch-stage. Scopes rewritten from each unit commit (all cited).
- Held: catalog-confluence-reads, on decision-blocker:confluence-openapi-redistribution; its tree
  `wave0929b-confluence` is archived, not merged.
- Filed during the wave: catalog-selection-parameter-bounds (done here),
  configuration-refusal-names-the-entry (done here), metadata-invoke-cost-flat-in-store-size,
  provider-refusal-reports-dispatch-stage (done here), provider-refusal-stage-for-writes-and-timeouts.
