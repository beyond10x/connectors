---
format: aep.planning-md/1
id: review-result:catalog-declared-dot-segment-resolution
kind: review-result
status: active
title: Declared dot segments refused at bind time
relations:
- reviews: story:catalog-operation-template
revision: 1
---
## What was open

A **document-declared** path containing dot segments — `/files/{name}/../admin` —
binds to a path that resolves elsewhere. The implementor's argument for leaving it
was that the declared path is the operation's own identity and no caller value
moves it. The adversary's counter was that `bundle::load` reads a document a
third party may have written, and its SHA-256 check vouches for the file matching
its index row, not for who authored it.

It was recorded `INFEASIBLE` because no case could be written against a stated
decision, and no caller that loads an untrusted bundle existed yet.

## How it was answered, and why it was not a decision for the operator

The template now refuses a declared path whose literal text carries a `.` or `..`
component, as `Refusal::PathDeclaredDotSegment`.

The reason this is a reversible call with an obvious default rather than a real
question: **the refusal costs nothing that worked before.** `ScopedHttp::request`
already rejects a `.` or `..` path segment at dispatch, so a template holding one
could never have reached a provider — it would have failed later, and less
legibly. Refusing at bind time moves an existing refusal earlier and names it,
rather than changing what the system will do.

The counter-argument is answered too: the question no longer waits for an
untrusted caller to arrive, because nothing is trusted about the declared path in
the first place.

A literal dot inside a segment — `/files/v1.0/{id}` — is not a dot segment and
still binds. That is asserted, so the refusal cannot widen silently.

The case the adversary could not write now exists: four declared paths refused by
name, the benign one bound, mutation-checked by removing the refusal and watching
it fail.

Taken under working rules section 9: if the operator says nothing and the default
holds, nothing breaks, so it is work rather than a decision. Recorded here and
reported in one line.
