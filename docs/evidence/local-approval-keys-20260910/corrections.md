# Corrections observed during implementation

The starting clean local main was 40d43a11cac1880dd9f098d47e63cbf47cbcd499.

- Initial pinned ESS validation refused four transition outcomes without observable events (`empty_change`, with accompanying `missing_causation`). The authored model now emits publication, revocation, retirement and deletion facts; later validation passes. The initial full diagnostics were returned in the session.
- CLI projection refused `Uuid` option primitives, then three-level command paths. The selected authored presentation uses `approvals key-*` and String UUID coordinates, with strict canonical non-nil UUID validation in production handlers. Domain identities remain UUIDs. The generator was not changed.
- The pinned generator then refused pre-existing unenrolled output. The exact HEAD specification and CLI binding were extracted with `git archive`, regenerated into a task-owned directory and checked against the committed parser. `ess generate output adopt --owner cli-binding` enrolled only those exact baseline bytes before regeneration. No generated file was hand-repaired.
- Compilation caught a borrowed record moved while constructing current-key configuration, a test barrier lifetime and an ambiguous CLI error conversion. These were fixed in authored Rust; failed compiler logs are retained.
- The existing direct-custody test fixture's bus parent was not private enough for the production socket admission path. The fixture now creates that task-owned parent with mode 0700. Product admission was not weakened. One diagnostic command used a partial name with `--exact` and ran zero tests; that result is retained and is not acceptance evidence. The corrected exact invocation exposed the fixture permissions failure.
- A first production CLI fixture appended a root TOML setting below the adapters table. Moving the fixture's socket setting to the document root fixed it.
- The first full gate caught the old sixteen-command inventory assertion. It now includes the six new commands; their generated handler/result paths are covered by conformance fixtures.
- The second gate caught a null optional issuer in a structural example. The selected ESS schema requires omission, so both production uninitialized status and its fixture omit that member. A production process test checks omission and rejects malformed/nil/noncanonical UUIDs before key-state mutation.
- The AEP proposed move initially refused the absent serves edge. The existing objective vision:independent-contract-adapters was related through the CLI before the legal draft → proposed → active moves. Three critic findings were fixed and recorded; all four second-round critics approve. Their reports cover revision two; the subsequent bounded CLI projection choice and conformance corrections retain the same outcome.

Raw generated help contains generator-owned trailing whitespace on its new group line; it is retained unchanged. Raw verification logs are also preserved without whitespace normalization.

After the full gate, the new contract's passive-status wording was clarified:
status creates no authority/key records, while existing SQLite locking and
sidecar bookkeeping remain permitted infrastructure work. Runtime/model inputs
are unchanged; the final website/reference checks use the clarified document.
`gate-source-inputs.sha256` retains the gate's exact source set.

The staged whitespace check also reports final blank lines in the eight immutable
AEP review records. Those CLI-owned records retain the returned review text; they
are not rewritten to silence a whitespace check. The authored-source check excludes
those records, generator-owned help and raw evidence logs.
