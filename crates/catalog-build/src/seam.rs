//! The wiring point between the build's orchestration and the compiler crate.
//!
//! `build` and `diff` are orchestration: discover providers, read committed bytes, load, compare,
//! write. The loading itself belongs to `connector-spec`, and this module is the only place that
//! crate's **entry points** are called. Loading is a pure function of bytes, which is why all IO
//! lives in [`crate::pipeline`]. (Other modules name the crate's *types* — the IR travels through
//! orchestration untranslated — but nothing else calls a loader or an ingest.)
//!
//! # The predecessor had a second wiring point, and it is gone
//!
//! `Connector` IR → `.flux` module + `.connector.toml` manifest was the other half, owned by
//! `connector-flux`. Design 02 §2 leaves the emitter behind: the reviewed artifact is the canonical
//! document ([`crate::document`]), the distributable one is the pack ([`crate::pack`]), and neither
//! is code. So there is one wiring point here, and the IR has exactly one lowering.
//!
//! # What spec ingest does and does not do here
//!
//! [`ProviderInputs`] carries the provider's whole spec cache and [`load`] hands it to
//! `connector_spec::provider::load_with_spec`, which resolves the file's `[spec] path` against it,
//! ingests that document, and publishes the operations `[[patch.operations]]` selects. Resolving
//! the pin picks exactly one document; one connector compiling from several at once is a separate
//! question the IR does not answer yet. See [`load`].

use anyhow::{bail, Result};

use crate::discovery::Provider;

/// A loaded, validated connector.
///
/// Re-exported rather than wrapped: nothing in this crate wraps the IR, so it travels through
/// orchestration untranslated.
pub use connector_spec::Connector;

/// The generator identity stamped into every artifact.
///
/// **`flux-connectors`, deliberately, and the version with it.** This string is embedded in every
/// canonical document and in every `connectors.lock` row, so it is an *input* to the one-time
/// migration differential design 02 §7 item 6 requires: our documents and pack, byte-identical to
/// the predecessor's at the same inputs. Renaming it rewrites all 55 documents and the pack digest,
/// which is a reviewed catalogue change and not a side effect of standing up a workspace. It moves
/// when the differential is retired, in a story whose diff is the whole catalogue.
///
/// It is also part of the hash domain `connectors.lock` records: a generator change must invalidate
/// generated output, or a stale artifact survives a codegen fix.
pub fn generator() -> String {
    format!("flux-connectors {}", env!("CARGO_PKG_VERSION"))
}

/// One provider's committed inputs, already read into memory.
///
/// Bytes, not paths, deliberately: it is what keeps [`load`] pure and lets `connector-spec` stay
/// fully unit-testable offline.
#[derive(Debug, Clone)]
pub struct ProviderInputs {
    /// The provider name.
    pub name: String,
    /// The contents of `providers/<name>.toml`.
    pub definition: String,
    /// **Every** vendored document under `specs/<name>/`, in discovery order.
    ///
    /// The whole cache rather than one chosen document, and the difference is a correctness one.
    /// `specs/<provider>/` holds more files than a connector compiles: versions of one document, and
    /// — since C-410 — the documents of the connector's other services. Only the provider file's
    /// `[spec]` / `[[spec]]` entries say which of them the connector is built from and how many.
    /// Handing the loader a document this layer had already picked would move that decision out of
    /// the file that owns it, which is what discovery's `Provider::spec()` did before C-410 deleted
    /// it: it returned the **last by file stem**, so pinning `specs/zendesk/2024-06-01.json` beside
    /// a newer `2025-01-01.json` compiled the newer one, successfully and with no diagnostic. So the
    /// pin is resolved where the pin is read.
    ///
    /// Populated for every provider whose cache directory holds anything, and *ingested* only when
    /// a `[spec]` entry asks for it — a distinction the loader makes, not this one (C-4).
    pub specs: Vec<SpecInput>,
}

