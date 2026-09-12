---
format: aep.planning-md/1
id: story:kubernetes-helm-release-reads
kind: story
status: implemented
title: Read Helm release history, status, values and manifest through the local CLI
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:persistent-kubernetes-journey
- informed_by: specification:recent-agent-adapter-usage-20260909
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/kubernetes
- confidence: cited
  path: adapters/kubernetes/contracts
- confidence: cited
  path: adapters/kubernetes/generated/descriptor.json
- confidence: cited
  path: adapters/kubernetes/spec
- confidence: cited
  path: adapters/kubernetes/src
- confidence: cited
  path: adapters/kubernetes/tests
- confidence: cited
  path: docs/local-kubernetes-cli.md
revision: 5
---
## Acceptance

From an admitted Kubernetes connection, the local CLI reads a selected Helm
release's revision history, its currently deployed revision status, the values
recorded for a selected revision and that revision's rendered manifest, with
bounded pages, explicit completeness and provenance that names the exact release
Secret each observation came from. A release outside the configured namespace
scope, and a namespace whose release Secrets the credential cannot read, are
distinguishable from an empty history.

## Why this family and not the other two

The 132 recorded Helm command sites in `docs/evidence/recent-adapter-usage-20260909/actions.csv`
split into three families by execution semantics. The split is exhaustive: the
three counts below sum to 131, and the file's remaining row is `helm,unresolved,1`.

| Family | Recorded actions and command sites | Execution semantics |
|---|---|---|
| A. Release state reads | `history` 19, `get values` 3, `status` 1, `list` 1, `get manifest` 1 — 25 sites | Kubernetes API reads of the release storage objects |
| B. Local chart work | `lint` 50, `template` 24, `show` 5, `pull` 9, `version` 14, `template test` 2, `registry login` 1 — 105 sites | a local `helm` process and an OCI registry; no cluster is involved |
| C. Release mutation | `rollback` 1 site | rewrites release storage and applies workload changes |

This story takes family A only, because it is the one that lands on execution
semantics the repository already has. Families B and C are held under
`decision-blocker:helm-execution-family`.

`docs/recent-adapter-usage-20260909.md:147` already records the position this
follows: "Helm rendering/linting are local tool work; release operations need
their own explicit semantics, not an implied Kubernetes list capability."
`initiative:complete-local-connectors` line 35 assigns C15 and the Helm actions to
Kubernetes.

## What must be established before implementation

**The release storage format is not pinned in this repository.** No Helm upstream
source is vendored here, and nothing in `adapters/kubernetes/` states the object
name, type, encoding or JSON shape of a stored release. The repository's own
discipline for this is
`adapters/kubernetes/contracts/logs/v1alpha1/evidence/20260909/provider-sources.md`:
pin the exact upstream URL, uncompressed SHA-256 and byte length, then cite source
lines for every field the binding reads. Helm's storage driver and release types
must be pinned that way before a native contract states a shape. Until that
exists, every statement about the storage format is a hypothesis.

**Release Secrets are outside the current configured scope.** `resource_kinds` is
a closed enum of `pods`, `services`, `deployments`, `endpointslices` in both
`adapters/kubernetes/spec/adapter.json` and `adapters/kubernetes/src/lib.rs:38`.
Release storage is not among them, and this story must not widen that enum into a
general Secret read: a Helm release read is a distinct operation with its own
admitted target, not a resource kind a caller selects.

**Disclosure is the hard part, not the read.** A Helm release carries the values
it was installed with, and those routinely contain credentials. `get values` and
`get manifest` therefore need an explicit safe projection before advertisement,
and the default must be refusal to disclose rather than passing provider bytes
through. This is the one place where family A is not simply another bounded read.

**The typed nouns have no home yet.** A Helm release and a release revision are
entities no ESS document in this repository declares. Under planning skill
guardrail 7 the domain is drafted and validated before this story is decomposed;
it cannot be drafted honestly before the upstream source above is pinned, so the
cardinality between a release and its revisions is recorded as `UNMAPPED:` rather
than guessed.

**Cluster-wide listing is a separate selection.** `helm list` without a namespace
is the all-namespaces mode that
`adapters/kubernetes/contracts/auth/v1alpha1/semantics.md` §4.4 treats as a
distinct scope requiring its own admission and evidence. A per-namespace allow
cannot synthesize it.

## Sequence and boundaries

1. Pin the Helm upstream storage-driver and release-type sources with URL, digest
   and byte length; cite the exact lines for name, type, encoding and every read
   field.
2. Draft and validate the Helm release domain in `adapters/kubernetes/spec/ess`,
   keeping unresolved relations explicit.
3. Write the native read contract under `adapters/kubernetes/contracts/`, including
   the safe projection and the refusal default for recorded values.
4. Add the operations to `adapters/kubernetes/spec/adapter.json`, regenerate the
   descriptor, and bind them in `adapters/kubernetes/src/` with `Effect::Read`
   requirements in the executable composition.
5. Extend the fixture cluster in `adapters/kubernetes/tests/local_runtime.rs` and
   add a CLI journey case beside the existing two.

Excluded: every family B and C action; installing, upgrading, uninstalling or
rolling back a release; chart rendering, linting, packaging or registry access;
widening `resource_kinds`; and any general Secret read operation.

## Current state

Draft. Nothing is implemented and no upstream source is pinned. This story is not
scheduled and is not part of the increment currently sitting uncommitted in
`kubernetes-local-cli-20260911`.
