# Full review response — 2026-09-08

All 17 findings in `.local/review/2026-09-08-full-review.md` have a disposition
below. Remediation follows `story:full-review-remediation`, after the locally
committed implementation at `19ce58b`. The original review remains unchanged.

| # | Disposition | Change or evidence |
|---|---|---|
| 1 | Fixed | `adapters/sql/tests/protocol.rs` adds loopback PostgreSQL wire fixtures through the public adapter API: credential authentication, input parsing and bounds before credential/connection access, native text/null rows, empty/exact/overflow page boundaries, row capacity, parameter-count errors, and sanitized SQLSTATE mapping. SQL validates its input schema at the library boundary too. |
| 2 | Fixed | HTTP disables built-in roots when `ca_file` is present; SQL starts an empty root store. Both accept explicit PEM bundles and refuse empty/invalid bundles. A real loopback TLS handshake tests matching/unrelated/default trust, while SQL's root-store test confirms that no public roots remain in custom-CA mode. |
| 3 | Fixed | Federation refreshes the stale leaf and publishes a complete validated snapshot before returning `stale_description`. The caller can then describe and deliberately resubmit. No invocation is replayed. Refresh failures preserve the prior snapshot and return the actual failure. `Adapter::invoke_at` carries the admitted revision into atomic route selection. A running-leaf replacement test verifies changed routes, stale revision refusal, zero replay and successful deliberate resubmission. |
| 4 | Fixed | Request and operation identifiers use escaped Debug fields in tracing. A captured-log regression sends newline, carriage-return and ANSI identifiers, verifies one safe log line and confirms zero dispatch. |
| 5 | Fixed in contract | The wire contract enumerates the fixed body, concurrency, deadline, row and cursor bounds. It no longer advertises configuration overrides; request page/row limits can be smaller. |
| 6 | Retained and documented | One connection and read-only transaction per request remains intentional for this bounded local profile: fresh credential resolution and discarded session state, with connection-setup latency accepted. No pool or throughput claim is made. README and the contract now document `search_path = public, pg_catalog` and explicitly qualified names for other schemas. Pooling is not part of this remediation. |
| 7 | Fixed | Rust `ServiceConfig`, `HttpConfig` and `CredentialRef` plus their schema annotations are the single editable owners. The host's optional schema feature derives them; adapter declarations use closed, versioned offline imports. The compiler expands those imports into self-contained descriptors and includes the expansion in revision digests. All three descriptors and the GitLab manifest were regenerated. Unknown imports and preserved sibling constraints/instance data are tested. |
| 8 | Fixed | Replaced the deprecated direct YAML parser with `serde_yaml_ng` 0.10.0; configuration YAML/JSON strictness and the full pinned upstream import pass. Replaced `rustls-pemfile` with rustls's `pki_types::pem::PemObject`. Neither old package remains in the lockfile. Audit: 306 dependencies, zero vulnerabilities, zero warnings. |
| 9 | Fixed | CLI input uses the core duplicate-rejecting JSON reader and is parsed before credential resolution or service contact. Process-level tests reject duplicate keys at the root and nested levels. |
| 10 | Fixed | Projected routes share an `Arc<Leaf>` containing one descriptor and transport per downstream. They do not copy a full leaf descriptor per operation. |
| 11 | Fixed | A credential-free `Endpoint` transport is retained in routes; authenticated `Client` values exist only for individual calls. File-rotation tests cover invocation and stale-snapshot refresh. |
| 12 | Fixed | The design's opening status now identifies the verified local implementation and unreleased status, consistent with sections 27–28. |
| 13 | Environment difference clarified | This remediation shell reports `ess 0.9.2`; the reviewer reported `ess 0.18.0`. The reproducibility pin remains 0.9.2. Version mismatch errors identify both versions and the supported `--ess`/`CONNECTORS_ESS` selection. The gate propagates the selected executable into generation tests. Documentation describes the explicit upgrade/regeneration requirement. |
| 14 | Superseded by completed implementation | `crates/connectors-build` exists in `19ce58b`, and its check/package commands were verified in the preceding story. This remediation also builds it and adds the gate command. |
| 15 | Fixed | Removed the duplicate `tempfile` dev-dependency from `connectors-spec`. The gate creates and automatically removes a private temporary directory under `.local/tmp`; README also sets TMPDIR for its initial Cargo bootstrap. No global temporary settings are changed. |
| 16 | Investigated; no corrupt history found | AEP's markdown projection hydrates internal IDs afresh per invocation. Its event reader selects by the stable event `entity` and `id`, not the historical `args.target`. Both stories' explanation outputs show the intended lifecycle and their own test evidence. Historical events were not rewritten. Details below. |
| 17 | Fixed locally | `cargo run --locked -p connectors-build -- gate --msrv` replaces the manual gate list. It runs format, descriptor drift, offline builds/tests/Clippy, independent adapter builds, dependency-boundary checks, optional minimum-Rust checking, ESS and AEP validation. CI remains unconfigured because this repository has no remote or rollout target. |

