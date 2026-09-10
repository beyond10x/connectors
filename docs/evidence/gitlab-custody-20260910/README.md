# Secret Service custody checkpoint — 2026-09-10

This checkpoint adds scoped immutable custody writes and exact reads to the host,
with an explicit filesystem durability barrier for the pinned GNOME Keyring
implementation. It does **not** complete `story:persistent-gitlab-journey` or make
saved connections usable through the CLI. The publication/retirement coordinator,
protected input, adapter bootstrap and owner supervision remain implementation
work. Dedicated GitLab sandbox acceptance is still pending.

## Inputs

Source parent: `2960b58023cab67944d393972da1a6c89879cc4c`. The commit containing
this receipt is the checkpoint source revision; no receipt embeds its own future
commit ID. [Implementation hashes](implementation-sha256.txt) identify the exact
affected inputs, including unchanged shared SDK and dependency pins.

Tests ran on Linux x86_64 and ext4. The recorded
[OS package versions](os-packages.txt) and [executable hashes](os-artifacts-sha256.txt)
identify the observed native runtime. The daemon is GNOME Keyring 50.0 from source
commit `2ff8b070763ae025b90916a7b98643865819b451`; the audited source-file hashes are
in [upstream-source-sha256.txt](upstream-source-sha256.txt), with their corresponding
upstream paths linked from the [binding decision](../../local-secret-service.md).
The binding checks the daemon digest at runtime and refuses other filesystem
families; package observations do not claim reproducible OS image construction.

## Commands and results

All Cargo commands used two jobs, with both `TMPDIR` and `CARGO_TARGET_DIR`
under `.local/tmp/gitlab-runtime-20260910` (the latter's `target/` child).

| Command | Result | Evidence |
|---|---|---|
| `cargo test --locked --offline -p connectors-host --lib local::keyring -- --include-ignored --nocapture` | Exit 0; four tests passed, including both opt-in native fixtures | [Runtime log](runtime-tests.log.gz) |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | Exit 0; all repository checks passed | [Gate log](gate.log) |
| `connectors-build docs --check` using the built task binary | Exit 0; 41 contract pages, 90 total reference pages, no drift | [Documentation log](docs-check.log) |
| `aep plan artifact validate` after the checkpoint evidence update | Exit 0; 154 artifacts, 82 existing prose-only review warnings, valid | [AEP output](aep-validation.log) |

The gate includes shared and independently compiled adapter ESS, generation/drift,
authored conformance, workspace tests, Clippy, dependency boundaries and Rust 1.88
all-target checks. No ESS/generated inputs, dependency versions or website
publication inputs changed. The gate leaves the native fixtures ignored; the
separate explicit run above supplies their actual evidence.

## Runtime observations and limits

The disposable fixtures created their own bus and daemon with an encrypted login
keyring. They used fictional credentials and an owned stdin password pipe; they
never opened, unlocked, read or modified the desktop keyring.

Verified: exact binary and filesystem admission; non-nil private identities;
bounded values including binary and maximum-size material; scope denial; missing
versions; immutable duplicate refusal; competing writes through independent
handles; unsafe file permissions; altered native metadata; an injected non-encrypted
file format; native write failure; and a failed post-write synchronization barrier.
Both possible-write failures returned uncertainty, with no automatic retry.

After killing the exact owned daemon, existing capabilities refused the dead
owner. A locked replacement refused custody. After an unlocked replacement,
previously written exact versions remained readable and a failed native write
remained missing. Physical deletion of a test-owned version survived a further
restart while another version remained readable. The unpublished uncertain
candidate remained separately identifiable; the test published no connection.

Native interoperation required an explicit `xdg:schema` attribute to prevent
GNOME from synthesizing a different schema during reload, and handling its fixed
`text/plain` reply label for opaque binary values. These are recorded in the
binding decision; the resolver still rejects unexpected metadata.

Physical deletion is exposed only to the qualification test. The production
retirement fence, trustworthy retention checks and active-use/recovery exclusions
are not implemented. `setup check` therefore continues to report persistent
custody qualification as failed. These tests prove process restart behavior on the
observed backend; they do not simulate power loss, certify storage hardware,
qualify another Secret Service implementation, or satisfy provider sandbox cases.

All fixture children were killed and reaped before their temporary paths were
removed. The shared build cache remains owned by the active implementation goal.
Connectors source stays local on `main`; this checkpoint publishes no provider,
MCP, Connectors, Atlas catalog or cloud changes.
