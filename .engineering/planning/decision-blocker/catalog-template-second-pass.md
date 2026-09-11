---
format: aep.planning-md/1
id: decision-blocker:catalog-template-second-pass
kind: decision-blocker
status: open
title: The catalog template unit is red after its second adversary pass
relations:
- blocks: story:catalog-operation-template
revision: 1
---
## What is blocked

`story:catalog-operation-template` is green on its own suite and red on its second adversary pass.
The two-pass attack budget is spent, so the unit does not merge and the decision is a person's.

## The trend, from `aep plan artifact findings`

Pass 1 found 7, all CONFIRMED and introduced. The correction resolved all 7 — the ledger reports
`carried 0`, `resolved 7`. Pass 2 then found 5 more, `new 5`, on ground the correction itself
created: four of the five are defects in the `location:name` key convention the implementor
invented to answer pass 1's finding about name-plus-location identity.

A falling count with nothing carried is a correction that landed. A second pass whose findings are
all new is ground that is still moving. Both are true here, which is why this is a judgement rather
than a rule.

## The open five

Two blockers: an absent-value refusal that names a key the caller already supplied, so re-supplying
it cannot satisfy the refusal; and the `location:name` key being an unescaped string prefix, so a
parameter literally named `query:trace` collides with the qualified spelling of `trace`.

Two warnings: a key the guard accepts but the precedence rule never reads, so a supplied value is
silently dropped; and header names accepted that are not RFC 9110 tokens.

One note, INFEASIBLE and recorded as residue rather than a case: a document-declared path holding
dot segments resolves elsewhere, which matters only once a caller loads a bundle it did not write.

## The decision

Whether a third pass is opened, whether the key convention is replaced by a typed key that cannot
collide, or whether the unit ships with the two warnings open and the two blockers fixed. The
implementor's own proposals are in the pass-2 review-result.

## What holds while this is open

The unit's worktree is retained — it is the only copy of the work. Nothing merged to `main`. The
rustfmt correction the wave applied to the three earlier catalog commits is in that same tree, so
`main` stays red on `cargo fmt --package connectors-catalog -- --check` until this is resolved or
that correction is landed separately.
