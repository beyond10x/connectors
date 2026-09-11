---
format: aep.planning-md/1
id: story:catalog-local-toml-action
kind: story
status: active
title: Expand a locally authored TOML action into the same template
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-catalog/Cargo.toml
- confidence: cited
  path: crates/connectors-catalog/src/authored.rs
- confidence: cited
  path: crates/connectors-catalog/tests/authored.rs
revision: 6
---
## Outcome

A person writes a short TOML file naming an HTTP action and gets the same
validated template an imported OpenAPI operation produces. Nothing in the
authored path is a reduced second engine: the file is expanded into an
`inventory::Operation` and then into a `template::Template`, so every refusal
the imported path enforces applies to the authored one unchanged.

## Scope

`crates/connectors-catalog/src/authored.rs` and its tests, plus the `toml`
dependency, which is already in the workspace lock. The file declares a provider,
an action name, a method, a path, and optional parameters with their location and
whether they are required, request media types, and an auth profile reference.

An authored document is read into the same `Operation` the inventory emits, and
then handed to `Template::from_operation`, which is the only place a template is
built. Refused by name: an unknown method, a path that does not begin with `/`, a
parameter location the model does not carry, a duplicate action name within one
file, a missing required field, an unknown field, and a document over a bounded
size. A template refusal is reported as itself rather than rewritten.

No credential, no host, no execution. The file names an auth profile; it does not
carry one.

## Acceptance

A minimal authored file with one path parameter and one query parameter produces
a template that binds exactly as the equivalent OpenAPI operation would, proven
by building both and comparing the binding; each refusal case names its field or
value; an unknown field is refused rather than ignored; and a file declaring two
actions with one name is refused rather than keeping the last.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
