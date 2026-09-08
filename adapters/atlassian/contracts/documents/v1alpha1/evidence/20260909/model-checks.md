# Atlassian model checks — 2026-09-09

The repository-pinned binary reports `ess 0.20.0`:
`/home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess`.
Commands ran from the assigned datasource worktree at base
`8e1836cad8ae1b2127ce9ae306c6d8131960db4c`.

```console
ess specify validate --path adapters/atlassian/spec/ess
connectors_atlassian v1 — 2 file(s), valid
```

Validation ran before and after the document correction; both exits were 0.
Compilation ran in both phases with this command, substituting `before` and
`after` for `PHASE`:

```console
ess specify compile --path adapters/atlassian/spec/ess --format json --out .local/spec-completion-20260909/atlassian-PHASE-ir.json
```

Both compile exits were 0. `cmp` of the emitted files exited 0. Their unchanged
SHA-256 is `230cf351782cde13f59fe09d451d84b382180861ab7438c509b161c02a5ee3a9`.
`jq` inspection reports 1 domain, 3 types, 0 entities and 0 commands.

The existing private model types DocumentRepresentation, DocumentScopeKind and
CqlSearchSelection already name the native vocabulary. No new durable entity or
ownership edge is introduced. Native payload schemas, lossless numbers, grammar,
query filtering, optional source-field interpretation, byte arithmetic, full
context equality, immutable lineage/capacity and current publication remain
explicitly UNMAPPED binding obligations. The unchanged IR is expected from this
prose/evidence correction; successful compilation does not execute a decision
reducer or validate provider behavior.

The 42 [manual document/CQL cases](scenarios.md) are separately evaluated textual
traces. Two literal byte counts were checked by `printf '%s' <literal> | wc -c`
(43 and 8; both exit 0); that does not prove lossless decoding/projection.
Six new CQL archives matched their cached source bytes under decompression/`cmp`
and SHA-256 checks (all exit 0). The PageBulk facts JSON was extracted with `jq`
from the already archived official source, preserving its source digest/pointers.

No runtime suite, provider call, public codec, parser, host-cache implementation,
adapter-kind schema or planning-store change is part of this work. The coordinator
owns shared-model/integration checks and independent final review.
