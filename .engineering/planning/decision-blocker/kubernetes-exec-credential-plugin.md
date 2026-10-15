---
format: aep.planning-md/3
id: decision-blocker:kubernetes-exec-credential-plugin
kind: decision-blocker
status: open
title: Nobody has decided whether Connectors runs a kubeconfig-named exec credential plugin
relations:
- blocks: story:kubernetes-eks-exec-auth-and-kinds
revision: 1
---
## The question

Does the bounded process-execution family run a kubeconfig-named exec credential plugin
(for example `aws eks get-token` for an EKS cluster)?

`decision-blocker:helm-execution-family` was cleared with a family that "does not select
arbitrary commands, unpinned executables, filesystem access or credential inheritance by
default". A kubeconfig exec plugin needs all four: the kubeconfig names the command, the
executable is not pinned, it reads provider files such as `~/.aws`, and it inherits the
environment's credentials. So the exec half of
`story:kubernetes-eks-exec-auth-and-kinds` is not implemented. Its reads half (statefulsets,
`nodes.list`, `custom_resources.list`) is.

## If yes, three choices

1. The executable: pinned by absolute path and digest in the connection configuration, or
   taken from the kubeconfig as written.
2. What it inherits: which environment variables and which files it may read.
3. Who runs it: the Kubernetes adapter (inside an admitted operation, with a bounded
   deadline), or the host. The host route collides with the host and CLI work.

## Options

- a. No: EKS stays unsupported through exec; a cluster is reached with a bearer token or a
  client certificate that something outside Connectors refreshes.
- b. Yes, pinned: the connection configuration names the plugin by absolute path and digest
  and an allow-list of environment variables; the adapter runs it only inside an admitted
  operation. Recommended: it keeps the digest rule every other executable follows.
- c. Yes, as the kubeconfig says: the closest to kubectl, and the widest trust.

## Today

The reads contract and `docs/local-kubernetes-cli.md` say exec credentials are not supported.
