---
id: S-070
title: "A provider can hold a credential it cannot spend"
pillar: Catalog
status: done
priority:
design: ../design/16-subscription-credential-custody.md
epic: subscription-custody
areas: [connector-spec, catalog-build]
note: "design 16: the custody_only declaration kind. Today the loader refuses a provider with neither [spec] nor [[operations]], so custody-without-use cannot be expressed. The two workarounds available are both dishonest; this is the schema consequence."
---

# A provider can hold a credential it cannot spend

## Goal

Let a declaration say "this provider owns a credential and no request surface", so a credential
whose *use* belongs to another component can still have an owner, an address and a lifecycle here —
without any path by which connectors could spend it.

## Acceptance

- [x] `custody_only = true` is a provider-level declaration field, schema'd in
      `crates/connector-spec/schema/provider-toml.schema.json` and represented in the IR.
- [x] The loader **refuses**, with a named reason each, a `custody_only` provider that declares
      `[spec]`, `[[operations]]`, `[[services]]`, `base_url`, `verify`, `[[channels]]`,
      `[[events]]`, `[[discoveries]]`, `[[graphs]]`, `[patch]`, `const_headers`, `default_auth`, or
      an `auth.oauth2` block. Each is a refusal, not a silent allowance, so the kind cannot smuggle
      a half-declared ordinary provider past review. Asked of the **declared key**, so
      `base_url = ""` and `operations = []` are refused too.
- [x] The loader **requires** at least one `[[auth]]` on a `custody_only` provider — a provider that
      holds nothing has no reason to exist. `auth = []` is refused for the same reason.
- [x] The existing refusal at `crates/connector-spec/src/provider/validation.rs:31` is unchanged
      for every other provider, and its golden `tests/golden/nothing-to-generate.error` still
      matches byte for byte.
- [x] The invariant is parameterised over every provider setting the flag, in the one consolidated
      file `crates/catalog-build/tests/main/catalog_invariants.rs` —
      `a_custody_only_provider_publishes_no_surface_and_every_other_provider_does`, asserting both
      directions. **No per-provider test file** — AGENTS.md states the rule once and parameterises
      it.
- [x] `catalog build` emits a custody-only entry; `catalog diff` is clean twice; `catalog check`
      passes. The catalog document publishes `custody_only` and carries no service at all, rather
      than one implicit service with an empty base URL.

## Progress

- 2026-08-25 — landed. `custody_only` is on `Connector` and `ProviderFile`, `base_url` became
  optional at the serde layer, and `validate_custody_only` refuses every key that could describe an
  outbound request, by name, while requiring `[[auth]]`.
- In the hash domain, `skip_serializing_if` when false: `catalog diff` reports 68 artifacts up to
  date over 64 providers, so no provider that predates the kind moved.
- 2026-08-25, after independent review — three defects found and fixed, each of which would have
  made the story's Goal false:
  - **`catalog build` could not emit one.** `service_names` synthesises the implicit
    `DEFAULT_SERVICE` for a connector that declares none; that entry carries an empty `base_url`,
    and the document schema requires `minLength: 1`. The loader accepted the declaration and the
    build refused it one stage too late to name the file. A custody-only provider now lowers to no
    service at all, proved constructively by
    `document_tests.rs::a_custody_only_connector_renders_a_document_with_no_surface`.
  - **`[[channels]]` was not refused.** A channel binding carries its own `auth`, and
    `connector-resolve`'s channel composition places those resolved credentials onto the composed
    URL and headers. The only thing stopping it was an incidental empty-base-URL backstop. Every
    key that could describe a request is now enumerated in `CUSTODY_ONLY_REFUSED_KEYS`, and
    `auth.oauth2` is refused on the credential.
  - **Refusal was by emptiness, not by presence.** `#[serde(default)]` erases the difference, so
    `base_url = ""`, `operations = []`, `services = []` and `spec = []` all loaded. The check now
    reads the declared TOML keys, the way `implicit_service_members` already did.
- Also fixed while here: the schema's `$ref` to `#/$defs/authRequirements` named no definition
  (`authRequirement` is the one that exists), so the document failed to compile as a schema and
  every rule downstream of that `$ref` validated nothing. Pre-existing at `fee2d58`;
  `every_ref_resolves_to_a_declared_def` now catches it.
