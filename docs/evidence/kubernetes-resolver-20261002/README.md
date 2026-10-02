# Kubernetes task protocol resolution, 2026-10-02

The repository-pinned AEP 0.65.0 resolves the retained Kubernetes task against the exact protocol snapshot selected by `.engineering/project.yaml`, commit `665cd6eddd512dff037530e4503a58b679b55854`. The successful output is [resolved.json](resolved.json): task `connectors-kubernetes-spec-service`, protocol `adp/1`, profile `development.standard`, workflow `adp/default`, eleven obligations.

Reproduce by obtaining the cached snapshot path reported by `aep doctor --format json`, checking that its commit equals the project's pinned locator, then:

```sh
"$CONNECTORS_AEP" govern resolve --root "$protocol_snapshot" \
  --task .engineering/tasks/kubernetes-spec-service.yaml --format json
```

The command exited 0 with both AEP 0.65.0 and 0.68.0. Supplying the same pinned documents explicitly is a command-input correction, not a protocol/profile substitution. The implicit-root command still exits 1 on AEP 0.68.0:

```text
error: the task cannot be resolved: [unknown_protocol] protocol adp/1: no protocol document declares `adp/1` (hint: no protocol documents are loaded at all)
```

This proves read-only task resolution. It does not launch a paid driver, prove Kubernetes generation/runtime behavior, or authorize a different protocol revision. Kubernetes implementation must use current source and released tool pins rather than historical source-pin prose in the retained draft task.
