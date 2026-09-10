# GitLab native auth verification — 2026-09-10

This checkpoint implements GitLab-owned protected token parsing and baseline
identity/scope/expiry validation. It does **not** complete the persistent GitLab
CLI journey or the GitLab → Kubernetes → PostgreSQL goal. CLI acquisition,
qualified custody, connection publication and supervised invocation remain open.

## Inputs and changed behavior

Base: `c1edb5d4e713ac22fe7656825c590eeb795d52bb`, initially clean local main.
The commit containing this report owns the implementation. Exact changed input
hashes are in [implementation-sha256.txt](implementation-sha256.txt).
The ESS pin remains source `6f7ef46163e758f3401945d1a946e0fc80ebc003`, version
0.20.0. Its executable digest and receipt were checked before use.

The independent GitLab auth model validates and compiles as two files with six
declarations. It adds native values; shared connection/custody/generation ownership
is unchanged. The existing generated GitLab request model also validated before
changes. No generated file or vendor input was edited.

The native helper consumes the exact scoped HTTP capability supplied by its host
and checks `/user` and `/personal_access_tokens/self`. It rejects mismatched
identities, missing/insufficient scope evidence, inactive/revoked/expired tokens,
malformed responses, stale validation and backwards clocks. The entry type has
no Debug/Serialize, bounds the document/token and clears its owned buffers.
Shared SDK Secret now clears its owned bytes on drop; HTTP header assembly clears
its temporary buffer. Affected legacy/federation/conformance conversions keep
their existing result types while allowing the source Secret to be cleared.
Transport/client copies retain their own lifetimes; no universal erasure is claimed.

## Verification

With task-owned TMPDIR/CARGO_TARGET_DIR under `.local/tmp/gitlab-runtime-20260910`
and `CARGO_BUILD_JOBS=2`:

```sh
cargo test --locked --offline -p connectors-gitlab --test auth
cargo run --locked --offline -p connectors-build -- gate --msrv
```

Both exit 0. [Six native auth tests](auth-tests.log.gz) include a disposable HTTP
server reached through production ScopedHttp, asserting the captured token header
and both exact endpoint reads. Other cases exercise strict protected JSON,
identity disagreement, missing fields, native grants, UTC expiry, response errors,
size limits, no retry and clock/budget boundaries. These are deterministic local
fixtures, not a dedicated GitLab sandbox or CLI restart acceptance.

The [full gate](gate.log) passes shared ESS and independent native roots,
generation/drift checks, full workspace builds/tests/Clippy, adapter and generic
CLI boundaries, authored conformance synthesis, Rust 1.88 all-target checks and
AEP validation. The main compiler is Rust 1.98.1. Initial compiler failures exposed
Secret field moves in legacy consumers; those were corrected before this gate.
Two test fixture assumptions were corrected: a timestamp labeled midnight was
eight hours late, and a query with no parameters may retain an empty delimiter.
The final test expectations retain UTC and parameter semantics.

`connectors-build docs --check` exits 0: 41 contract pages, 90 reference pages,
no drift; see [output](docs-check.log). The new native auth contract/model is not
added to the website's explicit publication selection in this checkpoint; no
website presentation or browser example changes require browser tests.

Fresh [AEP validation](aep-validation.log) reports 154 artifacts, the same 82
historical prose-only review warnings, and valid. Its output was relayed verbatim.
The auth test log is retained losslessly with deterministic gzip because its raw
final blank line is rejected by the repository's whitespace check.

## Custody investigation and remaining work

Installed GNOME Keyring is `1:50.0-1`; GLib is `2.88.3-1`. The daemon executable
SHA-256 is `b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b`.
Upstream 50.0 resolves to `2ff8b070763ae025b90916a7b98643865819b451`. Inspection of
its [transaction source](https://gitlab.gnome.org/GNOME/gnome-keyring/-/blob/2ff8b070763ae025b90916a7b98643865819b451/pkcs11/gkm/gkm-transaction.c)
shows a file fsync followed by rename. The complete durability barrier and its
binding to the admitted service's actual storage remain unqualified. This is
not evidence that a successful D-Bus write can already acknowledge durable custody.
Exact downloaded source and hashes remain in the task's local evidence directory.
No existing secret or keyring item was read, written, deleted or migrated.

The next work remains protected-source preflight, private bootstrap, custody
qualification, serialized metadata publication and owner supervision. The GitLab
sandbox request remains unanswered and its existing credential blocker stays open.
The initiative and story remain active. A single writer updates AEP; no new
decomposition or critic panel is introduced for this continuation.
