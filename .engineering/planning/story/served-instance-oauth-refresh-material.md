---
format: aep.planning-md/3
id: story:served-instance-oauth-refresh-material
kind: story
status: draft
title: 'A served instance can hold and refresh OAuth-acquired material: decision needed'
summary: Served start refuses OAuth-acquired profiles, so per-person mail and calendar connections cannot run on a cluster; an ADR must choose how
relations:
- serves: vision:independent-contract-adapters
- informed_by: architecture-decision-record:oauth-material-as-static-entry
- informed_by: story:mounted-secret-custody
- informed_by: story:catalog-oauth2-refresh-profile
revision: 2
---
## Gap

A served instance cannot run any connection whose material is acquired by OAuth.
`docs/running-services.md` ("Start also refuses") makes start refuse such an instance, because
the CLI writes that material and nothing mounts it.

Locally, story:catalog-oauth2-refresh-profile (implemented) refreshes an access token from a
stored refresh token. architecture-decision-record:oauth-material-as-static-entry (accepted)
holds non-rotating refresh material as a static entry. story:mounted-secret-custody (implemented)
gives a served instance read-only items only.

So a per-person mail or calendar connection, Google's `oauth2_refresh` profiles for example, runs
on a workstation but not on a cluster. No story or ADR plans it.

## Decision needed

Before any implementation, an ADR has to choose how a served instance holds and refreshes OAuth
material. The candidates:

1. **Mounted refresh material, refresh in memory.** The refresh token is a read-only
   mounted-secret item. The host exchanges it at each request, or caches the access token in
   memory only. This works only for providers whose refresh token does not rotate, as the
   static-entry ADR describes. A rotating provider is refused at start.
2. **Writable custody for a served instance.** The host keeps rotated material in a store it
   owns, such as an encrypted file under `service.state`. This reopens what the served design
   excludes, a credential the host creates and keeps.
3. **An external token source.** Another component mints the access token, and the host reads it
   as a short-lived mounted item, as for the projected service-account token
   (story:served-instance-admits-a-service-account-token).

The ADR also answers four questions:
- how one served instance maps to one person's connection;
- what consent and revocation look like without the CLI;
- what start checks;
- what a refresh failure answers (`unavailable` versus a repair action nobody on the cluster can
  take).

## Acceptance (for the decision, not an implementation)

- An ADR is accepted that picks one option, or a combination, and records why the others were
  not chosen.
- This story is then rescoped, or split into implementation stories under that ADR. Until then
  it stays a draft.
