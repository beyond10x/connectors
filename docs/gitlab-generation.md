# Generate and package the GitLab service

The active implementation is driven by
[the v2 adapter document](../adapters/gitlab/spec/adapter.json). It preserves
`project.get`, `issues.list`, and `file.get` from the configured service contract,
and adds `pipelines.list`, `pipeline.get`, `pipeline.jobs`, `job.get` and `job.trace`.
The [native CI profile](../adapters/gitlab/contracts/ci/v1alpha1/semantics.md)
owns exact-commit matching, job projection, paging and trace interpretation. The
trace mapping explicitly selects a 512000-byte HTTP prefix; its generated finish
obligation receives completeness with the retained bytes. Other mappings retain
the complete-response transport signature.
See [v2 semantics](../spec-kinds/adapter/v2/semantics.md) for generation boundaries
and [source provenance](../adapters/gitlab/upstream/README.md) for the exact vendor
input and its separate license.

## Generate and check

The single editable ESS toolchain pin is
[`crates/connectors-spec/toolchain.json`](../crates/connectors-spec/toolchain.json).
It records the exact ESS Git commit and its reported release version. A release
tag is not required: a reviewed current-main commit, including an explicitly
selected local development commit, can supply the generator. Use the rustfmt
version recorded in the generated manifest too.

The generator, build tool and standalone generation tests share one resolver:
`--ess` overrides `CONNECTORS_ESS`; otherwise it checks the checkout-local
`.local/toolchains/ess/<commit>/bin/ess` and then every `ess` on PATH in order.
Explicit paths must match the pin and never fall back; a bare executable name
searches PATH for the matching identity. An exact-source selection requires an
adjacent `ess.receipt.json`: its source repository, commit and version must match
the pin, and its SHA-256 must match the executable. The resolver checks this
receipt **before executing** the tool, then checks `--version`. A binary from an
older commit cannot pass merely because it prints the same release version.
Resolution returns an absolute path, skips mismatched candidates during default
lookup, and reports the pin file and searched locations if none matches. The gate
passes its resolved path to tests.

To build the selected source, first obtain and review a local ESS checkout at the
exact pinned HEAD. Commit or preserve tracked changes and nonignored untracked
files before building. Then run:

```sh
mkdir -p .local/tmp/ess-build
export TMPDIR="$PWD/.local/tmp/ess-build"
cargo run --locked -p connectors-build -- toolchain --source /path/to/ess
```

This explicit Rust command checks the clean source and exact HEAD, creates an
independent temporary local Git clone, and builds only its committed content with
`cargo build --locked --package ess-cli --bin ess --jobs 2 --profile dev --target-dir target`.
Ignored scratch in the original checkout cannot become source input. Submodules
are refused until a recursive source policy is specified. The build uses a private
target inside its snapshot, disables incremental compilation and debug information,
and retains the command's log beside the installed cache. A failed Cargo build
retains its workspace and reports the path for inspection.

The command installs the executable together with a receipt recording its digest,
source identity, lockfile digest, Rust/Cargo versions and build arguments. It
reuses only a verified existing cache; it refuses to overwrite an incomplete or
mismatched installation. The receipt is a local build observation, not a signed
upstream attestation or a claim that different build hosts produce identical
binaries. Build scripts and dependencies execute under the caller's authority;
reviewing the selected source remains part of choosing a toolchain.

Resolution itself does not download, build or change global tools. Historical
release-only pins of the form `{"ess":"x.y.z"}` remain readable and use
`.local/toolchains/ess/<version>/bin/ess`; only those records use version-only
matching and accept a separately checksum-verified official release without a
source receipt. The current pin selects exact source.

A pin change requires revalidating registered ESS roots and regenerating and
reviewing affected bundles, schemas, reference/example output, import refusals and
build evidence. Generated manifests record the portable source identity; the
host-specific binary digest stays in the local receipt. A test checks the GitLab
manifest against the source pin. To generate and verify:

```sh
mkdir -p .local/tmp
export TMPDIR="$PWD/.local/tmp"
cargo run --locked -p connectors-spec -- --generate \
  --specification adapters/gitlab/spec/adapter.json \
  --output adapters/gitlab/generated
cargo run --locked -p connectors-build -- check
```

The second command checks the entire generated bundle against its pinned sources,
then validates the repository ESS domain. Generation tests also check drift,
source/mapping refusals, output ownership and preservation of handwritten files.
Use `cargo test --workspace --locked` for the complete test suite.

Normal `cargo build --workspace --locked --offline` reads generated Rust and does
not require ESS, Docker or a source download. The adapter library can build with
`--no-default-features`; the generic CLI and host have no adapter dependency.
The private implementation ports are compile-time requirements. A compile-fail
example checks that omitting them cannot instantiate an executable adapter.

