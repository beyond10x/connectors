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
revision: 1
---
## Outcome

Mounted-secret custody is proven against a real Kubernetes Secret volume, not only against a test
jail that imitates one.

## Today

story:mounted-secret-custody ships with its CLI tests in a user-namespace jail that builds the
volume layout the design infers: files `0440 root:<fsGroup>`, a read-only mount (`ST_RDONLY`) and the
`..data` symlink to a timestamped sibling directory. Those three facts are inferred from Kubernetes
documentation and not observed; the acceptance's `container::kind_secret_volume` case was not built
(no workflow or image for it).

## Change

A test that runs the connectors binary in a pod on a local cluster (kind) with a Secret mounted as
a volume and `fsGroup` set, connects under `mounted_secret`, invokes a read against a fixture
provider, rotates the Secret and observes `custody_unavailable` / `revalidate_connection` until a
revalidate. It runs in a CI job of its own and is listed in the ignored-test inventory until that
job exists.

## Acceptance

- `container::kind_secret_volume` passes in CI against kind, and the design's three inferred
  volume facts are recorded as observed, with the job's run as the source.
