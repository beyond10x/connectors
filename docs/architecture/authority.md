---
title: Connections and authority
description: How provider configuration, account authorization, Grants, approvals, and credential custody fit together.
sidebar_label: Connections and authority
sidebar_position: 2
b10x:
  documentType: architecture
  audiences: [developer, operator, evaluator]
---

# Connections and authority

Connecting an account and permitting an action are separate decisions. A Connection identifies
the authorized external account or target. A Grant determines which operations or events may use
that Connection. Provider credentials stay in Connectors' credential store.

## From a provider to a usable Connection

```mermaid
flowchart TB
    accTitle: Establish access before requesting an action
    accDescr: A deployment enables an Integration. A Connect Session establishes a Connection and credential custody; action admission remains a separate decision.
    provider[Provider: reviewed capabilities] --> integration[Integration: deployment configuration]
    integration --> session[Connect Session: acquire authorization]
    human[Authorized person or provider consent] --> session
    session --> store[Credential store]
    session --> connection[Connection: durable account or target]
    connection --> admission[Receiver policy and required Grants]
    admission --> call[Admitted invocation or event access]
```

The deployment enables an Integration and supplies its non-secret policy. Where acquisition is
required, the person completes a Connect Session through protected input or provider consent.
Credentials enter Connector custody; the caller observes the resulting Connection and its status.
Requests must still pass the authority checks that apply to their operation.

Not every Connection uses this acquisition flow. Local Kubernetes can use existing owner-controlled
cluster configuration; hosted OAuth and direct operator entry have different prerequisites.
The [provider guides](../../README.md#connect-a-provider) explain those differences.

## Two kinds of session

| Session | Owner and purpose | Result |
|---|---|---|
| Identity login session | Identity; lets a hosted client obtain short-lived access authority | Client login continuity, stored in the OS keyring |
| Connect Session | Connectors; a bounded attempt to establish or repair provider authorization | Connection status and reference, with credentials retained by Connectors |

The [Connect Session lifecycle](../../crates/service/src/connect_session.rs) starts at `pending`
and terminates at `completed`, `expired`, or `failed`. Terminal results cannot be completed again.
A terminal response does not confer general authority on its caller.

## The example: establish C1

The person adds a companion bot through its protected Connect Session. Connectors validates the
configured workspace and retains the app and bot credentials for the resulting Connection, C1.
The person's Identity login is separate: it supplies authority to access the hosted deployment.
Neither the application nor a model needs the provider credentials.

## The checks on an invocation

Hosted requests present short-lived Identity authority for the exact Connector audience and route
scope. Connectors validates it and derives the tenant and principal from that authority. Request
fields do not choose an independent tenant. Receiver-side policy further narrows admission.

The [Grant evaluator](../../crates/domain/src/evaluator.rs) checks the selected Connection and the
operation facts supplied by admission. A [Grant](../../crates/domain/src/grant.rs) binds one Connection and
can express a risk ceiling, admitted effects and idempotency classes, explicit operation allow/deny
sets, and a closed inbound event set. An explicit denial takes precedence.

Hosted effect-bearing invocations require an admitting Grant. Described read-only operations with
no approval requirement can instead use the existing receiver-policy read path. Event access has
its own route and backend admission. These paths do not all exercise the same Grant check.

The current hosted description also omits several reviewed traits. Its
[enforcement adapter](../../crates/server/src/hosted/enforcement.rs) supplies conservative risk,
effect, and idempotency bounds; an exact operation allow rule is the expected admission shape.
The richer Grant type alone does not prove that every transport supplies all of its facts.

For operations requiring approval, the [approval gate](../../crates/domain/src/approval.rs)
verifies issuer, subject, operation, Connection, canonical input digest, and expiry. It redeems an
approval once and records the attempt before dispatch. Recovery distinguishes an aborted attempt
from an indeterminate one whose effect may already have happened.

## The example: approve one reply

After receiving E1, the application proposes `slack-chat-post-message` through C1, with a specific
channel, thread, and message. It first describes the operation and receives D1.

The [hosted approval endpoint](../../crates/server/src/hosted/approval.rs) requires verified human
authority with `connectors.approvals.issue`. It rechecks D1 and C1, then records A1 against the
subject, operation, Connection, canonical input digest, and expiry. A changed message or thread
is different input and cannot reuse that approval.

**Receiving E1 is not approval to reply.** The hosted path loads and redeems an issued approval
record. The local runtime's separate `event:` claim mechanism does not establish an automatic
hosted reply or a universal ten-minute event grant.

| A caller has… | What still has to hold |
|---|---|
| A valid hosted login | A current access token with the correct audience and scope |
| A callable Connection | Receiver admission and, for a hosted write, an admitting Grant |
| A Grant | Current description/authority checks and any required approval |
| An approval reference | Exact request binding, validity, and successful one-time redemption |

## Where credentials live

The [SecretStore port](../../crates/connector-secrets/src/lib.rs) separates custody from provider
behavior. Local owner-bound storage and hosted Vault-backed storage implement different deployment
arrangements. Connectors resolves and places provider credentials inside the admitted execution
path; they are not operation input supplied by an agent.

Hosted administration separates non-secret configuration from credential entry. See
[Administer hosted Integrations](../guides/administer-hosted-integrations.md) for the operator
workflow and [Deployment and runtime](deployment.md) for storage arrangements.

[Previous: System overview](README.md) · **Next:** [Commands and interfaces](interfaces.md)
