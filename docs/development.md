# Build, test and generate

Run these commands from the repository root. For service configuration and invocation,
see [Run connector services](running-services.md).

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
independent compilation of each internal adapter's model, the release-binaries check of the guides
(`connectors-build release-binaries`, below), the website examples model synthesis
(`connectors-build examples` without its WASM build), the drift check of the documentation
site's generated pages (`connectors-docs generate --check`) and AEP validation. It uses a task-owned temporary directory under
`.local/tmp`. The owner tests bind and connect Unix sockets inside it through a descriptor
on the state directory (`/proc/self/fd/<fd>/owner.sock`), as the host does, so a long
checkout path, such as a managed worktree under `~/.local/state/worktree/trees/…`, does not
push a socket path past `SUN_LEN`. The ignored Secret Service tests still give `dbus-daemon`
an absolute bus path and need a short `TMPDIR`. Only test binaries run under the gate's
temporary directory: the gate passes it to them through a Cargo runner for the host triple
that `rustc -vV` reports (`--config target."<host>".runner = ["sh", "-c",
"export TMPDIR=\"$0\"; exec \"$@\"", "<root>"]`), so neither the root nor a test binary's
path is parsed as an assignment or as shell text. Cargo, the compiler, a configured compiler
wrapper and build scripts keep the caller's `TMPDIR`; on Cargo older than 1.89 doctests do
too. An sccache server started during the gate therefore binds its startup socket under the
caller's `TMPDIR`, and a long checkout path does not push it past `SUN_LEN`; keep that
`TMPDIR` short. During the gate this runner replaces any runner the user configures for the
host: a command-line `--config` outranks `CARGO_TARGET_<TRIPLE>_RUNNER` and
`[target.<triple>] runner` in a configuration file, and Cargo does not consult
`[target.'cfg(…)'] runner` for a target whose triple has a runner. `CARGO_TARGET_DIR` selects the build output base; the MSRV check uses
its `msrv/` subdirectory (default `target/msrv`).

On macOS the default `TMPDIR` lies under `/var`, a symbolic link to `/private/var`, and the
host refuses a configuration or state path with a symbolic link in any component, so tests that
run `setup` under it answer `invalid_configuration`. Run the host and CLI tests there with
`TMPDIR` set to a directory whose path has no symbolic link, as
[`.github/workflows/macos-host.yml`](../.github/workflows/macos-host.yml) does with the
runner's temporary directory.

[`.github/workflows/rust-gate.yml`](../.github/workflows/rust-gate.yml) runs this
gate without `--msrv` on every pull request and every push to `main`. It installs
Rust 1.98.1 with rustfmt and Clippy, installs the pinned ESS and AEP releases after
checking each archive against its release's `SHA256SUMS`, fetches the locked
dependencies and runs `cargo run --locked -p connectors-build -- gate`. It reads the
repository with `contents: read` only and carries no credential. The `--msrv`
checks remain local.

The host's internal Entity Runtime/Eventlog adapter replays the complete recorded store
once per process and store; later opens and writes in that process read back
only what was appended since. Every verified read and every batch execution
still costs time that grows with the store, so a long-lived fixture can make
unoptimized dev/test binaries exhaust the synchronous bridge's 30-second
operation deadline. Both dev (including the actual CLI) and test profiles use
`opt-level = 1`; debug assertions and overflow checks remain enabled. The
release profile and the bridge deadline are unchanged.

## Pinned tools

The repository pins released tool versions, not source builds: ESS 0.59.0 in
[`crates/connectors-spec/toolchain.json`](../crates/connectors-spec/toolchain.json)
and AEP 0.71.3 in [`crates/connectors-build/aep-toolchain.json`](../crates/connectors-build/aep-toolchain.json).
Use the `ess` and `aep` executables the Beyond10x plugins install (`b10x upgrade`
keeps them current). The Cargo dependencies select Entity Runtime 0.30.3 and the
Eventlog release it builds on (0.8.3); ESS 0.59.0, like 0.56.0 and 0.57.0, additionally brings
Entity Runtime Core 0.28.0 through `ess-entity-runtime`. The CLI binding stays
`ess-cli/1` with all three globals, so the committed `ess-cli-plan/1` plan keeps its
bytes; ESS 0.57.0 and later generate a CLI runtime that reads an `ess-cli-plan/2` plan's optional
`config` and `output` globals too.

