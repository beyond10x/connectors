---
format: aep.planning-md/3
id: story:short-form-endpoint-move-answers-authentication-changed
kind: story
status: draft
title: A short-form connect that moves an instance to another host answers the changed-authentication code
summary: connect --endpoint onto another provider authority still answers lifecycle_conflict while the same registry rule elsewhere answers authentication_changed
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- informed_by: review-result:wave-b-config-change-adversary-1
revision: 1
---
## Defect

Found by the adversary pass on story:configuration-change-error
(review-result:wave-b-config-change-adversary-1, finding on `apps/connectors/src/local/endpoint.rs`).
The answer predates that story; the split between two codes for one cause is new.

`connectors connect <connector>/<instance> --endpoint <other host>` on an instance that holds a
connection under another provider authority answers `lifecycle_conflict` at stage
`configuration` with `next_action = check_configuration` and `field = endpoint`
(`apps/connectors/src/local/endpoint.rs`, `contracts/cli/v1alpha1/semantics.md` on short-form
connect). The registry rule behind it is the same one that now answers `authentication_changed`
at `admission` ("another host is another instance"): only a new instance id admits a connection.
A caller therefore sees two codes for one cause, and the older one invites a retry.

## Acceptance

- A short-form connect whose `--endpoint` moves a connected instance to another provider
  authority answers the same code as a connect under a changed authentication, still naming
  `field = endpoint`, and nothing is written.
- `security_d218_u2.rs` and the contract text say the same; the vocabulary count is unchanged.
