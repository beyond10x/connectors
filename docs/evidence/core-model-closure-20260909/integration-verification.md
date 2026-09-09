# Combined model verification before final review

Baseline: `8a5cf563fc717fd4b23e4dd7d470d0c967f39461`, the reviewed semantic
checkpoint. This stage adds selected models and their owning prose; it changes
no runtime, adapter native source, strict adapter schema, Cargo manifest or lock.

The shared root now has 20 entities / 314 declarations in 18 files. Pinned ESS
0.20.0 `specify validate --path ess` and `specify compile --path ess --out ...`
both exited 0; a second canonical compile was byte-identical. The structured
[model comparison](combined-model-checks.json) checks every original identity,
lifecycle and field type and every original named type against the baseline IR.
All remain intact. Additive relations link actual declared targets; no stub
entities or generic property bags supply missing meaning.

The actual `connectors-build ... ess-boundary` run passed the shared source
ownership check and independently validated/compiled all five authored native
adapter roots. [Verbatim output](combined-boundary.log) records the compiler
counts. This is structural checking, not provider or persistence execution.

Coordinator joins after worker handoff retain the integrated mediated route while
adding the reviewer-requested baseline evidence carrier. AttemptRecord references
its independently retained Connection; AuditRecord optionally references a known
selected Connection as well as its existing optional attempt. Neither relation
owns deletion or grants current use. Audit key encoding now fixes unsigned
big-endian byte lengths and rejects noncanonical forms. Final prose states
cross-record instance/connection equality as an owner predicate. The original
unit reports remain unchanged and do not claim to have tested these later joins.

Manual integration cases: a revoked Connection remains referentially available
to historical attempts/audit without authorizing use; an early refusal cannot
manufacture a Connection reference from caller text; an audit reference under
another service instance cannot resolve to the original record; a Connection
can simultaneously carry its fixed mediated binding and its own applicable
material baseline without making historical discovery coordinates retain rows.
These are textual/model audits, not executed storage or codec cases.

Initial reviewer A's [MCA-01 report](reviews/reviewer-a-initial.md) is immutable;
the [auth correction](../model-auth-closure-20260909/correction-mca01/addendum.md)
records the single-field delta and unit checks. Final independent whole-stage
reviews and the complete repository gate are separate, pending observations at
this source freeze. Their final records will identify the exact reviewed commit.
