# catalog

**The connector catalogue as typed `&'static` data, served from the embedded catalog pack.**

Part of [b10x/connectors](https://github.com/b10x/connectors). An internal workspace
member: nothing here publishes.

```rust
use catalog::{OperationKey, ProviderKey, Risk};

let show = catalog::operation(OperationKey::id("zendesk-ticket-show")).unwrap();
assert_eq!(show.risk, Risk::Low);
assert_eq!(show.provider, "zendesk");

// "every operation in this provider" is one call.
let zendesk = catalog::operations_of(ProviderKey::id("zendesk"));
assert!(zendesk.iter().all(|operation| operation.provider == "zendesk"));
```

## What it is

One deterministic JSON document per provider (`catalog/<id>.catalog.json`) is compiled by
`connectors catalog build` into one versioned, digest-carrying pack, which `catalog-reader` embeds.
This crate is the shim over that pack: it parses the documents once, behind a `OnceLock`, and hands
out the typed view a consumer decides with — risk, idempotency, credential placement, endpoint
configuration, channel bindings — plus `catalog::reader`, the reader re-exported whole.

There is no filesystem lookup at query time, no engine anywhere in the dependency graph, and no
secret in any of it: a credential declaration names a header, a prefix, a path leaf and
environment-variable *keys*, and there is no field a value could live in.

## What changed from the predecessor

The predecessor stored this surface as generated Rust — 16,909 lines of `&'static` tables under
`src/generated/`, plus 835 `ops/**/*.flux` renderings embedded with `include_str!` — because the
API promised one field the documents do not carry: `Operation::flux`, the emitted Flux text. The
emitter does not migrate, so:

- **`Operation::flux` is gone.** A request is derived from the document's request template by
  `connector-resolve`.
- **The table is built lazily from the pack**, not laid out by the compiler in `.rodata`. It is
  still `&'static`; the cost moved from compile time to one parse per process.
- **`OAuth2` carries no `client_id`.** The canonical document has no field for a registration
  value by design, and the generated table could only ever emit an empty string there.

Design 04 made both formerly implicit facts explicit in document data. Minting joins can construct
`Acquisition::Minted` from the canonical acquisition record, and each operation publishes its
`CredentialRequirement` token. The table parser maps those fields; it does not infer either rule
from predecessor behavior.
