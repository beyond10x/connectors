---
format: aep.planning-md/3
id: decision-blocker:kubernetes-secret-disclosure
kind: decision-blocker
status: open
title: May a Kubernetes Secret read return a value, and under what rule?
relations:
- blocks: story:parity-kubernetes-secret-read
revision: 1
---
## Question

May `kubernetes.secret.read` (story:parity-kubernetes-secret-read, 1 call in the baseline) return a
Secret's data to a caller, and under what rule? Today `resources.list` excludes the `secrets`
kind (`adapters/kubernetes/spec/adapter.json`, the kinds enum) and `custom_resources.list` refuses
`k8s.io` groups (`docs/local-kubernetes-cli.md`, "Custom resources"), so no path discloses one.

## Options

- A: metadata only. Name, namespace, type, the data keys and each value's byte length; never a
  value. Serves "which keys exist" and "is it set"; the one baseline call read a value.
- B: values under an approval that names the Secret and the key, redacted from the audit, with the
  value delivered the way a consumer launch delivers a credential (descriptor 3), never in the
  CLI's answer.
- C: not built. The story is archived with this blocker's answer recorded.

Default on silence: A is drafted first because it discloses nothing; B needs the operator's word
because it is a security trade-off outside the approved designs. The story stays blocked until
this is cleared.
