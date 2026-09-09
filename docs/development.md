# Build, test and generate

Run these commands from the repository root. For service configuration and invocation,
see [Run adapter services](running-services.md).

Rust 1.88.0 is the checked minimum. The full test/generation gate uses the ESS pin in
[`crates/connectors-spec/toolchain.json`](../crates/connectors-spec/toolchain.json)
and the rustfmt recorded in the GitLab generated manifest (currently Rust 1.98.1).
`--msrv` additionally checks all targets on installed Rust 1.88.0. The Rust gate runs
formatting, descriptor drift, offline builds/tests/Clippy, library dependency
boundaries, the [shared ESS provider boundary](../adapters/README.md),
independent adapter-model compilation and AEP validation. It uses a task-owned temporary directory under
`.local/tmp`. `CARGO_TARGET_DIR` selects the build output base; the MSRV check uses
its `msrv/` subdirectory (default `target/msrv`). This local repository has no
configured CI or publication target.

```sh
mkdir -p .local/tmp
TMPDIR="$PWD/.local/tmp" CARGO_BUILD_JOBS=2 cargo run --locked -p connectors-build -- gate --msrv
```

Normal builds consume checked-in generated files and need neither ESS nor networked
vendor-source refresh. GitLab uses the full v2 generation pipeline:

```sh
cargo run --locked -p connectors-spec -- \
  --generate \
  --specification adapters/gitlab/spec/adapter.json \
  --output adapters/gitlab/generated
```

Repeat with `--check` to verify the complete bundle. Generation and its tests need
the pinned ESS release and the recorded rustfmt version. The shared resolver checks
`--ess`, then `CONNECTORS_ESS`, then `.local/toolchains/ess/<version>/bin/ess`, then
each `ess` on PATH for the exact pinned version. Explicit paths must match; default
lookup skips other versions. It never installs a tool or changes PATH. See the
[toolchain setup](gitlab-generation.md#generate-and-check).
Kubernetes and SQL retain v1 descriptor
generation: omit `--generate` and select their `generated/descriptor.json` output.

Each adapter's default `service` feature adds its standalone executable and host
wiring. To embed only its contract implementation, disable default features:

```sh
cargo build --locked -p connectors-kubernetes --lib --no-default-features
```

The same command works for `connectors-gitlab` and `connectors-sql`. These libraries
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