/// A vendored spec document, already read into memory.
#[derive(Debug, Clone)]
pub struct SpecInput {
    /// The upstream version, from the cache file's stem.
    pub version: String,
    /// The repository-relative path, spelled as `[spec] path` spells it — which is what makes the
    /// pin resolvable rather than merely comparable to a version.
    pub path: String,
    /// The document's bytes.
    pub document: String,
}

impl ProviderInputs {
    /// Read everything discovery found for one provider.
    pub fn read(provider: &Provider) -> Result<Self> {
        let definition = crate::artifact::read(&provider.definition)?;
        let mut specs = Vec::new();
        for spec in &provider.specs {
            specs.push(SpecInput {
                version: spec.version.clone(),
                path: crate::workspace::spec_path(&provider.name, &spec.path),
                document: crate::artifact::read(&spec.path)?,
            });
        }
        Ok(Self {
            name: provider.name.clone(),
            definition,
            specs,
        })
    }

    /// How the definition names itself in an error — `providers/zendesk.toml`.
    ///
    /// `connector_spec::provider::load` uses its `name` argument only to label diagnostics, so the
    /// caller decides what an author sees. A path is what they can open.
    fn label(&self) -> String {
        format!("{}/{}.toml", crate::workspace::PROVIDERS_DIR, self.name)
    }
}

// ---------------------------------------------------------------------------------------------
// WIRING POINT 1 of 2 — C-3
// ---------------------------------------------------------------------------------------------

/// Parse and validate a provider's inputs into the connector IR.
///
/// This is `connector_spec::provider::load`: bytes in, a validated [`Connector`] out, no IO and no
/// network. Every rule an author can break — an unknown key, a missing method or path, a
/// requirement naming an undeclared credential — is diagnosed there, with *every* problem in the
/// file reported at once rather than one per run.
///
/// # A provider that points at a spec is compiled through ingest (C-4)
///
/// `connector_spec::provider::load_with_spec` is the spec front-end: it resolves each of the file's
/// `[spec]` / `[[spec]]` pins against the cache, ingests **those** documents into every operation
/// the vendor declares, publishes the ones `[[patch.operations]]` selects into the service each
/// document names, and validates the result through the same pass a hand-authored file goes through.
/// This function's whole job on that path is to hand it the cache discovery already read.
///
/// **This layer chooses nothing.** It passes every document under `specs/<provider>/` and lets the
/// loader resolve the pins, because which documents a connector compiles from — and how many — is
/// the provider file's decision and only the provider file's. Choosing here is precisely the defect
/// the resolution exists to prevent: discovery's `Provider::spec()` returned the **last by file
/// stem**, so a pin at `specs/zendesk/2024-06-01.json` beside a newer `2025-01-01.json` compiled the
/// newer one — exit 0, no diagnostic, an operation built from a document the file never named. For
/// babelforce's five documents it selected the four-operation `user` one over the 356-operation
/// `manager` one. C-410 deleted it rather than teaching it to choose better.
///
/// **A document is read only because the file asked for one.** A cache directory holding files is
/// not a declaration; `[spec] path` is. The loader makes that distinction, so passing the cache
/// unconditionally is safe and a provider with no `[spec]` block compiles exactly as it did before
/// C-4.
pub fn load(inputs: &ProviderInputs) -> Result<Connector> {
    Ok(load_reported(inputs)?.connector)
}

/// [`load`], keeping what the vendored document got wrong.
///
/// The split exists because a diagnostic is **not** a failure. A real vendor document is never fully
/// well-formed — an ingest that refused one would compile nothing — so a skipped endpoint has to
/// travel beside the connector rather than instead of it. What matters is that the cost stays
/// visible: each line names the endpoint and says what it cost, so a `select` that could never have
/// matched is legible before someone goes looking for the operation it names.
pub fn load_reported(inputs: &ProviderInputs) -> Result<Loaded> {
    let loaded = load_full(inputs)?;
    Ok(Loaded {
        diagnostics: loaded
            .diagnostics()
            .iter()
            .map(|diagnostic| format!("{}: {diagnostic}", inputs.name))
            .collect(),
        connector: loaded.connector,
    })
}

