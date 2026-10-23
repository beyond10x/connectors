---
format: aep.planning-md/3
id: story:mounted-custody-kind-secret-volume
kind: story
status: draft
title: Mounted custody proven against a Kubernetes Secret volume
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:mounted-secret-custody
- decomposes: epic:connectors-with-claude
- depends_on: story:mcp-stdio-runtime-branch
scope:
- confidence: cited
  path: .github/workflows/kind-custody.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors/tests/mounted_custody.rs
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: docs/local-mounted-secret-custody.md
revision: 8
---
## Outcome

Mounted-secret custody is proven against a real Kubernetes Secret volume, not only against a test
jail that imitates one.

## Today

story:mounted-secret-custody ships with its CLI tests in a user-namespace jail that builds the
volume layout the design infers: files `0440 root:<fsGroup>`, a read-only mount (`ST_RDONLY`) and the
`..data` symlink to a timestamped sibling directory. Those three facts are inferred from Kubernetes
documentation and not observed. Its acceptance case `container::kind_secret_volume`
(`.engineering/planning/story/mounted-secret-custody.md:513-516`) was not built (no workflow or
image for it); this story takes that case over, with its condition that the documented manifest
runs.

## Change

A test that runs the connectors binary in a kind cluster from the CronJob manifest of
`docs/local-container.md:63-110` (a Secret mounted as a volume, `fsGroup: 10001`), connects under
`mounted_secret`, invokes a read against a fixture provider, rotates the Secret and observes
`custody_unavailable` / `revalidate_connection` until a revalidate.

## Scope

- `apps/connectors/tests/mounted_custody.rs` (new): `container::kind_secret_volume`, `#[ignore]`d
  locally.
- `crates/connectors-build/src/ignored.rs`: its inventory entry, with the CI job as its runner.
- `.github/workflows/kind-custody.yml` (new): the job that creates the kind cluster and runs it.
- `docs/local-mounted-secret-custody.md:80-90`: the three volume facts.
- `CHANGELOG.md`.

## Acceptance

- `container::kind_secret_volume` in `apps/connectors/tests/mounted_custody.rs` passes in the
  `kind-custody.yml` job, running the manifest of `docs/local-container.md` unchanged.
- `docs/local-mounted-secret-custody.md:80-90` records each of the three volume facts as
  "observed in run <url>", naming that job's run.

## Order

- Depends on story:mcp-stdio-runtime-branch: both add entries to the `classify` match of
  `crates/connectors-build/src/ignored.rs`.
- story:claude-getting-started-tutorial depends on this story for the same file.
- story:standing-write-approval-for-unattended-agents cites the Secret-update behaviour this story
  observes; it has no edge here (its Order says why).
