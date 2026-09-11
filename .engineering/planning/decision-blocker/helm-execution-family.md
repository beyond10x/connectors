---
format: aep.planning-md/1
id: decision-blocker:helm-execution-family
kind: decision-blocker
status: open
title: Nobody has decided whether Connectors runs external provider binaries
relations:
- blocks: initiative:complete-local-connectors
- informed_by: architecture-decision-record:declarative-http-provider-runtime
withholds: test_result
revision: 1
---
## The question nobody has answered

106 of the 132 recorded Helm command sites need an execution family this
repository does not have: a local process, or an OCI registry client.

| Family | Recorded sites | What it needs |
|---|---:|---|
| B. Local chart work — `lint`, `template`, `show`, `pull`, `version`, `template test`, `registry login` | 105 | run the `helm` binary locally, or reimplement chart rendering; `pull` and `registry login` additionally need OCI registry access and its own credential profile |
| C. Release mutation — `rollback` | 1 | rewrite release storage and apply the workload changes that follow from it |

Counts are from `docs/evidence/recent-adapter-usage-20260909/actions.csv`.

## Why it is not a small extension

## Why it is not a small extension

Every capability an adapter has today is HTTP. `connectors_sdk` declares
`AuthenticatedHttp` (GET), `AuthenticatedWrite` (one consuming PUT) and
`AuthProbe` (one fixed-endpoint POST). There is no process-execution capability
anywhere in `crates/connectors-sdk/src`, `crates/connectors-client/src` or any
adapter: the only `Command::new` calls outside tests are
`crates/connectors-host/src/local/runtime/process.rs:42` and
`crates/connectors-host/src/local/owner/transport.rs:459`, which are the owner
spawning its own digest-verified adapter child, not a provider capability.

Family B therefore introduces a third execution family beside HTTP and the native
SQL protocol — running an external binary the repository does not own, does not
pin and cannot digest-verify the way it verifies adapter executables. That is a
platform decision, not a Kubernetes one. C16 (exec, copy, port forward) and the
Docker workflows in C17 would land on the same family, so the answer here decides
more than Helm.

`architecture-decision-record:declarative-http-provider-runtime`, accepted after
this blocker was first drafted, sets the principle the answer must satisfy:
generated endpoint definitions are "data, not arbitrary scripts", and an extension
needing a new algorithm "belongs in a reusable auth mechanism with its own
implementation and evidence, rather than an inline script repeated per action".
Running `helm` is the arbitrary-script shape that ADR rules out for the
declarative path, so family B cannot arrive as template data. It is either a
specified reusable capability or it is excluded. The same ADR keeps Kubernetes on
explicit native semantics rather than unary templates, so family A is unaffected
by it.

Family C is worse than a write. `helm rollback` is not one API call: Helm derives
the target revision from stored release history, writes new release storage and
then applies the workload changes. Reimplementing that derivation against the
Kubernetes API would be this repository asserting Helm's own semantics without
Helm, and the mutation contract's standard — an exact success/no-effect evidence
binding before advertisement, per
`adapters/kubernetes/contracts/mutations/v1alpha1/semantics.md` — could not be met
by a reimplementation. The single recorded `rollback` site does not justify it.

## What would clear this

An operator decision that names which of these the platform takes:

1. **Exclude both.** Helm rendering, linting, registry access and rollback stay
   outside Connectors, and `initiative:complete-local-connectors` line 35 is
   corrected so C15's Helm half and C16 are no longer claimed as Kubernetes scope.
2. **Take family B as a new execution family.** A process-execution capability is
   specified first — pinned binary identity and digest, argument construction,
   bounded output, exit-status semantics, working directory and filesystem
   admission — as platform work, with Helm as its first consumer. C16 and C17 then
   have somewhere to land.
3. **Take B, then C on top of it.** Rollback runs through the pinned `helm` binary
   rather than being reimplemented, and inherits the existing approval, ledger and
   unknown-outcome machinery for its mutation.

Option 1 is the smallest truthful answer and the one the recorded evidence
supports on volume alone — but it removes workflows the acceptance scenarios
currently claim, so it is the operator's call, not a default this blocker may take
on silence.

Recording this does not schedule any of the three. `story:kubernetes-helm-release-reads`
covers the 25 release-read sites and is not blocked by this question.