/// [`load_reported`], keeping the whole [`connector_spec::LoadedProvider`] rather than the two parts
/// a build needs.
///
/// `scaffold` (C-419) is the caller: it has to read the *declarations* — which documents the file
/// pins and with what provenance, which parameters a `[[patch.operations]]` block omits — and not
/// only the connector they compiled to. Those are exactly the fields a build has no use for, which
/// is why `load_reported` drops them and why this is a second function rather than a wider return
/// type on the one the pipeline calls.
pub fn load_full(inputs: &ProviderInputs) -> Result<connector_spec::LoadedProvider> {
    let label = inputs.label();
    // The whole cache, unfiltered. Which documents are compiled is the `[spec]` entries' decision
    // and the loader's to resolve — this layer picking one would be exactly the silent substitution
    // the resolution exists to prevent. A provider with no cache passes an empty slice rather than
    // taking a different code path, so "the pin resolves to nothing" is one refusal in one place
    // whether the cache is empty or merely missing the pinned file.
    let cache: Vec<connector_spec::SpecDocument<'_>> = inputs
        .specs
        .iter()
        .map(|spec| connector_spec::SpecDocument {
            path: &spec.path,
            document: &spec.document,
        })
        .collect();
    Ok(connector_spec::provider::load_with_spec(
        &label,
        &inputs.definition,
        &cache,
    )?)
}

/// Read one vendored document without a provider file to resolve it against — C-419.
///
/// The third and last call into `connector-spec`'s front ends, and the only one that is not part of
/// compiling a connector. `scaffold` needs it for the case the other two cannot serve: a document
/// sitting in `specs/<provider>/` that **no `[spec]` entry pins**, because the provider file does not
/// exist yet or does not name it. That is the whole point of scaffolding — the file that would pin
/// it is the output, not the input.
pub fn ingest(document: &str) -> Result<connector_spec::Ingested> {
    Ok(connector_spec::openapi::ingest(document)?)
}

/// One provider's IR, plus everything its vendored document got wrong — see [`load_reported`].
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The validated connector.
    pub connector: Connector,
    /// One line per endpoint the ingest could not read, already prefixed with the provider name.
    /// Empty for every hand-authored connector, which today is all forty-five of them.
    pub diagnostics: Vec<String>,
}

pub fn select_service(connector: &Connector, selector: &str) -> Result<Connector> {
    let available = connector.service_names();
    let matched = available
        .iter()
        .copied()
        .find(|name| *name == selector)
        .or_else(|| {
            available.iter().copied().find(|name| {
                connector
                    .gid_of(name)
                    .is_some_and(|gid| gid.to_string() == selector)
            })
        });

    let Some(service) = matched else {
        bail!(
            "connector `{}` has no service `{selector}`; {}",
            connector.id,
            describe_services(connector)
        );
    };

    Ok(Connector {
        services: connector
            .services
            .iter()
            .filter(|declared| declared.name == service)
            .cloned()
            .collect(),
        operations: connector.operations_of(service).cloned().collect(),
        // **Every member kind of the service, not just the callable one** (C-83). A selection that
        // stayed operations-only would be the worst kind of partial success: `--service s3` would
        // emit an s3 manifest carrying another service's events, and it would do so successfully.
        // The kinds partition the same way for the same reason — each member names exactly one
        // service — so one filter per kind is the whole rule. `config` and `graphs` arrived after
        // C-83 wrote that and were carried through by the tail below until C-194; each has had its
        // accessor since it landed.
        events: connector.events_of(service).cloned().collect(),
        channels: connector.channels_of(service).cloned().collect(),
        config: connector.config_of(service).cloned().collect(),
        graphs: connector.graphs_of(service).cloned().collect(),
        // `verify` is connector-level but *denotes* an operation, and an operation has exactly one
        // service — so it is service-derived, and the tail carried it through unnarrowed (C-194).
        // Neither "keep" nor "drop" is right on its own: kept across a boundary it names an operation
        // this connector no longer declares, which `connector_spec`'s own `validate_verify` refuses
        // on load; dropped unconditionally it would strip a legitimate Test-connection button from
        // the service that owns it. So it survives exactly when its operation does.
        verify: connector.verify.clone().filter(|id| {
            connector
                .operation(id)
                .is_some_and(|op| op.service == service)
        }),
        ..connector.clone()
    })
}

