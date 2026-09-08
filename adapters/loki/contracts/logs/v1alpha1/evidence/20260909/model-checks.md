# Loki model checks — 2026-09-09

Executed with the repository-pinned binary
`/home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess`.
`ess --version` returned `ess 0.20.0` (exit 0). Commands ran from the assigned
datasource worktree at base `8e1836cad8ae1b2127ce9ae306c6d8131960db4c`.

```console
ess specify validate --path adapters/loki/spec/ess
connectors_loki v1 — 2 file(s), valid
```

Validation ran before and after the prose correction; both exits were 0.
Compilation ran before and after with the following command, substituting
`before` and `after` for `PHASE`:

```console
ess specify compile --path adapters/loki/spec/ess --format json --out .local/spec-completion-20260909/loki-PHASE-ir.json
```

Both compile exits were 0. The emitted IR was compared with `cmp` (exit 0) and
was unchanged, SHA-256
`70baafbf6eb852679d7a3a82f9310d17d7563002093e92af1bc594aef8cbda02`.
`jq` inspection reports 1 domain, 3 types, 0 entities and 0 commands.

The model already types native LogRangeSelection, LogDirection and
LogScopeEquality. This change introduces no new entity or model value. Query
grammar, canonical/numeric bounds, exact equality admission, ordering, multiplicity,
decoding, cause predicates, cache capacity and current publication remain explicitly
UNMAPPED binding obligations. The unchanged IR is expected for prose/evidence
work; these four successful model commands are not runtime scenario executions.
The 41 manual native log cases are separately recorded by their adapter owners.

The three previously absent provider source archives were checked against the
original source hashes (all OK), and decompressed archives matched source bytes
with `cmp` (exit 0). Those checks establish retained bytes, not provider behavior.
The coordinator runs shared-model and repository integration gates after merging.
