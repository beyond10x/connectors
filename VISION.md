# Vision: independent integrations, shared guarantees

Applications and agents should be able to use an integration without taking on a
large, inseparable provider platform. They should also be able to combine
integrations while retaining clear answers about identity, access, provenance and
what happened when a request failed.

Connectors is built around independent adapters and shared semantic contracts.
An adapter knows its provider or protocol. A contract explains the guarantees that
an application can rely on across implementations. A host binds infrastructure and
can federate services; direct access remains a first-class path.

## Who this serves

Integration builders are the primary audience: developers consuming an adapter,
implementing a new one, or combining several into a product. Operators need
explicit configuration, readiness and failure behavior. Agent developers need the
same predictable boundaries, with discoverable operations and safe outcomes.

Consumers should not need to understand a provider's implementation to call it.
Adapter authors should not need to reproduce credential coordination, caller
admission or service plumbing for every provider. Operators should be able to
place an adapter where its provider is reachable and bind the infrastructure it
requires.

## What useful composition looks like

- **Explore engineering data.** Use GitLab for repository information and SQL for
  bounded database reads through the same service discovery and invocation boundary.
- **Turn observations into deliberate access.** Discover an endpoint through
  Kubernetes, select an appropriate adapter and explicitly configure its authority,
  credentials and route. Discovery itself never grants access.
- **Combine independent services.** Reach adapters directly or through federation,
  preserving source identity, limits, partial results and failure information.
- **Extend to other protocols.** Apply shared log, series, session or media
  semantics where they fit, while retaining each protocol's native behavior and
  explicit limitations. These are longer-term capabilities, not claims about the
  current runtime.

## Principles

1. **One owner for each behavior.** Shared contracts own generic guarantees.
   Adapters own provider mappings, native semantics, fixtures and protocol code.
   Infrastructure comes through explicit host capabilities.
2. **Independent delivery.** An adapter can be built, run and eventually released
   or extracted independently. A new provider should not require changing a
   provider list in the generic client or host.
3. **Explicit authority.** Caller identity, permission to use a connection,
   provider credentials and provider-side permission remain distinct. Descriptions
   and observations are information, not execution grants.
4. **Truthful results.** Bounded and partial reads say what they include. Lost
   responses and uncertain effects remain uncertain. Retry and recovery rules are
   declared, with observable refusal cases.
5. **Specifications that can be checked.** Versioned semantics, typed ESS models,
   generated interfaces and independent conformance cases should agree. Generation
   exposes missing behavior; compilation alone does not prove it works.
6. **Composition without mandatory central services.** Federation and catalog
   distribution are optional capabilities. Provider credentials stay within their
   admitted execution boundary.

## How we will judge success

A consumer can discover supported operations, invoke an admitted request and
interpret its result without provider-specific transport branches. An adapter
author can implement a provider using shared contracts and narrow infrastructure
ports, without importing sibling adapters. An operator can identify what is
implemented, enabled, ready and authorized as separate facts.

An adapter can leave this repository with its native specifications, fixtures,
provenance and implementation, consuming exact reviewed shared artifacts. Changes
to shared guarantees identify the affected implementations and conformance work.
Examples and public documentation explain both successful use and meaningful
failure behavior using the same contract sources.

These criteria are the direction of the project. Each implementation milestone
must name the subset it proves and retain evidence for that claim.

## Current milestone and boundaries

The local v0.1.0 specification baseline covers **Kubernetes including discovery,
GitLab and SQL**. The repository also contains an earlier working data/discovery
runtime slice. The subsequent contract hardening and persistent ESS models do not
claim that the newer runtime behavior has been implemented. See
[current capabilities](README.md#where-the-project-stands) and the
[changelog](CHANGELOG.md).

The broader contract and adapter documents preserve future direction. They are
not a commitment to implement every provider or profile in one release. Concrete
implementation work is selected and tracked through the repository's planning
process.

The goal does not require a universal workflow language, a generic entity
database, a service mesh, or a runtime plugin ABI. It does not require preserving
the old repository's package layout or copying its entire catalog. Compatibility
and migration promises need explicit selection and evidence.

The [design](docs/design.md) develops these boundaries. The
[contract index](contracts/README.md) and [adapter index](adapters/README.md) show
their current documented scope.
