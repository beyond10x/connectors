---
format: aep.planning-md/3
id: story:a-marked-instance-takes-only-approvals-a-human-signed
kind: story
status: draft
title: A marked instance takes only approvals a human signed
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

An operator's rule can say: no write to this instance without a human's approval of that
write. The product cannot hold that rule against a process that runs as the operator's own
user, and agent sessions do.

Facts at v0.77.0:

- `approvals issue`, `approvals grant-standing`, `approvals key-init` and `approvals
  policy-set` take no proof from another party. `grant-standing` fetches the instance's own
  approval-signing key and signs in the calling process
  (`crates/connectors-host/src/local/owner/approval_issuance.rs:643-675`,
  `apps/connectors/src/local/approvals.rs:50-66`).
- The signing key is an Ed25519 seed in Secret Service custody
  (`docs/local-approval-keys.md:3-4`). Every process of the operator's user that reaches the
  session bus can use it while the keyring is unlocked.
- The contract states it: "A process of that UID can run `approvals grant-standing` itself, as
  it can run `approvals issue`: this is the same-user boundary a per-input approval has"
  (`contracts/cli/v1alpha1/semantics.md:890-893`; `docs/local-approvals.md:168-170`).
- What verifies an approval also belongs to that user: the public key record and the
  published policy (SQLite in the state directory), the grant file, the hourly use count
  (`standing-grants.sqlite3`) and the configuration file.
- No rule exists per instance kind. A proof or a grant for a Zendesk instance is issued the
  same way as one for any other instance.
- The owner admits a write with the public key only (`keys.key()`,
  `crates/connectors-host/src/local/owner/mutation/execution.rs:168`, `:211`); it does not
  need the seed. A signer outside the owner's process is possible without a change of the
  proof format.

One more fact bounds every option but D: the provider credential of the instance is in the
same keyring. A process of the operator's user can read it and call the provider without this
product. A rule inside the product binds only a caller that goes through the product.

## Outcome wanted

An instance can be marked so that an approval for it exists only when a human made it:

1. The approval signer of an instance can be bound to a custody that no agent process of the
   operator's user can use.
2. An instance can be marked "human proof only": `approvals issue`, `approvals
   grant-standing`, `approvals key-init` and `approvals policy-set` refuse for it when they
   run as the agents' user, and the owner admits no write to it under a key, a policy or a
   grant that user could have made.

## Options

Each option names what it closes, what it leaves open and what it costs. The costs are
estimates from the code read for this draft; nothing was measured or built.

### A. A marker in the configuration, checked by the binary

- Change: an instance entry gets `approvals = "human_proof_only"`. The four commands refuse
  for it with `forbidden` unless a human-presence check passes (a polkit prompt or a password
  on a terminal).
- Closes: an agent that issues an approval by mistake or by habit through the released binary.
- Leaves open: the marker, the key, the policy and the grant file belong to the agents' user.
  A process of that user can remove the marker, run an older binary, or sign with the seed
  from the keyring.
- Cost: one story. Configuration member, four refusals, one prompt, contract and guide text.
  No change of custody, of the proof or of the deployment.

### B. The signer runs as another user

- Change: the seed lives with a second local user (for example `connectors-approver`). `issue`
  and `grant-standing` send the exact subject to a signer of that user over a socket; the
  signer shows it to a human and signs after a confirmation (password, fingerprint). The
  trust anchor of a marked instance (public key, policy, the marker itself) moves to a file
  that only that user or root writes, and the owner reads it from there. The grant file is
  written by the signer and readable by the owner; the check "private to the operator"
  (`unavailable` today) becomes "written by the signer's user".
- Closes: an approval, a policy change or a standing grant for a marked instance made by a
  process of the agents' user, as long as the write goes through the owner.
- Leaves open: the hourly use count stays with the agents' user (a process of that user can
  free a cap, `contracts/cli/v1alpha1/semantics.md:1145-1147`). The provider credential stays
  in the agents' keyring.
- Cost: an epic of about five stories: signer process and its protocol, the anchor file and
  the owner's reading of it, custody binding for the second user, installation (a system
  user, a socket, a unit), and the human prompt. Linux first; macOS and Windows need their
  own answer for the second user. Every per-write approval for a marked instance costs one
  human confirmation; a standing grant costs one.

### C. A hardware signer

- Change: the approval key of a marked instance is a key on a security token, and each
  signature needs a touch. The anchor moves as in B.
- Closes: the same as B, and also a signer process that is itself taken over: without the
  touch there is no signature.
- Leaves open: the same as B. No approval without the token and a person at the machine.
- Cost: B's stories, plus a token per operator and a second signature scheme in the proof:
  a token's Ed25519 signature covers a wrapped message, so `approvals/proof.rs`, the grant
  file's signature and the contract's proof format change, with a compatibility rule for
  binaries that read only the current format.

### D. The owner and the credential run as another user

- Change: the instance is served by an owner under a service user that holds the provider
  credential and the anchor. Agents reach it over a socket and can only invoke. Approvals for
  it are signed by a human-held key (B's signer or C's token) and verified by that owner.
- Closes: everything above, and the direct call to the provider: the agents' user holds no
  credential of the instance.
- Leaves open: a human who approves without reading.
- Cost: the largest. The local CLI's model "the owner is a process of the calling user"
  changes for marked instances: a served deployment per workstation, moving the credential
  out of the operator's keyring, caller authentication on the socket, and the upgrade path
  (`setup owner-stop` across users). It builds on the service runtime of
  `docs/running-services.md`, which is a first slice today.

## Decision this story waits for

The pick between A, B, C and D is a change of architecture and is the operator's. A reading,
not a decision: A alone does not hold the rule and is worth building only as the first step
of B; B holds the rule for every caller of the product; only D holds it against a process
that reads the credential itself.

## Not in this story

- No implementation and no design document before the pick.
- The rule for standing grants on unmarked instances (which instance and operation may get
  one) is an operating rule of the installation, not of this story.

## Acceptance (to be written for the picked option)

For every option the first statement is the same and is checkable today as a red test: a
process of the agents' user that runs `approvals issue` or `approvals grant-standing` for a
marked instance gets `forbidden`, and a write to that instance with a proof or under a grant
that this user signed is refused before any provider request.
