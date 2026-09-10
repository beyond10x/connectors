# Isolated write-generation checkpoint

The active `story:guarded-gitlab-merge` now has a closed v3 frontend, separate
read/private descriptor projections, ESS-generated write input/output types and
an immutable prepared request. Execution consumes both the request and its SDK
write capability. Native bindings classify applied, refused and unknown effects;
invalid safe output after a known applied effect remains Applied with an error.

The [generation contract](../../../spec-kinds/adapter/v3/semantics.md) states the
supported subset and ownership. Production adapters still select v1/v2. No
approved merge command, private protocol two, provider write, sandbox acceptance
or reproducible distribution is claimed by this checkpoint. Next is the private
prepare/commit exchange and its connection-bound approval/dispatch coordinator.

## Verification

Runs used `TMPDIR=$PWD/.local/tmp/gitlab-runtime-20260910`,
`CARGO_TARGET_DIR=$TMPDIR/target` and `CARGO_BUILD_JOBS=2`. Cargo runs were
serialized. Compressed logs beside this file preserve commands' complete output.

| Command | Result |
|---|---|
| `cargo test --locked --offline -p connectors-spec --test write_generation -- --nocapture` | Initial four generator tests pass, including three generated-consumer runtime tests and three compile-fail tests. The subsequent pinned GitLab merge import test passes too. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Pass on integrated main: shared ESS (23 files, 429 declarations), six independent native models, unchanged CLI/adapter generation, all workspace tests, Clippy, boundaries, Rust 1.88, conformance and AEP. The host suite has 103 passes and 18 explicitly ignored cases. All five write-generation tests pass. |
| `cargo +1.88.0 test --locked --offline -p connectors-spec --test write_generation -- --nocapture` | All five tests pass in 55.56 seconds, including the separately compiled consumer's three runtime tests and three compile-fail cases. |
| `cargo clippy --locked --offline -p connectors-spec -p connectors-sdk -p connectors-build --all-targets -- -D warnings` | Final source and strengthened tests pass. Authored-package formatting also passes. |
| Website `npm run typecheck`, `npm run build`; `cargo run --locked --offline -p connectors-build -- docs --check` | Pass. Build generates 46 contract pages and 99 total references, builds the Rust/WASM examples, indexes 120 pages and audits 487 public files. Reference output has no drift. |

The Rust 1.88 test used `$TMPDIR/target/msrv` and a task-owned PATH entry whose
`rustfmt` symlink selects the same Rust 1.98.1 formatter as the ordinary gate.
The generated consumer therefore exercises the existing generator inputs with
the minimum supported compiler. Its temporary workspace uses the repository's
locked dependency versions, checked against Cargo metadata, and its own target.

The fixture compares two isolated bundles and every legacy output against an
independent v2 generation. It verifies strict codecs, exact path/query/body
mapping, no transport during preparation, one send after success/refusal/lost
response, preservation of applied effect knowledge after output failure, and
legacy invoke refusal. Compile-fail consumers cannot use GET authority or reuse
requests/capabilities. Drift, unowned collisions and symlink destinations refuse;
authored files survive regeneration. Import tests refuse changed pins, omitted
requirements, wrong types/constants, unsupported schema meanings and references.
The GitLab check uses the existing source SHA-256
`f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530`;
it sends no provider request.

Conformance retains 315 scenarios (34 authored) and 22 existing synthesis
refusals. These are reported limitations, not evidence of the pending production
write coordinator. The dedicated GitLab sandbox and create/update atomic-head
blockers remain open. Full GitLab still precedes Kubernetes with Helm,
PostgreSQL, MCP and the remaining providers.

## Corrections and integrated baseline

The first generated ESS model refused an underscore in its component name. The
generator now emits a valid hyphenated component identifier; its model was
regenerated rather than patched. Standalone formatting of the authored consumer
fixture required `rustfmt --config skip_children=true`, because its generated
module paths exist only in the temporary consumer workspace.

An initial gate stopped on one collapsible conditional; the parser was corrected
and Clippy passed. A later gate completed code checks while a separate release
integration advanced main and temporarily exposed mismatched AEP journal/files
to the final validation. After that integration, AEP validated and the full gate
passed again on `f475e0b5610b1d7334094e5ab507cfa1aa030e45`. The release changes
and its history are preserved; this increment performs no source publication.

After the full gate, the v1 refusal test gained an explicit valid-v1 baseline so
unrelated v2 fields cannot mask a reader accepting writes. The final complete
write suite passes on Rust 1.88 and the affected Clippy/format checks pass. The
gate and final source manifests differ only in that strengthened test file;
implementation bytes are unchanged between those checks.

`source-inputs-gate.sha256.gz` and `source-inputs.sha256.gz` identify the 378
checked source inputs. Earlier logs and the pre-integration source manifest are
retained separately. The integrated lockfile is
`2e5536b428d4193128f6edd1ced2529a253ce5c3c870899e8f78b8ef91c04801`;
this increment changes no dependency pin. ESS remains source commit
`6f7ef46163e758f3401945d1a946e0fc80ebc003` (reported version 0.20.0).

Root is the sole writer of this implementation/planning increment on primary
main. The clean Atlas authority checkout matches remote main
`1e9ea6546fcecbc87335d9f407f17790c296cc4e`; its bot-wrapper digest is
`8645c7100bda1e1f5a8285aa93f7feed045779ddf503d440dc2df15615c1e6ee`.
No task-owned linked checkout was created. The separate release integration and
its worktree cleanup completed independently. Planning's full unedited final
output is retained in [aep-validation.log](aep-validation.log).
