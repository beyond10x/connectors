---
format: aep.planning-md/3
id: story:auth-profile-login-exchange
kind: story
status: draft
title: An auth profile exchanges a username and password for a bearer token
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

An auth profile may declare a login exchange: the stored credential is a username and a password,
and the provider is called with a bearer token obtained by posting them to the profile's login
endpoint, renewed before it expires, never written to custody. Homer (story:parity-homer-sip-reads)
authenticates this way (a JWT from `POST /api/v3/auth`), and so do private APIs that ship no
OAuth; today no reviewed profile scheme supports it (`parity-homer-sip-reads`, "Domain draft").

## Surface

The auth profile contract `contracts/auth/profile/v1alpha1/semantics.md` gains the scheme
`login_exchange`: a login URL relative to the connector's API base, the request shape (JSON with
two named members), where the token is read in the answer (a JSON pointer), its lifetime (from
the answer or a declared default) and the header it is sent in. The host's credential custody
keeps the username and password as the custody item; the exchanged token is validation evidence
with a lifetime, recollected the way lapsed evidence is today (0.54.0, "a read on lapsed
validation evidence recollects it once"). The catalog provider sends the token as a bearer. The
definition schema in the definitions repository (`urn:beyond10x:connectors-definition:1`,
`auth` section) admits the scheme so a definition file can declare it.

## Acceptance

- Spec first: the scheme is modelled in the shared auth domains (`ess/domains/auth_access.yaml`,
  `ess/domains/credential_evidence.yaml`, `ess/domains/credentials.yaml`), validated with the
  pinned `ess`, regenerated, and the contract page updated, before the host changes.
- `connections connect --profile <login-exchange profile>` prompts for a username and a password
  (hidden), posts them to a fixture login endpoint, stores only the two in custody, and the
  connection is `ready`; `connections describe` shows the scheme and no token.
- A read after the token's lifetime lapsed recollects a new token from custody without a prompt and
  is served; a login endpoint answering `401` leaves the connection `stale` with the refusal named.
- The token never appears in the audit, in `connections describe`, in a log or in a consumer
  launch's environment; a test asserts each.
- A definition file declaring the scheme lowers and runs through `operations invoke --definition`
  with the same prompts.

## Scope

Primary: `contracts/auth/profile/v1alpha1/semantics.md`, `ess/domains/auth_access.yaml`,
`ess/domains/credential_evidence.yaml`, `crates/connectors-host/src/local/oauth.rs` (sibling
module for the exchange), `crates/connectors-host/src/local/registry/revalidation.rs`,
`adapters/catalog/src/lib.rs` (bearer from evidence), the definitions repository's
`spec/domains/definition.yaml`. Collides with any story touching `revalidation.rs`
(story:upgrade-moves-catalog-adapters, story:connection-follows-configuration-upgrade): runs in a
later wave than those.
