# codewandler-connector-catalog-reader

Dependency-free reader for the connectors **catalog pack** — every connector's canonical
document, compiled into one embedded, versioned, digest-checked file.

The pack is built by [`catalog build`](https://github.com/b10x/connectors)
from the reviewed per-provider documents (`catalog/<name>.catalog.json`). This crate embeds the
pack that matches its own release and serves it with **zero non-optional dependencies**: no JSON
parser, no compression codec, no hash crate — the container is offset-indexed UTF-8 and the
SHA-256 check is vendored.

```toml
[dependencies]
codewandler-connector-catalog-reader = "0.26"
```

The library name is `catalog_reader`:

```rust
// The embedded pack: the catalogue this crate was released with.
let zendesk = catalog_reader::provider("zendesk").expect("shipped provider");
let document: &str = zendesk.document(); // canonical JSON, byte for byte

let show = catalog_reader::operation("zendesk-ticket-show").expect("shipped operation");
assert_eq!(show.provider(), "zendesk");
let record: &str = show.record(); // the operation's own JSON object

// A newer catalogue than this crate was built with, from a file:
let pack = catalog_reader::Pack::load("catalog.pack")?;
assert!(pack.provider("zendesk").is_some());
# Ok::<(), catalog_reader::Error>(())
```

Records are canonical JSON **text** — bring whatever JSON parser you already have. `Pack::load`
refuses a wrong container version, a wrong document schema version, or a digest mismatch before
serving a single record, each by name.

## Fetching a newer catalogue than the crate embeds

B10x has no pre-v1 release artifacts. Historical predecessor release assets are provenance
only; they are not a B10x distribution channel and consumers must not treat them as current
contract bundles.

The first supported distribution must follow architecture ADR 0019 and S-031: a private OCI
artifact, immutable digest pin, signed bundle manifest, and independently verified release origin.
Until that release path exists, use the embedded pack or a locally reviewed build. `Pack::load` can
read externally supplied bytes, but the caller must authenticate and digest-pin those bytes before
loading them. After that out-of-band verification, loading is one step:

```rust
let pack = catalog_reader::Pack::load("catalog.pack").expect("a verified pack");
let zendesk = pack.provider("zendesk").expect("shipped provider");
```

`Pack::load` verifies before it serves a single record, and every refusal is a named variant of
`Error`:

- `UnsupportedFormat { found }` — the pack declares a container format newer than this reader
  implements. Upgrade the reader; the file is not corrupt.
- `UnsupportedSchema { found }` — the pack's documents carry a schema version this reader does not
  serve. It fails closed rather than handing out records it cannot vouch for.
- `DigestMismatch { stated, computed }` — the pack's header digest is not the digest of its own
  content: truncation, corruption or a hand edit.
- `NotAPack`, `NotText`, `Malformed(_)`, `Io(_)` — not a pack, not UTF-8, structurally not a
  version-1 pack, or unreadable.

**Two checks, deliberately.** A future release digest authenticates the artifact as a whole and
belongs outside the file; the in-band header digest covers every byte after itself and detects local
corruption before serving. They are not comparable, and the in-band value is not an authentication
boundary because an author who can rewrite the payload can rewrite its header too.

For the typed `&'static` catalogue API (`providers()`, `operation()`, risk and idempotency enums,
embedded Flux), use
[`codewandler-connector-catalog`](https://crates.io/crates/codewandler-connector-catalog), which
re-exports this crate as `catalog::reader`.

License: MIT OR Apache-2.0.