## Verification

The [complete gate log](evidence/review-2026-09-08/gate.log) records **35 passing
tests, zero failures and zero ignored tests**, warning-denying workspace Clippy,
all three generated descriptors, the complete GitLab generation/reproducibility
suite, offline workspace builds, independent provider libraries, and the generic
CLI's dependency boundary. All targets also pass `cargo +1.88.0 check`; workspace
packages now explicitly inherit the declared `rust-version = "1.88"`.
Tests execute on Rust 1.98.1; the minimum-version result is a compile check of
all targets, not a second runtime test run. The
[audit report](evidence/review-2026-09-08/audit.json) records the advisory database
revision and no vulnerabilities or warnings.

AEP initially reported a missing machine-readable scope while still returning
`valid`; it was added through `aep plan artifact scope`. Final validation and
story explanations are retained beside the gate log. All planning-store changes
used the AEP CLI. This was interactive single-agent work in the primary checkout,
as directed by the project; no approval bypass records were needed.

These are local protocol/TLS fixtures and repository checks. This remediation
does not claim another live GitLab/k3s/PostgreSQL acceptance run or a rebuilt
container image. Earlier [live and image evidence](verification.md) remains tied
to its original source/binary digests. Private provider credentials and temporary
certificates are absent from the retained evidence.

## AEP journal investigation

Read-only source inspection used the AEP checkout at
`e27c84bd2f5e565a7974d889ee7e3d27dee872d1`:

- `crates/aep-backend-markdown/src/projection.rs`: the module contract says the
  backend opens per invocation; `MarkdownProjection::hydrate` seeds from the
  markdown documents and rebuilds the artifact-to-internal-ID map.
- `crates/aep-backend-markdown/src/provider.rs`, `events_raw`: history filtering
  compares `event.entity == entity && event.id == id`.
- `aep plan artifact explain story:gitlab-spec-service` and
  `aep plan artifact explain story:three-adapters-e2e` report their respective
  implemented revisions and test-result references.

The repeated `01MEM…` values in command arguments are process-local entity IDs,
not persistent artifact identities. Their reuse across invocations is not evidence
that one story was moved in place of the other. No AEP changes are required here.

## Dependency and schema references

The chosen YAML fork describes its compatibility intent in the
[maintainer's repository](https://github.com/acatton/serde-yaml-ng).
The [rustls-pemfile advisory](https://rustsec.org/advisories/RUSTSEC-2025-0134.html)
and [supported PEM reader](https://docs.rs/rustls-pki-types/latest/rustls_pki_types/pem/trait.PemObject.html)
explain the parser replacement. Shared configuration schema generation uses
[Schemars's explicit draft and inline settings](https://docs.rs/schemars/1.2.2/schemars/generate/struct.SchemaSettings.html).
The offline import contract is documented in
[adapter v1 semantics](../spec-kinds/adapter/v1/semantics.md#shared-configuration-schemas)
and applies to v2 as well.
