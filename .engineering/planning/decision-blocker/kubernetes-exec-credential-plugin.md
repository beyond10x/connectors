---
format: aep.planning-md/3
id: decision-blocker:kubernetes-exec-credential-plugin
kind: decision-blocker
status: cleared
title: Nobody has decided whether Connectors runs a kubeconfig-named exec credential plugin
relations:
- blocks: story:kubernetes-eks-exec-auth-and-kinds
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-15T08:13:24Z", actor: "human:timo", revision: 3}
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

## Decided (2026-10-15)

Option b, within the bounded pinned-binary process family the execution-family decision
admitted:

- The exec credential plugin runs only when a connection's configuration names it
  explicitly: an absolute path and a sha256 digest, verified before every run and refused on
  a mismatch.
- The argument list comes from the kubeconfig, as data, never through a shell.
- The configuration names, per connection, an allow-list of environment variables and any
  files the plugin may read (for example the AWS profile files). Nothing the configuration
  does not name is inherited.
- It runs only inside an admitted operation, with a bounded deadline and bounded output.
- Its token is held in memory until its stated expiry and never written to disk.
- No default configuration enables it. Running whatever the kubeconfig names (option c) is
  refused.
