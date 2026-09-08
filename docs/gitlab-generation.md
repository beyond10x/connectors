# Generate and package the GitLab service

The active implementation is driven by
[the v2 adapter document](../adapters/gitlab/spec/adapter.json). It preserves
`project.get`, `issues.list`, and `file.get` from the configured service contract.
See [v2 semantics](../spec-kinds/adapter/v2/semantics.md) for generation boundaries
and [source provenance](../adapters/gitlab/upstream/README.md) for the exact vendor
input and its separate license.

## Generate and check

Use ESS **0.9.2** and the rustfmt version recorded in the generated manifest.
This is a reproducibility pin, not a claim about the newest installed ESS. The
review shell reported 0.18.0; the implementation/remediation shell reports 0.9.2.
If `ess --version` differs, pass `--ess /path/to/pinned/ess` to the generator or
build tool. The gate propagates this selection as `CONNECTORS_ESS` to tests;
standalone generation tests accept that environment variable too. An upgrade
requires regenerating and reviewing the bundle, import refusal and build evidence.
On a machine with a small system temporary filesystem, set `TMPDIR` to an owned
writable directory first:

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

ESS 0.9.2's build compiler omits an empty `secrets` field that its BuildKit reader
requires. `build-ir.json` preserves the original compiler result;
`build-ir.projectable.json` adds only the absent `secrets: []` for that pinned reader.
No build secret, credential, or network permission is introduced by this adaptation.
The Docker packaging step uses `--network=none`; its source context contains the
binary, runtime dependencies, certificates and available system license notices.
The public GitLab specification is a generation input, not part of the service image.
The image uses the local host's exact runtime libraries; the manifest makes this
checkable but does not claim reproducible images across different build hosts.

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
