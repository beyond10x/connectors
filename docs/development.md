# Build, test and generate

Run these commands from the repository root. For service configuration and invocation,
see [Run adapter services](running-services.md).

Rust 1.88.0 remains the checked minimum for independent libraries. The local
Eventlog-backed CLI/host runtime is checked on Rust 1.91.0. The full
test/generation gate uses the ESS pin in
[`crates/connectors-spec/toolchain.json`](../crates/connectors-spec/toolchain.json)
and the rustfmt recorded in the GitLab generated manifest (currently Rust 1.98.1).
Planning validation uses the separate [AEP executable pin](../crates/connectors-build/aep-toolchain.json).
The protocol document pin in `.engineering/project.yaml` selects governance documents;
it does not identify an installed executable.
`--msrv` additionally checks the pure workspace surface on installed Rust 1.88.0
and the Eventlog-backed runtime roots on installed Rust 1.91.0. The Rust gate runs
formatting, descriptor drift, offline builds/tests/Clippy, library dependency
boundaries, the [shared ESS provider boundary](../adapters/README.md),
independent adapter-model compilation, the website examples model synthesis
(`connectors-build examples` without its WASM build) and AEP validation. It uses a task-owned temporary directory under
`.local/tmp`. The owner tests bind and connect Unix sockets inside it through a descriptor
on the state directory (`/proc/self/fd/<fd>/owner.sock`), as the host does, so a long
checkout path, such as a managed worktree under `~/.local/state/worktree/trees/…`, does not
push a socket path past `SUN_LEN`. The ignored Secret Service tests still give `dbus-daemon`
an absolute bus path and need a short `TMPDIR`. A compiler wrapper started under the gate's `TMPDIR` has the same limit: with `rustc-wrapper = "sccache"` in the Cargo configuration and no sccache server already running, run the gate from a long checkout with `RUSTC_WRAPPER=` set empty (story:gate-temporary-root-and-compiler-wrapper). `CARGO_TARGET_DIR` selects the build output base; the MSRV check uses
its `msrv/` subdirectory (default `target/msrv`).

[`.github/workflows/rust-gate.yml`](../.github/workflows/rust-gate.yml) runs this
gate without `--msrv` on every pull request and every push to `main`. It installs
Rust 1.98.1 with rustfmt and Clippy, installs the pinned ESS and AEP releases after
checking each archive against its release's `SHA256SUMS`, fetches the locked
dependencies and runs `cargo run --locked -p connectors-build -- gate`. It reads the
repository with `contents: read` only and carries no credential. The `--msrv`
checks remain local.

The local Entity Runtime/Eventlog adapter replays the complete recorded store
once per process and store; later opens and writes in that process read back
only what was appended since. Every verified read and every batch execution
still costs time that grows with the store, so a long-lived fixture can make
unoptimized dev/test binaries exhaust the synchronous bridge's 30-second
operation deadline. Both dev (including the actual CLI) and test profiles use
`opt-level = 1`; debug assertions and overflow checks remain enabled. The
release profile and the bridge deadline are unchanged.

## Pinned tools

The repository pins released tool versions, not source builds: ESS 0.45.0 in
[`crates/connectors-spec/toolchain.json`](../crates/connectors-spec/toolchain.json)
and AEP 0.65.0 in [`crates/connectors-build/aep-toolchain.json`](../crates/connectors-build/aep-toolchain.json).
Use the `ess` and `aep` executables the Beyond10x plugins install (`b10x upgrade`
keeps them current). The Cargo dependencies select Entity Runtime 0.25.1 and
Eventlog 0.6.0; ESS 0.45.0 additionally brings Entity Runtime Core 0.24.1 through
`ess-entity-runtime`.

The gate selects `--ess`, then `CONNECTORS_ESS`, then `ess` on PATH, and likewise
`--aep`, then `CONNECTORS_AEP`, then `aep` on PATH. A candidate is accepted only when
its `--version` names the pinned release exactly; an explicit selection that does
not refuses without fallback. Resolution never installs or replaces a binary.

Do not transcribe a `review-result` into a `verification-report` (practice stopped
2026-09-15). Record findings in the review-result itself, in its findings block;
author a `verification-report` only when a story or a gate actually consumes it.
The 52 existing drafts under `.engineering/planning/verification-report/` stay as
history and are neither deleted nor rewritten.

ESS 0.22 introduces generated-output ownership. On a checkout whose existing CLI
fixture has no local ownership record, generate a fresh reference and adopt only
byte-identical output before regenerating it. `--check` needs no adoption:

```sh
"$CONNECTORS_ESS" generate cli --path ess --binding apps/connectors/spec/cli.yaml \
  --out .local/tmp/cli-adoption-reference
"$CONNECTORS_ESS" generate output adopt --ownership-root apps/connectors-cli-contract \
  --from .local/tmp/cli-adoption-reference --owner cli-binding
cargo run --locked -p connectors-build -- cli
```

Use a fresh task-owned reference path. Set `CONNECTORS_ESS` to the pinned `ess` release (`command -v ess`). Preserve an adoption refusal and inspect the differing
bytes; never overwrite them to manufacture an ownership record.

The gate selects authored Cargo workspace members for formatting. `cargo fmt --all`
also follows excluded path dependencies, including generated CLI sources; use the
gate or `cargo fmt --package <authored-package>` to preserve generated bytes.

```sh
mkdir -p .local/tmp
TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked -p connectors-build -- gate --msrv
```

Normal builds consume checked-in generated files and need neither ESS nor networked
vendor-source refresh. Kubernetes and SQL use v1 descriptor generation: omit
`--generate` and select their `generated/descriptor.json` output. GitLab is served
by the catalog provider from a bundle compiled with `connectors-build catalog`;
see [the catalog provider guide](local-catalog-provider.md).

The v2 generator (`connectors-spec --generate`, ESS-backed Rust for a GET-only
adapter document) keeps its tests against the frozen fixture
`crates/connectors-spec/tests/fixtures/gitlab-v2.json`, whose upstream pin names
`adapters/gitlab/upstream/openapi_v3.yaml`; no committed adapter selects it
today. Generation and its tests need the pinned ESS release and the recorded rustfmt
version, resolved as described under [Pinned tools](#pinned-tools).

Each adapter's default `service` feature adds its standalone executable and host
wiring. To embed only its contract implementation, disable default features:

```sh
cargo build --locked -p connectors-kubernetes --lib --no-default-features
```

The same command works for `connectors-sql` and `connectors-catalog-provider`. These libraries
depend on shared contracts/SDK and their protocol dependencies, with no host,
client, or sibling adapter dependency. The generic client and federation host have
no adapter dependencies. A new provider supplies an `Adapter` implementation and
configuration; it requires no provider switch in the shared client or host.


## Documentation website

The local Docusaurus site combines authored guides, canonical contract views, ESS
model reference and Rust/WASM examples. Node is needed for the site, not ordinary
Cargo builds. Follow [website setup and checks](../website/README.md).

The Rust build tool owns its generation and public-output audit:

```sh
cargo run --locked -p connectors-build -- docs
cargo run --locked -p connectors-build -- examples
cargo run --locked -p connectors-build -- docs --check
cargo run --locked -p connectors-build -- docs-audit
```

`examples` requires the installed `wasm32-unknown-unknown` Rust target and the
example workspace's locked dependencies in the Cargo cache. `docs-audit` checks an
existing website production build. Neither command contacts a provider or deploys
anything. The live preview starts with `npm start` from `website/`.
