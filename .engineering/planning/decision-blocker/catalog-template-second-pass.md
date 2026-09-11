---
format: aep.planning-md/1
id: decision-blocker:catalog-template-second-pass
kind: decision-blocker
status: cleared
title: The catalog template unit is red after its second adversary pass
relations:
- blocks: story:catalog-operation-template
revision: 3
---
## What is open

One thing, and it is the residue of the second adversary pass rather than a defect anybody has
demonstrated reaching: a **document-declared** path containing dot segments — `/files/{name}/../admin`
— binds to a path that resolves elsewhere. The implementor's argument for leaving it is that the
declared path is the operation's own identity and no caller value moves it. The adversary's counter
is that `bundle::load` reads a document a third party may have written, and its SHA-256 check
vouches for the file matching its index row, not for who authored it.

It is recorded `INFEASIBLE` because no case was written against it: asserting the opposite of a
stated decision asserts a preference, and no caller that loads an untrusted bundle exists yet.

## What is settled

The two blockers and two warnings the second pass raised were fixed on the operator's instruction
and the unit merged as `9b88cd0`: 53 cases, `cargo test`, `cargo clippy -D warnings` and
`cargo fmt --package connectors-catalog -- --check` all exit 0, with all eleven adversarial cases
from both passes in the suite and none modified.

## The decision, when a caller arrives

Whether a document-declared path is trusted because the bundle's digest matched, or whether the
template refuses a declared path whose segments resolve away from it. The question becomes live the
first time something loads a bundle this repository did not produce.
