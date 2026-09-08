# datasource.records/v1alpha1 — document envelopes

**Status:** proposed; selected public document codecs are not implemented. Existing
bounded record pages remain defined by [service v1alpha1](../../../service/v1alpha1/semantics.md).

## Ownership and binding

This shared contract defines the obligations of a bounded document or document-list
result. An adapter owns the profile identifier, typed selectors, item schema,
native representations, source protocol, containment proof and continuation model.
Profile identifiers are open; installing an adapter does not amend a shared enum.
An advertised profile must bind an exact version of this contract and supply its
own versioned schema and conformance obligations. Framing alone is insufficient:
[service compatibility](../../../service/compatibility.md) governs typed readers.

The [Atlassian document binding](../../../../adapters/atlassian/contracts/documents/v1alpha1/semantics.md)
owns the currently proposed native document/search profiles. Its provider traces,
representation limits, cache predicates and query grammar travel with that adapter.
The link is an implementation index, not a dependency of this contract on that adapter.

## Result and representation obligations

A detail result has `item`, `body`, `complete` and `provenance`, with no document
cursor. A document-list profile declares `items`, `complete`, `next_cursor` and
`provenance`; it states whether each item is a full detail result. Each selected
schema is closed and typed by that profile. A receiver must not insert arbitrary
provider objects into an undeclared extension field.

`body` contains `representation`, `bytes`, `content`, `truncated` and `truncation`.
The profile owns the representation identity and exact content type, original-byte
measurement, safe shortening or whole-body omission, and allowed truncation causes.
`bytes` counts the full representation from a fully received, valid, bounded source
observation; a cutoff cannot establish an exact original length. A native object
must not silently become a string or a partial object with fields removed.

Detail `complete` describes successful establishment of the selected authorized
object, independently of explicit body shortening/omission. A malformed or partial
detail response cannot establish success. List completeness describes exhaustion
of the admitted selection under its declared collection rules; body shortening is
reported separately. No generic list codec promises a snapshot, deduplication or
stable provider membership.

Provenance identifies the admitted instance and source/resource, original
observation time, and a nullable source revision. Missing provider revisions are
null; display timestamps are not invented global content versions. A cached body
retains its original observation time and revision. A later authorization check
does not refresh those facts.

## Admission, caching and continuation

Current principal, operation, source/connection, scope, policy, credential and
route bindings govern every dispatch and disclosure. Caller coordinates, returned
links, cached membership and content versions grant no authority. Each adapter
must state what its provider evidence proves and which concurrent changes it
cannot prevent. Host checks do not make multiple provider calls transactional.

Caching and continuation require explicit profile support. Reuse includes the
complete current private context and original projection/representation and
limits. Known revocation, changed bindings and invalidation defeat age-based reuse;
no stale-on-error or cross-principal reuse is implied. A cursor grants no authority:
current admission precedes disclosure of validity or content. A profile must state
whether replay returns a retained observation or repeats provider work, and how it
bounds expiry, capacity, progress and source requests. It cannot silently substitute
one continuation model for another.

## Bounds, errors and verification

Every binding specifies finite input, encoded and decoded source, metadata, body,
serialized-result, provider-call, deadline and retained-state limits. Auxiliary
lookups and permission checks consume the original shared budget. Storage ceilings
include backing allocations and context/index overhead, with atomic admission
across concurrent requests. No truncation may omit required identity or authority
metadata; failure to establish a bounded valid observation is a safe error.

Errors use the shared binding's safe classifications without raw provider bodies,
credentials or foreign scope metadata. A profile specifies ambiguity-hiding absence
behavior and distinguishes host denial, provider absence, timeout and unusable
source observations. This contract grants no automatic refresh or resend.

Generic body facts and completeness decisions are modeled in
[shared ESS](../../../../ess/domains/datasource_reads.yaml). Native representations
and selectors live in the adapter's authored ESS. Parser correctness, scope proof,
byte accounting, current publication fences and cache behavior remain binding
obligations; ESS shape validation does not execute them.
