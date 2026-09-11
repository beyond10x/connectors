---
format: aep.planning-md/1
id: story:catalog-local-toml-action
kind: story
status: implemented
title: Expand a locally authored TOML action into the same template
relations:
- derived_from: specification:catalog-http-runtime-handoff
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: inferred
  path: crates/connectors-catalog/Cargo.toml
- confidence: cited
  path: crates/connectors-catalog/src/authored.rs
- confidence: cited
  path: crates/connectors-catalog/src/inventory.rs
- confidence: cited
  path: crates/connectors-catalog/src/template.rs
- confidence: cited
  path: crates/connectors-catalog/tests/authored.rs
revision: 11
---
## Outcome

A person writes a short TOML file naming an HTTP action and gets the same
validated template an imported OpenAPI operation produces. Nothing in the
authored path is a reduced second engine: the file is expanded into an
`inventory::Operation` and then into a `template::Template`, so every refusal
the imported path enforces applies to the authored one unchanged.

## Scope

`crates/connectors-catalog/src/authored.rs` and its tests, the `toml` dependency, and — corrected
after implementation — `src/inventory.rs`, `src/template.rs` and the root `Cargo.lock`. The file
declares a provider, an action name, a method, a path, and optional parameters with their location
and whether they are required, request media types, and an auth profile reference.

An authored document is read into the same `Operation` the inventory emits, and then handed to
`Template::from_operation`, which is the only place a template is built. Refused by name: an unknown
method, a path that does not begin with `/`, a parameter location the model does not carry, a
duplicate action name within one file, a parameter declared twice under one name and location, a
request media type offered twice, an `action` array declaring no action, a missing required field,
an unknown field, and a document over a bounded size. A template refusal is reported as itself
rather than rewritten. A document that exists but cannot be read is distinguished from one that is
absent.

`inventory::Location::parse` became crate-visible and `Location::label` replaced the private copies
of the same table in the authored reader and in the template, because a module whose claim is that
it is not a second engine cannot keep its own fork of a table the engine owns.

No credential, no host, no execution. The file names an auth profile; it does not carry one.

## Acceptance

A minimal authored file with one path parameter and one query parameter produces
a template that binds exactly as the equivalent OpenAPI operation would, proven
by building both and comparing the binding; each refusal case names its field or
value; an unknown field is refused rather than ignored; and a file declaring two
actions with one name is refused rather than keeping the last.

## Verification

`cargo test --locked -p connectors-catalog`, with the exit code in the report.
