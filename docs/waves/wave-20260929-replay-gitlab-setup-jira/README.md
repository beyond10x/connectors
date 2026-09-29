# Wave 2026-09-29 (second): metadata replay, GitLab repository reads, setup init format, Jira reads

Skill version: aep implementing 0.18.0. Approved by the operator ("dispatch next wave"); a release
(0.16.0) follows a green close.

## Units

| story | serves | scope | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|---|
| story:metadata-open-is-not-a-full-replay | vision:independent-contract-adapters | inferred | aep:implementor, aep:adversary, aep:security-reviewer | impl/metadata-open-is-not-a-full-replay | `<managed-trees>`/wave0929b-replay | `<tree>`/target | `<wave-scratch>`/replay | planned |
| story:catalog-gitlab-repository-reads | vision:independent-contract-adapters | inferred | aep:implementor, aep:adversary | impl/catalog-gitlab-repository-reads | `<managed-trees>`/wave0929b-gitlab | `<tree>`/target | `<wave-scratch>`/gitlab | planned |
| story:setup-init-writes-current-config-format | vision:independent-contract-adapters | inferred | aep:implementor, aep:adversary | impl/setup-init-writes-current-config-format | `<managed-trees>`/wave0929b-setup | `<tree>`/target | `<wave-scratch>`/setup | planned |
| story:catalog-jira-cloud-reads | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/catalog-jira-cloud-reads | `<managed-trees>`/wave0929b-jira | `<tree>`/target | `<wave-scratch>`/jira | planned, starts when a slot frees |

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