## Build a local image

The Rust executor consumes
[the local service declaration](../adapters/gitlab/realizations/local.json), checks
generation, builds the selected Cargo binary offline, and packages its runtime
libraries and CA trust bundle. The current profile is native **Linux x86_64**, with
a development Cargo build and no production performance claim.

```sh
cargo run --locked -p connectors-build -- package \
  --output .local/gitlab-package --image connectors-v2-gitlab:local
```

The output directory must be new. The executor records the complete source and
root-filesystem hashes, Cargo build log, Rust/ESS versions, image identity, an ESS
build graph, ESS-projected Dockerfile, and a validated physical realization tied
to the exact image and ESS source digest. A local repository name is required by
ESS's image-output schema; this executor never pushes an image or changes a registry.

The BuildKit projection consumes the compiler's original `build-ir.json` directly.
The former empty-`secrets` adaptation was removed after verifying that the current
reader accepts the unmodified compiler output with that optional field omitted.
The Docker packaging step uses `--network=none`; its source context contains the
binary, runtime dependencies, certificates and available system license notices.
The public GitLab specification is a generation input, not part of the service image.
The image uses the local host's exact runtime libraries; the manifest makes this
checkable but does not claim reproducible images across different build hosts.

## ESS upgrade reviewed on 2026-09-08

The pin was upgraded from 0.9.2 to the verified official **0.20.0** release
([release provenance and archive checksum](evidence/ess-toolchain-2026-09-08/release.json)).
The existing global 0.9.2 and 0.18.0 executables were preserved; the verified binary
is installed only in this checkout's versioned `.local/toolchains/ess/` cache.
The CLI calls now use `specify validate/compile/realization`,
`infra import openapi`, and `generate synthesize/build/project`.

A [trial regeneration](evidence/ess-toolchain-2026-09-08/upgrade-trial.log) into
`.local/ess-020-trial` preceded replacing the committed bundle. The complete
[reviewed diff](evidence/ess-toolchain-2026-09-08/trial.diff) changes only
`manifest.json` (ESS version and import-report hash) and `ess-import.json`.
The other 20 files, including generated Rust, descriptor, source selection,
coverage and typed IR, are byte-identical. ESS now stops at the OpenAPI 3.0.0
version refusal instead of also reporting downstream object-schema problems.
It still exits 1 with null import output. This is not successful vendor import;
the existing explicit Connectors mapping and response obligations still apply.
The vendor document and handwritten implementation were not edited to bypass it.

The subsequent gate, image build and direct/federated live GitLab results are
recorded in [verification](verification.md#ess-pin-upgrade--2026-09-08).

## Run and prove the container

Prepare a non-secret configuration from `examples/gitlab.yaml`, set the service
listener to `0.0.0.0:7101`, and point its caller credential at `/secrets/service.secret`.
For public upstream reads, `http.credential` is null. For private reads, use a
separate owner-only provider credential file and reference its mounted path.
The listener's public bind is inside the container; Docker publishes only loopback.

```sh
docker run -d --name connectors-v2-gitlab-test \
  --user "$(id -u):$(id -g)" --read-only --cap-drop ALL \
  --security-opt no-new-privileges --memory 256m \
  --publish 127.0.0.1:17101:7101 \
  --mount type=bind,src="$PWD/.local/gitlab-config",dst=/config,readonly \
  --mount type=bind,src="$PWD/.local/gitlab-secrets",dst=/secrets,readonly \
  connectors-v2-gitlab:local
```

The image defaults to UID/GID 65532. This local command uses the caller's numeric
UID/GID so the mounted secret passes the host's ownership check. The configuration
and credential directories are separate and absent from the image. Use unused
names/ports and do not overwrite or remove another run's resources.

Start the generic federation host with a single `gitlab` downstream using the
container's loopback endpoint. Then run:

```sh
cargo run --locked -p connectors-conformance -- \
  --gitlab-only --token-file .local/gitlab-secrets/service.secret \
  --gitlab http://127.0.0.1:17101/ --gateway http://127.0.0.1:17100/ \
  --allow-plaintext
```

This checks authenticated discovery, token refusal, all three reads, continuation,
and scope refusal directly and through federation against public GitLab HTTPS.
Private upstream credentials, malformed responses, schema refusals and pagination
edge cases use deterministic transport/HTTP fixtures. The broader Kubernetes/SQL
live recipe remains in [live-e2e.md](live-e2e.md).

After acceptance, send SIGTERM to owned services, verify exit code 0, and stop/remove
only the containers this run created. Retain the small build and acceptance reports;
the exact `rootfs` directory under this task's package output is reproducible storage.
