# Private GitLab runtime checkpoint — 2026-09-10

The generic host can now launch an exact configured adapter artifact, check its
independently computed bootstrap, transfer protected material separately from
control JSON, and validate bounded results. GitLab's executable composition uses
that channel for native PAT validation and its three existing reads through
immutable authenticated HTTP capabilities. This is a working transport/composition
increment; production CLI connect/repair, owner supervision and the persistent
GitLab restart acceptance remain unfinished.

## Inputs and environment

Source parent: `796ecd5414704d80dd7838f513ca4c51fe5d3094`. The commit containing this
receipt identifies the increment. [Input hashes](implementation-sha256.txt) bind
the affected source roots, generated artifacts, native/shared model, dependencies
and support documents; unchanged inputs remain at the source parent. [Executable
hashes](executable-sha256.txt) identify the built CLI, native GitLab binary and
pinned ESS generator. [Tool versions](tool-versions.txt) record the observed Linux
x86_64 environment and Rust 1.88 compatibility toolchain.

The dependency lock only adds GitLab test edges to already selected sha2, hex,
rcgen and tokio-rustls packages; no dependency version or vendor input changed.
Native configuration semantics were modeled and validated before their dependent
implementation. The authored GitLab descriptor schema now accepts disjoint legacy
service and local effective configurations. Regeneration changed its descriptor
and manifest; generated Rust and CLI deliverables did not change. Shared ESS
compiles 382 declarations and the independent GitLab root compiles eight.

## Verification

Cargo commands used `CARGO_BUILD_JOBS=2`, `TMPDIR` at
`.local/tmp/gitlab-runtime-20260910` and `CARGO_TARGET_DIR` at its `target` child.

| Command | Result | Evidence |
|---|---|---|
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Passed: ESS ownership and compilation, generated parser/descriptors, workspace tests, conformance, Clippy, dependency boundaries and all-target Rust 1.88 check | [Full gate](repository-gate.log) |
| `cargo test --locked --offline -p connectors-host --lib local::runtime` | Six passed, including the fixture entry used by three actual adversarial child launches | [Host fixture log](private-channel-tests.log.gz) |
| `cargo test --workspace --locked --offline` within the gate | All three native GitLab process/TLS tests passed | [Extracted native test result](native-process-tests.log) |
| `cargo run --locked --offline -p connectors-spec -- --generate --specification adapters/gitlab/spec/adapter.json --output adapters/gitlab/generated` | Regenerated from owning inputs; subsequent gate drift and reproducibility tests passed | [Generation](generation.log) |
| `cargo run --locked --offline -p connectors-build -- docs --check` | 41 contract pages, 90 reference pages; no drift | [Documentation check](docs-check.log) |
| `npm run build` in `website` | Static build, examples, search and 459-file public-path audit passed | [Website build](website-build.log.gz) |
| `npm run typecheck` in `website` | Passed | [Typecheck](website-typecheck.log) |
| `aep plan artifact validate` | Valid; existing historical review warnings retained | [Verbatim validation](aep-validation.log) |

The first native fixture exposed the missing local configuration alternative in
the descriptor schema; that was corrected in the owning input and regenerated.
The fixture also needed ordinary HTTP routing to ignore an empty query delimiter.
A lost response can be observed as peer closure just before the parent's deadline,
or as the socket timeout itself. The test accepts those two transport observations
and requires the same closed channel and no repeated provider request. It does not
reinterpret either as successful validation.

Final review added parent-side result JSON/schema validation and exact request-ID
failure tests. The full gate was rerun on those final implementation inputs.
Website CSS minimization emits the existing missing-font-size warnings but completes;
no presentation code changed and browser interaction tests were not repeated.
The separate [qualified custody/registry evidence](../gitlab-registry-20260910/README.md)
is retained for its unchanged implementation inputs. Its environment-dependent
keyring fixtures were not rerun merely for the additive private transport.

## Runtime observations and limits

The host tests exercise separate protected framing, duplicate/unknown/trailing
control JSON, excess lengths/depth, truncated and stalled frames, immutable sealed
executable capture despite both in-place and path replacement, and failed writes
or resizing of the sealed capture. An actual owned Rust test child returns invalid
JSON, a wrong result field type or the wrong request ID; each case yields protocol
failure and proves the host killed and reaped that exact child.

The native fixtures use the real `connectors-gitlab` executable, a private TLS
listener and fictional tokens. Bootstrap performs no provider calls. Validation
keeps two credential identities separate, reports native scope implication and
refuses duplicate input. Project permission and cross-partition cursors refuse
before provider work. Project, issue-page and file reads succeed. A replaced CA
file cannot change the running capability, while a fresh launch detects the changed
configuration revision. A stale stop leaves the child usable; exact stop ends it,
and a later fresh child validates through the restored configuration. Lost response
ends channel ownership, and a subsequent call sends no provider request.

The [binding contract](../../../contracts/cli/v1alpha1/private-adapter.md) records
the process and protocol limits. A kernel policy that creates non-executable memfds
is refused. Child ownership is tied to a stable spawning thread and exact pidfd;
this is not yet the persistent owner with coalesced startup and durable suppression.
The generated Sources integration remains a specified next step, not implemented
capture. These fixtures supply no credential publication, real GitLab sandbox
acceptance, power-loss evidence, business-write execution or reproducible image
claim. Dynamic loader/system dependencies are not sealed by the executable capture.

`credential-blocker:gitlab-runtime-sandbox` remains open. The story, initiative and
goal remain active. Next wire reviewed local policy, the owner, protected source
admission, registry publication and exact retained-version invocation through the
production CLI; then prove connect/read/restart reuse. Provider order remains
GitLab, Kubernetes, PostgreSQL, MCP, then the remaining providers.
