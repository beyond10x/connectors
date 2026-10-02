---
format: aep.planning-md/3
id: tooling-blocker:kubernetes-driver-protocol-loading
kind: tooling-blocker
status: cleared
title: AEP resolver loads no protocol documents for the Kubernetes draft
relations:
- blocks: story:kubernetes-spec-service
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-02T12:42:09Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
---
## Observed refusal

On 2026-09-10, AEP 0.54.0 in the integrated Connectors checkout at 49bb646 was run read-only with:

```sh
aep govern resolve --task .engineering/tasks/kubernetes-spec-service.yaml --format json
```

It exited 1 and printed:

```text
error: the task cannot be resolved: [unknown_protocol] protocol adp/1: no protocol document declares `adp/1` (hint: no protocol documents are loaded at all)
```

The full stderr and empty JSON output are retained in primary .local/integration-20260910/kubernetes-task-resolution.stderr and kubernetes-task-resolution.json. This reproduces the earlier preparation refusal. aep doctor separately reports that .engineering/project.yaml parses and its pinned protocol snapshot is cached; that does not establish successful task resolution.

## What clears it

A future selected Kubernetes run must demonstrate successful loading and resolution of its declared protocol/profile under the current project configuration without replacing protocol semantics to bypass the refusal. Record the actual command and successful result before clearing this blocker. The current integration request does not authorize modifying the sibling AEP implementation or launching a paid driver run.

This blocks the future governed Kubernetes launch, not consolidation of the existing draft, local Git integration or recovery cleanup. No Kubernetes implementation-completion evidence is claimed.

## Resolved input, 2026-10-02

Both current AEP 0.68.0 and repository-pinned 0.65.0 exit 0 when govern resolve is passed --root pointing to the unchanged cached snapshot 665cd6eddd512dff037530e4503a58b679b55854 from .engineering/project.yaml. Output selects adp/1, development.standard, adp/default and eleven obligations. Exact pinned output and reproducible command are retained in docs/evidence/kubernetes-resolver-20261002/. Implicit project-root resolution still has the historical unknown_protocol refusal; explicitly selecting the configured protocol documents removes the launch prerequisite without changing their semantics. No paid driver was launched.