/// The services a connector has, with their addresses when it declares an authority — so the error
/// names both spellings a `--service` selector may use.
fn describe_services(connector: &Connector) -> String {
    let described: Vec<String> = connector
        .service_names()
        .into_iter()
        .map(|name| match connector.gid_of(name) {
            Some(gid) => format!("{name} ({gid})"),
            None => name.to_owned(),
        })
        .collect();
    format!("available services: {}", described.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A complete hand-authored connector, in the form an author writes it.
    const HAND_AUTHORED: &str = r#"
id = "acme"
vendor = "Acme"
base_url = "https://api.acme.example"

[[operations]]
id = "acme-ticket-show"
method = "GET"
direction = "read"
path = "/v2/tickets/{ticket_id}"
description = "Fetch one Acme ticket."
risk = "low"
idempotency = "idempotent"

[[operations.params.path]]
name = "ticket_id"
required = true
schema = { type = "integer" }
"#;

    /// A two-service provider, AWS-shaped: one authority, two hosts, one API date each.
    const TWO_SERVICE: &str = r#"
id = "aws"
vendor = "Amazon Web Services"
authority = "com.amazonaws"
base_url = "https://amazonaws.com"

[[services]]
name = "s3"
description = "Object storage."
base_url = "https://s3.amazonaws.com"
api_version = "2006-03-01"

[[services]]
name = "bedrock-runtime"
description = "Model inference."
base_url = "https://bedrock-runtime.us-east-1.amazonaws.com"
api_version = "2023-09-30"

[[operations]]
id = "aws-object-get"
service = "s3"
method = "GET"
direction = "read"
path = "/objects/{key}"
description = "Fetch one object."
risk = "low"
idempotency = "idempotent"

[[operations.params.path]]
name = "key"
required = true
schema = { type = "string" }

[[operations]]
id = "aws-model-invoke"
service = "bedrock-runtime"
method = "POST"
direction = "write"
path = "/model/{model_id}/invoke"
description = "Invoke a model."
risk = "medium"
idempotency = "non_idempotent"

[[operations.params.path]]
name = "model_id"
required = true
schema = { type = "string" }
"#;

    /// The same two-service shape, now declaring **every** service-partitioned surface: one
    /// `[[config]]` field and one `[[graphs]]` flow per service, plus a connector-level `verify`
    /// naming an s3 operation.
    ///
    /// Constructed rather than shipped, because no single shipped provider carries all three.
    /// `anthropic`, `contentful` and `postmark` are multi-service *and* declare per-service
    /// `[[config]]`, and `anthropic`, `contentful` and `microsoft_graph` declare a `verify` — those
    /// are covered against what ships, by
    /// `tests/service_units.rs::narrowing_a_shipped_provider_carries_no_other_services_config_graphs_or_verify`.
    /// **No provider in `providers/` declares `[[graphs]]` at all**, so a graph has no shipped case
    /// and only a fixture can assert it.
    ///
    /// It goes through the real loader, so the value a narrowing starts from is one `connector_spec`
    /// accepts — which is what makes "the narrowed value would not load" a statement about the
    /// narrowing rather than about the fixture.
    ///
    /// Each service's `base_url` carries **its own** template variable, bound by its own field. That
    /// is the detail that makes the leak observable rather than merely untidy: carry `region` into an
    /// s3-only connector and it binds `{region}`, which no remaining base URL has anywhere.
    const TWO_SERVICE_CONFIGURED: &str = r#"
id = "aws"
vendor = "Amazon Web Services"
authority = "com.amazonaws"
base_url = "https://amazonaws.com"
description = "Object storage and model inference."
verify = "aws-object-get"
default_auth = [{ credentials = ["aws.token"] }]

[[auth]]
name = "aws.token"
scheme = "bearer"
env = ["AWS_TOKEN"]

[[services]]
name = "s3"
description = "Object storage."
base_url = "https://{bucket}.s3.amazonaws.com"
api_version = "2006-03-01"

[[services]]
name = "bedrock-runtime"
description = "Model inference."
base_url = "https://bedrock-runtime.{region}.amazonaws.com"
api_version = "2023-09-30"

[[config]]
name = "bucket"
service = "s3"
label = "Bucket name"
help = "The bucket this connector reads objects from."
binds = "endpoint.bucket"

[[config]]
name = "region"
service = "bedrock-runtime"
label = "Model region"
help = "The AWS region hosting the models you invoke."
binds = "endpoint.region"

[[operations]]
id = "aws-object-get"
service = "s3"
method = "GET"
direction = "read"
path = "/objects/{key}"
description = "Fetch one object."
risk = "low"
idempotency = "idempotent"

[[operations.params.path]]
name = "key"
required = true
schema = { type = "string" }

[[operations]]
id = "aws-model-invoke"
service = "bedrock-runtime"
method = "POST"
direction = "write"
path = "/model/{model_id}/invoke"
description = "Invoke a model."
risk = "medium"
idempotency = "non_idempotent"

[[operations.params.path]]
name = "model_id"
required = true
schema = { type = "string" }

[[graphs]]
name = "object-fetch"
service = "s3"
description = "Read one object."

[[graphs.nodes]]
id = "get"
kind = { operation = { operation = "aws-object-get" } }
outputs = [{ name = "out" }]

[[graphs]]
name = "model-call"
service = "bedrock-runtime"
description = "Invoke one model."

[[graphs.nodes]]
id = "invoke"
kind = { operation = { operation = "aws-model-invoke" } }
outputs = [{ name = "out" }]
"#;

    /// A provider that points at a vendored spec and selects one operation out of it — the shape
    /// C-4 exists to compile.
    const SPEC_BACKED: &str = r#"
id = "acme"
vendor = "Acme"
base_url = "https://api.acme.example"

[spec]
path = "specs/acme/v1.json"

[patch.directions.default]
showTicket = "read"

[[patch.operations]]
select = "showTicket"
rename = "acme-ticket-show"
risk = "low"
idempotency = "idempotent"
"#;

    /// The vendored document `SPEC_BACKED` points at: two operations, one of which is selected.
    const SPEC_DOCUMENT: &str = r#"{
  "openapi": "3.0.3",
  "info": { "title": "Acme", "version": "1.0.0" },
  "servers": [{ "url": "https://{tenant}.acme.example" }],
  "paths": {
    "/v2/tickets/{ticket_id}": {
      "get": {
        "operationId": "showTicket",
        "summary": "Show one Acme ticket.",
        "parameters": [
          {
            "name": "ticket_id",
            "in": "path",
            "required": true,
            "schema": { "type": "integer" }
          }
        ]
      }
    },
    "/v2/tickets": {
      "get": { "operationId": "listTickets", "summary": "List Acme tickets." }
    }
  }
}
"#;

    fn inputs(definition: &str) -> ProviderInputs {
        ProviderInputs {
            name: "acme".to_string(),
            definition: definition.to_string(),
            specs: Vec::new(),
        }
    }

    /// The same inputs, with the vendored document discovery would have read beside them.
    fn spec_backed(definition: &str, document: &str) -> ProviderInputs {
        cached(definition, &[("specs/acme/v1.json", document)])
    }

    /// Inputs whose spec cache holds exactly the documents given, in the order discovery would
    /// have yielded them — sorted by file stem.
    fn cached(definition: &str, documents: &[(&str, &str)]) -> ProviderInputs {
        ProviderInputs {
            name: "acme".to_string(),
            definition: definition.to_string(),
            specs: documents
                .iter()
                .map(|(path, document)| SpecInput {
                    version: "v1".to_string(),
                    path: (*path).to_string(),
                    document: (*document).to_string(),
                })
                .collect(),
        }
    }

    /// The **other** document in the cache: same `operationId`, different request.
    ///
    /// Modelled on the real collision — `getUser` is `GET /api/v2/users/{id}` in babelforce's
    /// manager document and `GET /api/v2/user/me` in its user document.
    const OTHER_DOCUMENT: &str = r#"{
  "openapi": "3.0.3",
  "info": { "title": "Acme (other surface)", "version": "1.0.0" },
  "servers": [{ "url": "https://api.acme.example" }],
  "paths": {
    "/v2/user/me": {
      "get": { "operationId": "showTicket", "summary": "The wrong request entirely." }
    }
  }
}
"#;

    /// A complete hand-authored definition loads into the IR the rest of the pipeline lowers.
    ///
    /// The predecessor asserted this through the emitter — "both artifacts are produced" — and the
    /// emitter is gone. Loading is what is left of that wiring point, so this is what proves it is
    /// wired at all.
    #[test]
    fn a_hand_authored_definition_loads_into_the_ir() {
        let connector = load(&inputs(HAND_AUTHORED)).expect("the fixture loads");
        assert_eq!(connector.id, "acme");
        assert_eq!(connector.service_names(), vec!["default"]);
        let ids: Vec<&str> = connector
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect();
        assert_eq!(ids, vec!["acme-ticket-show"]);
        // Loading is pure and repeatable: the same bytes give the same IR, which is what the
        // determinism of every artifact below it rests on.
        assert_eq!(
            connector.canonical_json().unwrap(),
            load(&inputs(HAND_AUTHORED)).unwrap().canonical_json().unwrap()
        );
    }

    #[test]
    fn an_empty_definition_is_rejected() {
        let error = load(&inputs("   \n")).expect_err("empty definitions must not load");
        assert!(format!("{error:#}").contains("acme"));
    }

    /// The loader's diagnosis must reach the user with the file it is about, not be flattened into
    /// something this crate invented.
    #[test]
    fn the_loaders_own_diagnosis_survives() {
        let error = load(&inputs("id = \"acme\"\n")).expect_err("a connector needs a base URL");
        let rendered = format!("{error:#}");
        assert!(rendered.contains("providers/acme.toml"), "{rendered}");
        assert!(rendered.contains("base_url"), "{rendered}");
    }

    /// **C-4's wiring point.** A provider that points at a vendored document compiles: ingest turns
    /// the document into operations, the patch set says which of them are published, and what
    /// reaches the IR carries the parameters and schemas the document declared.
    ///
    /// This is the failing-first test the story names. Before C-4 it failed with "spec ingest
    /// (story C-4), which is not wired yet".
    #[test]
    fn a_spec_backed_provider_ingests_its_operations() {
        let connector = load(&spec_backed(SPEC_BACKED, SPEC_DOCUMENT))
            .expect("a spec-backed provider compiles once ingest is wired");

        let ids: Vec<&str> = connector
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec!["acme-ticket-show"],
            "only the selected operation reaches the IR"
        );

        let operation = &connector.operations[0];
        assert_eq!(operation.path, "/v2/tickets/{ticket_id}");
        assert_eq!(operation.description, "Show one Acme ticket.");
        let param = &operation.params.path[0];
        assert_eq!(param.name, "ticket_id");
        assert!(param.required);
        assert_eq!(param.schema, serde_json::json!({ "type": "integer" }));
    }

    /// **The pinned document is the document compiled**, even when a later-sorting one sits beside
    /// it in the cache.
    ///
    /// `discover_specs` was built for *versions of one document* and orders by file stem, so
    /// `2024-06-01.json` pinned beside a newer `2025-01-01.json` is the ordinary case, not an exotic
    /// one. A build that read the pin as a label and compiled the last file would emit a connector
    /// whose operations came from a document the provider file never named — exit 0, no diagnostic,
    /// plausible and wrong Flux. `AGENTS.md`'s "refuse ambiguous or unsafe output" is the rule it
    /// breaks.
    #[test]
    fn the_pinned_document_is_the_one_ingested_not_the_last_in_the_cache() {
        let connector = load(&cached(
            SPEC_BACKED,
            // Sorted as discovery yields them: the pinned `v1` first, a later stem after it.
            &[
                ("specs/acme/v1.json", SPEC_DOCUMENT),
                ("specs/acme/v2.json", OTHER_DOCUMENT),
            ],
        ))
        .expect("the pinned document is present, so this compiles");

        let operation = connector
            .operation("acme-ticket-show")
            .expect("the selected operation");
        assert_eq!(
            operation.path, "/v2/tickets/{ticket_id}",
            "the build compiled `showTicket` out of a document `[spec] path` does not name"
        );
    }

    /// A pin naming a document the cache does not hold is refused, and the refusal names both the
    /// pin and what is actually there — a message that names only the pin sends an author looking
    /// for a typo in the wrong file.
    #[test]
    fn a_pin_naming_an_absent_document_is_refused_and_lists_the_cache() {
        let error = load(&cached(
            SPEC_BACKED,
            &[("specs/acme/v2.json", OTHER_DOCUMENT)],
        ))
        .expect_err("`specs/acme/v1.json` is not in the cache");
        let rendered = format!("{error:#}");
        assert!(rendered.contains("specs/acme/v1.json"), "{rendered}");
        assert!(rendered.contains("specs/acme/v2.json"), "{rendered}");
    }

    /// **Ingest selects nothing.** A document with operations in it and a file that names none of
    /// them publishes none of them — selection is opt-in, and stays C-6's and C-411's to widen.
    #[test]
    fn a_spec_backed_provider_with_no_patch_publishes_no_operations() {
        let definition = "\
id = \"acme\"
base_url = \"https://api.acme.example\"

[spec]
path = \"specs/acme/v1.json\"
";
        let connector = load(&spec_backed(definition, SPEC_DOCUMENT))
            .expect("a spec with nothing selected is a connector with no operations, not an error");
        assert!(
            connector.operations.is_empty(),
            "ingest made two operations available and the file selected neither: {:?}",
            connector.operations
        );
    }

    /// Selecting a service yields a connector holding that service and nothing else — the property
    /// that makes `--service s3` mean "the whole s3 service".
    #[test]
    fn selecting_a_service_drops_every_other_operation() {
        let connector = load(&inputs(TWO_SERVICE)).unwrap();

        for selector in ["s3", "com.amazonaws/s3:2006-03-01"] {
            let selected = select_service(&connector, selector)
                .unwrap_or_else(|error| panic!("`{selector}` must select the s3 service: {error}"));
            assert_eq!(selected.service_names(), vec!["s3"]);
            let ids: Vec<&str> = selected
                .operations
                .iter()
                .map(|operation| operation.id.as_str())
                .collect();
            assert_eq!(ids, vec!["aws-object-get"]);
        }
    }

    /// **C-194.** The narrowing carries the selected service's surfaces and *nothing else* — the
    /// three the C-83 comment above [`select_service`] did not cover.
    ///
    /// The assertions are stated as the loader's own invariants rather than as field values, because
    /// that is what the leak actually broke: an endpoint-bound field for a `{variable}` the remaining
    /// base URL does not carry is refused by `validate_config`, and a `verify` naming an operation
    /// the connector does not declare is refused by `validate_verify`. Before the fix,
    /// `select_service` returned a `Connector` failing both.
    #[test]
    fn selecting_a_service_carries_no_other_services_config_graphs_or_verify() {
        let connector = load(&inputs(TWO_SERVICE_CONFIGURED)).unwrap();

        for (service, config, graph) in [
            ("s3", "bucket", "object-fetch"),
            ("bedrock-runtime", "region", "model-call"),
        ] {
            let selected = select_service(&connector, service).expect("a declared service");

            let names: Vec<&str> = selected
                .config
                .iter()
                .map(|field| field.name.as_str())
                .collect();
            assert_eq!(
                names,
                vec![config],
                "`--service {service}` carries another service's configuration fields"
            );

            let flows: Vec<&str> = selected
                .graphs
                .iter()
                .map(|flow| flow.name.as_str())
                .collect();
            assert_eq!(
                flows,
                vec![graph],
                "`--service {service}` carries another service's flow graphs"
            );

            assert!(
                selected.config.iter().all(|field| field.service == service),
                "`--service {service}` kept a configuration field naming another service"
            );
            assert!(
                selected.graphs.iter().all(|flow| flow.service == service),
                "`--service {service}` kept a graph naming another service"
            );
            if let Some(verify) = &selected.verify {
                assert!(
                    selected.operation(verify).is_some(),
                    "`--service {service}` kept `verify = {verify:?}`, an operation it no longer \
                     declares — a Test-connection pointer into a service this build is not producing"
                );
            }
        }

        // `verify` is connector-level but *denotes* an operation, and an operation has exactly one
        // service — so it is service-derived, and narrowing it is neither "keep" nor "drop". Both
        // directions are asserted, because dropping it unconditionally would strip a legitimate
        // Test-connection button from the very service that owns it.
        assert_eq!(
            select_service(&connector, "s3").unwrap().verify.as_deref(),
            Some("aws-object-get"),
            "the service owning the verify operation keeps it"
        );
        assert_eq!(
            select_service(&connector, "bedrock-runtime")
                .unwrap()
                .verify,
            None,
            "a service that does not declare the verify operation must not point at it"
        );
    }

    /// The other half of the same rule: what is **not** service-partitioned still comes through the
    /// `..connector.clone()` tail untouched.
    ///
    /// `AuthMethod` carries no `service` and an `AuthRequirement` names a credential connector-wide,
    /// so narrowing auth would need a reachability computation rather than a filter. That is a
    /// different story; this test is what says C-194 did not quietly take it.
    #[test]
    fn selecting_a_service_keeps_the_connector_level_surfaces() {
        let connector = load(&inputs(TWO_SERVICE_CONFIGURED)).unwrap();
        let selected = select_service(&connector, "s3").expect("`s3` is a declared service");

        assert_eq!(selected.auth, connector.auth);
        assert_eq!(selected.default_auth, connector.default_auth);
        assert_eq!(selected.id, connector.id);
        assert_eq!(selected.vendor, connector.vendor);
        assert_eq!(selected.description, connector.description);
        assert_eq!(selected.authority, connector.authority);
        // The connector-level default, not the service's own — a service resolves its base URL
        // through `base_url_of`, which the narrowed connector still answers correctly.
        assert_eq!(selected.base_url, connector.base_url);
        assert_eq!(
            selected.base_url_of("s3"),
            "https://{bucket}.s3.amazonaws.com"
        );
    }

    #[test]
    fn an_unknown_service_is_an_error_that_names_what_exists() {
        let connector = load(&inputs(TWO_SERVICE)).unwrap();
        let error = select_service(&connector, "s4").expect_err("`s4` is not a service");
        let rendered = format!("{error:#}");
        assert!(rendered.contains("s4"), "{rendered}");
        assert!(
            rendered.contains("s3 (com.amazonaws/s3:2006-03-01)"),
            "{rendered}"
        );
        assert!(rendered.contains("bedrock-runtime"), "{rendered}");
    }

}