Since ESS 0.53.0, `ess generate cli` writes only into output it owns, recorded in a
git-ignored `apps/connectors-cli-contract/.ess-output/` ledger. A fresh checkout has
the generated bytes but no ledger, so `connectors-build cli` (without `--check`)
refuses with "unowned output destination". Enroll the committed bytes once per
checkout, then regenerate:

```sh
ess generate cli --path ess --binding apps/connectors/spec/cli.yaml --out <new empty directory>
ess generate output adopt --ownership-root apps/connectors-cli-contract --from <that directory> --owner cli-binding
```

Adoption refuses unless the committed files match the reference byte for byte, so
generate the reference from the same specification as the committed package.
`connectors-build cli --check`, which the gate runs, needs no ledger.

The gate selects `--ess`, then `CONNECTORS_ESS`, then `ess` on PATH, and likewise
`--aep`, then `CONNECTORS_AEP`, then `aep` on PATH. Both flags belong to
`connectors-build` itself and come before the subcommand
(`cargo run --locked -p connectors-build -- --aep <path> gate`); after it, as in
`gate --aep <path>`, they are refused as unknown arguments. A candidate is accepted
only when its `--version` names the pinned release exactly; an explicit selection
that does not refuses without fallback. Resolution never installs or replaces a
binary.

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
internal adapter document) keeps its tests against the frozen fixture
`crates/connectors-spec/tests/fixtures/gitlab-v2.json`, whose upstream pin names
`adapters/gitlab/upstream/openapi_v3.yaml`; no committed adapter selects it
today. Generation and its tests need the pinned ESS release and the recorded rustfmt
version, resolved as described under [Pinned tools](#pinned-tools).

Each internal adapter's default `service` feature adds its standalone executable and host
wiring. To embed only its contract implementation, disable default features:

```sh
cargo build --locked -p connectors-kubernetes --lib --no-default-features
```

The same command works for `connectors-sql` and `connectors-catalog-provider`. These libraries
depend on shared contracts/SDK and their protocol dependencies, with no host,
client, or sibling internal adapter dependency. The generic client and federation host have
no internal adapter dependencies. A new provider supplies an `Adapter` implementation and
configuration; it requires no provider switch in the shared client or host.

## Release binaries

[`release-binaries.yml`](../.github/workflows/release-binaries.yml) packages the binaries its
build job's `BINARIES` lists, `connectors` and `connectors-catalog-provider`; its `cargo build`
and `Package` steps iterate that list. `connectors-build release-binaries`, which the gate runs,
reads the list and refuses a guide that runs an installed binary outside it. The guides are
`README.md`, the Markdown files directly under `docs/` and the authored pages under
`website/docs/`. A guide runs an installed binary where, in a fenced `sh`, `shell`, `bash`, `zsh`
or `console` block, a workspace binary's bare name starts a command, or a path into an extracted
archive (`connectors-$version-$target/<name>`) names it; `target/release/<name>` is a source
build and is not one. Every listed name must be a workspace binary. To ship another binary, add
it to `BINARIES`, then document it.

```sh
cargo run --locked -p connectors-build -- release-binaries
```

## Ignored tests

`connectors-build ignored` inventories every compiled default-feature workspace
test executable with Rust's `--ignored --list`, then runs each selected test by
its exact name. Unknown classifications refuse all execution. The existing
`gate` command is unchanged; this lane is not automatically enrolled in CI.

A test that a `cfg` keeps off a platform (`#[cfg(unix)]`, `#![cfg(unix)]`,
`#[cfg(target_os = "linux")]` and the like) does not compile there, so no binary
on that platform lists it. The report's `platform_gated` field names each such
test from a scan of the test targets' sources: package, target, libtest name,
the platforms it runs on (`linux`, `macos`, `windows`) and the count. The scan
decides only `unix`, `windows`, `target_os` and `target_family`; tests a macro
expands to are not seen. A scanned test that runs on the current platform but is
missing from its compiled binary's list refuses the command.

```sh
cargo run --locked -p connectors-build -- ignored --inventory
cargo run --locked -p connectors-build -- ignored
cargo run --locked -p connectors-build -- ignored --required
```

The default `disposable` family runs private custody, CLI and loopback browser
fixtures. Workspace compilation supplies the current production CLI and catalog
provider paths. Custody requires `/usr/bin/dbus-daemon` and the exact qualified
GNOME Keyring artifact described in [Secret Service custody](local-secret-service.md).
Browser fixtures require `google-chrome-stable` on PATH. The compatibility fixture
also requires `CONNECTORS_ADVERSARY_PRE_HANDSHAKE`, an executable from before the
owner handshake change. Unavailable prerequisites are reported by test name.
Use `--required` for a release family: missing prerequisites, empty selection,
test failures and zero-test successes refuse acceptance. An ordinary exploratory
run may finish with explicit prerequisite skips; it is not release acceptance.

Live sandboxes and timing probes require explicit family selection, regardless
of which credentials happen to be present:

```sh
cargo run --locked -p connectors-build -- ignored --family live --required
cargo run --locked -p connectors-build -- ignored --family timing --required
cargo run --locked -p connectors-build -- ignored --family disposable,live,timing,deferred --inventory
```

The `live` family includes the public Roughtime interoperability check and the
prepared PostgreSQL and Kubernetes sandboxes. PostgreSQL requires
`CONNECTORS_PG_SANDBOX=127.0.0.1:port`, `CONNECTORS_PG_CONTAINER` naming the owned
PostgreSQL 17.6 fixture with local `psql`, and Docker on PATH. Kubernetes requires
`CONNECTORS_K8S_SANDBOX`, `CONNECTORS_K8S_CA` and `CONNECTORS_K8S_TOKEN`; its new
provisioning cases also require a private `CONNECTORS_K8S_KUBECONFIG` file and
`/usr/bin/kubectl`. CLI journeys need custody and the production CLI; native
PostgreSQL future-drop cancellation needs only its database observation fixture.
Selecting `live` authorizes those existing tests;
the runner does not provision sandboxes. The `deferred` family contains the known
CLI contract invariant limitation and can be explicitly run to observe its
current result. Subprocess helpers are always excluded from top-level execution;
their owning parent tests invoke them.

The two catalog settlement-failure cases additionally require Linux x86_64 with
seccomp `USER_NOTIF` and the supported notification ABI. The runner queries those
capabilities without installing a filter. The fixtures must then be permitted to
install a listener on their own CLI descendants and inspect those processes' file
descriptors; a runtime policy refusal is an explicit test failure. The filter
returns `EIO` only for the fixture's selected metadata files during the armed
settlement window, and continues ordinary system calls outside it.

The JSON report defaults to `.local/ignored-suites.json`; `--report <path>` selects
another destination. It records every fully qualified test name, family,
selection, outcome and skip reason, plus exact inventory, selected, executed,
skipped, failed, missing-prerequisite and unknown counts. Raw test output lives
beside it in the corresponding `.logs` directory, keyed by the SHA-256 of the
fully qualified name. Inventory mode executes no tests and cannot be combined
with `--required`. Each executed fixture has a ten-minute deadline; its owned
process group is stopped and reaped on success, failure, timeout or runner error,
with a bounded cleanup wait. A subprocess that deliberately creates a separate
session escapes that group and remains the fixture's shutdown responsibility.
Temporary
fixture files default to `.local/tmp`. For long managed checkouts, supply
`--tmpdir <short-directory>`: an actual directory owned by your user with mode
700, without symlinks. Custody checks refuse overly long temporary paths and
report this option as the missing prerequisite. The runner creates and removes
its own subdirectory there; the supplied directory, reports and logs remain.

## Live read loop

`connectors-build live-read-loop` is the sustained live acceptance run. For a bounded time, it
runs a declared set of reads through the `connectors` command line against live connections the
operator names. It issues no `connections revalidate` and takes no other step between calls.
It runs on demand on the operator's host, never in CI and never in the gate.

The reads are a `connectors-live-read-loop/1` file kept outside the repository, because it names
live connections. Each read names its instance in an `adapter` field, which the loop
passes as the deprecated `--adapter` selector:

```sh
jq -n '{format: "connectors-live-read-loop/1", reads: [
  {name: "slack-channels",
   adapter: "slack-bot",  # the instance, passed as the deprecated --adapter
   connection: "conn_…", operation: "conversations.list", input: {limit: 20}}]}' \
  > ~/connectors-live/reads.json
```

Each read is resolved once with `operations describe` (retried in the next round if that fails).
It is then invoked every round with `operations invoke`; those are the only two commands the
loop issues. A call is answered only when it exits 0 with `{"ok":true}`, any provider `status`
in the result is 2xx and the provider body is not `{"ok":false}`. Anything else is unanswered:
a refusal, a timeout, a non-JSON answer, or an exit status that contradicts the envelope.

```sh
cargo run --locked -p connectors-build -- live-read-loop \
  --reads ~/connectors-live/reads.json --report ~/connectors-live/report-$(date +%Y%m%dT%H%M%S).json \
  --connectors target/release/connectors --duration-seconds 600
```

`--config` and `--state-dir` are passed through to `connectors`. `--pause-seconds` (default 10)
sets the pause between rounds, and `--call-timeout-seconds` (default 60) sets when a call is
killed and counted unanswered. A round that has started finishes.

The `connectors-live-read-loop-report/1` report must be a new file outside the repository; both
conditions are checked before the first call. For every call, it records the read, phase,
round, start, latency, outcome and, for an unanswered call, the refusal: the envelope's `error`
object or a description of what happened instead. It never records an input, a result or any
other output. Per-read summaries and totals follow, along with the distinct commands issued
and the number of revalidations. The command fails when any call went unanswered, when any read
was never answered or when anything revalidated, and it still writes the report.

## Documentation website

The Docusaurus site under `website/` builds Connectors' own documentation site on the shared
Docs System shell: hand-written concepts, guides and pages on internal adapters, generated reference pages
and status, and Rust/WASM examples. Node is needed for the site, not for ordinary Cargo builds.
Follow [website setup and checks](../website/README.md).

`connectors-docs` generates the derived pages (the CLI reference, the crate list, the contract
and ESS model reference that `website/publication.json` selects, and the status page with its
data) and checks them; `connectors-build` builds the examples and audits a production build:

```sh
cargo run --locked -p connectors-docs -- generate
cargo run --locked -p connectors-docs -- generate --check
cargo run --locked -p connectors-build -- examples
cargo run --locked -p connectors-build -- docs-audit
```

`generate` needs the pinned ESS release, selected like the gate's (`--ess`, then
`CONNECTORS_ESS`, then `ess` on PATH). `--check` writes nothing and fails on a missing, stale,
hand-edited or orphaned generated file and on a page that breaks the page rules. `examples`
requires the installed `wasm32-unknown-unknown` Rust target and the example workspace's locked
dependencies in the Cargo cache. `docs-audit` checks an existing website production build. None
of them contacts a provider or deploys anything. The live preview starts with `npm start` from
`website/`.

The `Documentation validation` workflow (`.github/workflows/pages.yml`) runs the generated-page
check, the docs crate's tests and the site build on every pull request and push, binds the built
site to its commit with `connectors-docs provenance`, and on a push to `main` uploads it as the
artifact `b10x-project-site`. It deploys nothing.
