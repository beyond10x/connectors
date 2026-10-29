---
format: aep.planning-md/3
id: story:broker-answers-a-confined-run-without-handing-over-the-credential
kind: story
status: draft
title: The broker answers a confined run without handing over the credential
summary: 'Glovebox wave 5: a process in a box presents only a run-scoped grant, the served host attaches the credential from custody, and the credential never enters the box.'
tags:
- glovebox
- o1
relations:
- informed_by: verification-report:findings-concept-stack-integration-20260908
- informed_by: verification-report:findings-wire-binding-r1-20260908
revision: 1
---
# Story: the broker answers a confined run without handing over the credential

Part of Glovebox (atlas `initiative:glovebox`, `docs/design/glovebox.md`), wave 5.

## Why

- Glovebox requirement 3: a process in the box can, "for example, list the operator's GitLab merge
  requests **without ever seeing the credential**, which lives beside the daemon"
  (`atlas/docs/design/glovebox.md:23-25`). Rule SM-D201: "API calls such as GitLab merge requests
  go through a Connectors broker" (`atlas/docs/design/glovebox.md:58-59`). The layer table gives
  Connectors and Secrets "hold every credential; answer the box's requests through a broker"
  (`atlas/docs/design/glovebox.md:41`).
- The operation exists: the GitLab catalog provider declares `merge_requests.list` as a read
  (`adapters/catalog/providers/gitlab/operations.json:19`).
- A served host exists, but with one shared caller secret: `connectors-host` binds a TCP listener
  (`crates/connectors-host/src/server.rs:226-244`) and admits a request only when its bearer equals
  the one configured `service_credential` (`crates/connectors-host/src/server.rs:88-101`). A box
  holding that secret could call every admitted operation for as long as the secret lives; it is
  not scoped to a run.
- Delegated context into Connectors is still unbound: review finding WB-05 says the proposed
  header "signs context and request_id but not destination leaf, method/path, body/input digest,
  operation/connection or revision" and asks that no keyed mutation or caller-context federation be
  advertised "using today's static leaf credential"
  (`verification-report:findings-wire-binding-r1-20260908`, line 74).
- Custody is not in Secrets yet: finding G4 records "`SecretStore::read(reference)` only … no
  `secrets-client` dependency", target "Connectors reaches Secrets through the released client and
  a workload identity; Vault path to be removed"
  (`verification-report:findings-concept-stack-integration-20260908`, line 71). Atlas's O1 row
  says "Connector credential migration remains open" (`atlas/ROADMAP.md:74`), and the dependency
  row `connectors → secrets` is "intended" (`atlas/ROADMAP.md:167`). No story in this store
  carries that migration (checked with `aep plan artifact list`, 2026-10-29).

## What changes

1. A design first, because it adds an authority to the service contract. It decides at least:
   - the run grant the broker accepts: who mints it (Connectors itself, as the issuer ADR 0011 of
     Substrate already names, or Identity), its audience, its lifetime, and what it binds: the run,
     the connection instance, and the set of admitted operations;
   - how a request is bound to the grant, closing WB-05: method and route, operation id, input
     digest and a one-use request id, verified before the body is decoded;
   - that the broker attaches the credential from custody on its side and returns only the
     operation's result; no response, error or audit record carries the credential;
   - which operations a run grant may admit: reads first; a write still needs an approval proof
     for its exact input, as today;
   - named refusals: no grant, expired grant, wrong audience, operation outside the grant,
     request not bound to the grant.
2. The served host accepts the run grant beside, not instead of, its service credential.
3. The audit record of each call names the grant and the run, so a reader of one record can
   answer Atlas O1's question.

## Acceptance

- The design is accepted before code.
- A test client holding only a run grant lists merge requests of a test GitLab double through the
  served host; the response holds the merge requests and nothing that matches the credential.
- The same grant is refused by name for an operation it does not admit, after it expires, and for
  a request whose body differs from the one it was bound to.
- The audit record of the accepted call names the grant and the run, and holds no credential.
- An end-to-end case with Substrate's `story:a-run-presents-its-delegated-grant-to-a-broker`: a
  confined child reaches the host through a declared aperture and lists merge requests without any
  credential in its environment, files or descriptors.

## Blocked on / Depends on

- Atlas wave 5 waits on Connector credentials in Secrets (`atlas/docs/design/glovebox.md:94`).
  This story can be designed now; whether the broker reads custody from Secrets or from today's
  custody is a decision for its design. No migration story exists here yet; filing one is the
  operator's call.
- Pairs with Substrate's `story:a-run-presents-its-delegated-grant-to-a-broker`, which gets the
  grant into the box.
