//! **Every catalogue invariant, over the whole catalogue.**
//!
//! One file, parameterised by the committed tree. This replaces a pattern the predecessor grew and
//! that does not carry over: one test file per provider — `babelforce_coverage.rs`,
//! `zendesk_spec_selection.rs`, sixteen of them — each asserting one provider's version of a rule
//! that is really a rule about *every* provider. The cost was structural rather than cosmetic:
//! adding a connector meant adding a file, so the fifty-sixth provider is covered only if somebody
//! remembers, and the rule itself is stated sixteen times and can be sixteen different rules.
//!
//! Every assertion below iterates the catalogue. A new provider is covered the moment it exists,
//! and a rule has exactly one statement.
//!
//! # The invariants
//!
//! 1. [`the_committed_tree_is_a_fixed_point_of_a_build`] and
//!    [`two_plans_over_the_same_inputs_are_byte_identical`] — determinism, both directions: a
//!    rebuild writes nothing, and two independent plans agree byte for byte.
//! 2. [`every_canonical_document_validates_against_the_committed_schema`] — and against the
//!    *planned* schema, so a schema change and a document change cannot pass separately.
//! 3. [`the_pack_serves_the_committed_documents_byte_for_byte`] — every provider record is its
//!    committed document, and every operation record is a substring of it that reparses equal.
//! 4. [`the_lockfile_agrees_with_every_input_and_every_artifact`] — every hash recomputed from the
//!    bytes on disk, in both directions: no row without a provider, no provider without a row.
//! 5. [`ids_are_unique_in_every_namespace_they_share`] — operation ids globally, and the three
//!    inbound member kinds within one service.
//! 6. [`no_input_or_artifact_carries_a_credential_shaped_value`] — the declaration surface has no
//!    field a secret could live in, asserted over the bytes rather than over the types.
//! 7. [`spec_backed_coverage_holds_in_both_directions`] — for every provider that declares spec
//!    ingest: nothing published that no document declares, and nothing selected that is not
//!    published.
//! 8. [`a_full_build_leaves_no_orphaned_artifact`] — no committed file under an artifact root that
//!    the plan does not claim.
//! 9. [`the_document_carries_the_callers_contract`] — every operation stores the model-facing
//!    contract (S-001): a description, a lowered object `input_schema`, and a symbol on every
//!    declared parameter — with the two measured cases pinned by name (babelforce's dotted
//!    parameter, airtable's error-envelope description).
//! 10. [`the_contract_and_the_params_state_the_same_symbols`] — the two places the document states
//!     a symbol (each param's `symbol`, the contract's `input_schema` keys and `required` order)
//!     agree, so a lowering bug cannot ship a contract keyed by names the params do not carry.
//! 11. [`every_format_origin_field_lowers_to_the_origin_slot`] — a `format = "origin"` config
//!     field's bound variable lands on exactly `["origin"]` in every operation that carries it
//!     (S-001; predecessor C-538 open question 3), so a provider declaring it for a variable
//!     inside a larger authority cannot silently drop Origin→Host with nothing red.
//! 12. [`the_credential_requirement_agrees_with_the_auth_list`] — the stored token (S-001) is
//!     `declared` exactly when the effective `auth` list is non-empty; the empty side carries one
//!     of the two distinction tokens the old derivation could not tell apart.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use catalog_build::pipeline::{self, Plan};
use catalog_build::workspace::Workspace;
use serde_json::Value;

// ---------------------------------------------------------------------------------------------
// The subject
// ---------------------------------------------------------------------------------------------

/// The repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

/// A whole-catalogue plan over the committed tree. Writes nothing.
fn full_plan() -> (Workspace, Plan) {
    let workspace = Workspace::new(repo_root());
    let plan = pipeline::plan(&workspace, None).expect("the committed catalogue compiles");
    // A plan over an empty tree would satisfy almost everything below while asserting nothing.
    assert!(
        plan.providers.len() >= 55,
        "the committed catalogue is 55 providers; the plan covers {}",
        plan.providers.len()
    );
    (workspace, plan)
}

/// The planned bytes of one artifact, by repository-relative path.
fn planned<'a>(workspace: &Workspace, plan: &'a Plan, relative: &str) -> &'a str {
    let path = workspace.root().join(relative);
    plan.artifacts
        .iter()
        .find(|artifact| artifact.path == path)
        .map(|artifact| artifact.contents.as_str())
        .unwrap_or_else(|| panic!("no artifact is planned at `{relative}`"))
}

/// Every provider's canonical document, parsed, keyed by connector id.
fn documents(workspace: &Workspace, plan: &Plan) -> BTreeMap<String, Value> {
    plan.providers
        .iter()
        .map(|provider| {
            let text = planned(workspace, plan, &format!("catalog/{provider}.catalog.json"));
            let value: Value = serde_json::from_str(text)
                .unwrap_or_else(|error| panic!("`{provider}`'s document is not JSON: {error}"));
            (provider.clone(), value)
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// 1. Determinism
// ---------------------------------------------------------------------------------------------

/// **A rebuild over unchanged inputs writes nothing.**
///
/// The whole-tree form of "equal inputs produce byte-identical artifacts": every committed artifact
/// is exactly what the plan would write, so `catalog build` on a clean tree is a no-op. A single
/// stale artifact fails here by name.
#[test]
fn the_committed_tree_is_a_fixed_point_of_a_build() {
    let (workspace, plan) = full_plan();

    let stale: Vec<String> = plan
        .changes()
        .map(|artifact| {
            format!(
                "  {} ({:?})",
                workspace.display_path(&artifact.path).display(),
                artifact.change
            )
        })
        .collect();

    assert!(
        stale.is_empty(),
        "the committed tree is not a fixed point of a build; run `catalog build`:\n{}",
        stale.join("\n")
    );
}

/// **Two independent plans over the same inputs agree byte for byte.**
///
/// The other half of determinism, and the one a fixed-point test cannot see: a build whose output
/// depended on the scheduler, the filesystem's directory order or a hash seed could still be a
/// fixed point of *itself* while differing between machines. Design 02 §7 item 2 states this over
/// two builds; this is two plans in one process, which is what a test can do cheaply — the
/// cross-machine form is what the lockfile's committed hashes carry.
#[test]
fn two_plans_over_the_same_inputs_are_byte_identical() {
    let workspace = Workspace::new(repo_root());
    let first = pipeline::plan(&workspace, None).expect("the catalogue compiles");
    let second = pipeline::plan(&workspace, None).expect("the catalogue compiles again");

    assert_eq!(first.providers, second.providers);
    assert_eq!(
        first.artifacts.len(),
        second.artifacts.len(),
        "two plans produced different artifact counts"
    );
    for (a, b) in first.artifacts.iter().zip(&second.artifacts) {
        assert_eq!(a.path, b.path, "two plans disagree about artifact order");
        assert_eq!(
            a.contents,
            b.contents,
            "two plans produced different bytes for {}",
            workspace.display_path(&a.path).display()
        );
    }
    assert_eq!(first.diagnostics, second.diagnostics);
}

// ---------------------------------------------------------------------------------------------
// 2. The schema
// ---------------------------------------------------------------------------------------------

/// **Every canonical document validates against the schema the same build plans.**
///
/// Against the *planned* schema rather than the committed one, deliberately: a change that moved
/// both would otherwise be able to pass while the pair on disk disagreed for one commit. The
/// committed schema is covered too, because the fixed-point test above requires the planned schema
/// to equal it.
#[test]
fn every_canonical_document_validates_against_the_committed_schema() {
    let (workspace, plan) = full_plan();

    let schema_text = planned(&workspace, &plan, "catalog/connector-document.schema.json");
    let schema: Value = serde_json::from_str(schema_text).expect("the schema is JSON");
    let validator = jsonschema::validator_for(&schema).expect("the schema compiles");

    for (provider, document) in documents(&workspace, &plan) {
        if let Err(error) = validator.validate(&document) {
            panic!("`{provider}`'s canonical document does not validate: {error}");
        }
        // The two facts the schema cannot state about the file it is validating: that the document
        // knows which connector it is, and that the connector is the one the file is named after.
        assert_eq!(
            document["connector"].as_str(),
            Some(provider.as_str()),
            "`catalog/{provider}.catalog.json` describes a different connector"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 3. The pack
// ---------------------------------------------------------------------------------------------

/// **The pack serves the committed documents byte for byte.**
///
/// Read through `catalog-reader` — the code that actually serves the pack — rather than through a
/// parser written for this test, so the assertion is about what a consumer gets. Three directions:
/// every provider record is exactly its committed document; every operation record is a *substring*
/// of that document that reparses equal to the operation the document carries; and the pack carries
/// neither more nor fewer providers than the catalogue.
#[test]
fn the_pack_serves_the_committed_documents_byte_for_byte() {
    let (workspace, plan) = full_plan();
    let documents = documents(&workspace, &plan);

    let pack_text = planned(&workspace, &plan, "crates/catalog-reader/catalog.pack");
    let pack = catalog_reader::Pack::from_bytes(pack_text.as_bytes().to_vec())
        .expect("the planned pack verifies");

    let pack_ids: Vec<String> = pack
        .providers()
        .map(|provider| provider.id().to_owned())
        .collect();
    let planned_ids: Vec<String> = plan.providers.clone();
    assert_eq!(
        pack_ids,
        {
            let mut sorted = planned_ids.clone();
            sorted.sort();
            sorted
        },
        "the pack and the catalogue carry different providers"
    );

    let mut operations = 0usize;
    for provider in pack.providers() {
        let id = provider.id().to_owned();
        let committed = planned(&workspace, &plan, &format!("catalog/{id}.catalog.json"));
        assert_eq!(
            provider.document(),
            committed,
            "the pack's record for `{id}` is not its canonical document"
        );

        let document = &documents[&id];
        let declared = document["operations"]
            .as_array()
            .unwrap_or_else(|| panic!("`{id}`'s document carries no operations array"));
        let mut seen = 0usize;
        for record in provider.operations() {
            operations += 1;
            seen += 1;
            let sliced: Value = serde_json::from_str(record.record()).unwrap_or_else(|error| {
                panic!("`{}`'s pack record is not JSON: {error}", record.id())
            });
            let carried = declared
                .iter()
                .find(|operation| operation["id"] == sliced["id"])
                .unwrap_or_else(|| {
                    panic!("the pack carries `{}`, which `{id}` does not", record.id())
                });
            assert_eq!(
                &sliced,
                carried,
                "the pack's record for `{}` is not the object its document carries",
                record.id()
            );
            assert_eq!(record.provider(), id);
            assert_eq!(
                record.service(),
                carried["service"].as_str().unwrap_or_default()
            );
        }
        assert_eq!(
            seen,
            declared.len(),
            "the pack carries {seen} operations for `{id}` and its document carries {}",
            declared.len()
        );
    }
    assert!(
        operations >= 835,
        "the catalogue is 835 operations; the pack carries {operations}"
    );
}

// ---------------------------------------------------------------------------------------------
// 4. The lockfile
// ---------------------------------------------------------------------------------------------

/// **`connectors.lock` agrees with every input and every artifact, in both directions.**
///
/// The lockfile is the drift record, so the two ways it can lie are a row whose hashes no longer
/// describe the bytes, and a *missing* row — a provider it stopped knowing about, which would make
/// a future `catalog check` report the catalogue clean because it no longer looked.
///
/// The dropped artifact classes are asserted absent by name. The predecessor's lock carried a row
/// per emitted `.flux` module, per `.connector.toml` manifest, per per-operation rendering and per
/// generated Rust table; none of those artifacts exists here, and a lock that still named one would
/// mean the pipeline had grown an emitter back.
#[test]
fn the_lockfile_agrees_with_every_input_and_every_artifact() {
    let (workspace, plan) = full_plan();

    let text = planned(&workspace, &plan, "connectors.lock");
    let lockfile = connector_spec::Lockfile::parse(text).expect("the planned lockfile parses");

    let rows: BTreeMap<&str, &connector_spec::LockEntry> = lockfile
        .entries()
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();

    let providers: BTreeSet<&str> = plan.providers.iter().map(String::as_str).collect();
    let recorded: BTreeSet<&str> = rows.keys().copied().collect();
    assert_eq!(
        providers, recorded,
        "the lockfile and the catalogue disagree about which providers exist"
    );

    // The pack row: the digest of the pack this same plan compiled.
    let pack_text = planned(&workspace, &plan, "crates/catalog-reader/catalog.pack");
    let pack = lockfile.pack().expect("a full build records the pack");
    assert_eq!(pack.path, "crates/catalog-reader/catalog.pack");
    assert_eq!(
        pack.sha256,
        connector_spec::sha256_hex(pack_text.as_bytes()),
        "the lockfile's pack digest is not the pack this build compiles"
    );

    /// Artifact-path shapes this pipeline no longer produces. A lock naming one is a pipeline that
    /// grew an emitter back.
    const RETIRED: &[&str] = &[
        "connectors/",
        "crates/catalog/ops/",
        "crates/catalog/src/generated",
        ".flux",
        ".connector.toml",
        "assets/readme-snippet",
    ];

    for provider in &plan.providers {
        let entry = rows[provider.as_str()];

        // The input half: the provider declaration's own bytes.
        let definition = std::fs::read(workspace.providers_dir().join(format!("{provider}.toml")))
            .unwrap_or_else(|error| panic!("read `providers/{provider}.toml`: {error}"));
        assert_eq!(
            entry.toml_sha256.as_deref(),
            Some(connector_spec::sha256_hex(&definition).as_str()),
            "the lockfile's `toml_sha256` for `{provider}` is not the file on disk"
        );
        assert!(
            entry.generator.starts_with("flux-connectors "),
            "`{provider}`'s row records generator `{}`",
            entry.generator
        );

        // The artifact half: every recorded hash is the bytes this build would write, and every
        // artifact this build writes for the provider is recorded.
        assert!(
            !entry.artifacts.is_empty(),
            "`{provider}`'s row records no artifact at all"
        );
        for (key, hash) in &entry.artifacts {
            for retired in RETIRED {
                assert!(
                    !key.contains(retired),
                    "`{provider}`'s row records `{key}`, an artifact class this pipeline does not \
                     produce"
                );
            }
            let contents = planned(&workspace, &plan, key);
            assert_eq!(
                hash,
                &connector_spec::sha256_hex(contents.as_bytes()),
                "the lockfile's hash for `{key}` is not the bytes this build writes"
            );
        }
        assert!(
            entry
                .artifacts
                .contains_key(&format!("catalog/{provider}.catalog.json")),
            "`{provider}`'s row does not record its canonical document"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 5. Identity
// ---------------------------------------------------------------------------------------------

/// **Ids are unique in every namespace that shares one.**
///
/// Two namespaces, and they are different shapes:
///
/// - **Operation ids are global.** They are what a caller names an operation by, so a duplicate
///   across two providers would make one of the two unreachable through the pack's flat lookup.
/// - **Operations, events and channel bindings share one namespace per service.** A channel named
///   after an operation is the "event dressed up as a pollable op" confusion in identifier form.
#[test]
fn ids_are_unique_in_every_namespace_they_share() {
    let (workspace, plan) = full_plan();

    let mut global: BTreeMap<String, String> = BTreeMap::new();
    for (provider, document) in documents(&workspace, &plan) {
        let mut per_service: BTreeMap<(String, String), &'static str> = BTreeMap::new();

        let mut claim = |kind: &'static str, service: &str, name: &str| {
            let key = (service.to_owned(), name.to_owned());
            if let Some(previous) = per_service.insert(key, kind) {
                panic!(
                    "`{provider}` declares `{name}` twice in service `{service}`: as {previous} \
                     and as {kind}. The three member kinds share one namespace per service"
                );
            }
        };

        for operation in document["operations"].as_array().into_iter().flatten() {
            let id = operation["id"].as_str().expect("an operation id");
            let service = operation["service"].as_str().expect("an operation service");
            claim("an operation", service, id);
            if let Some(owner) = global.insert(id.to_owned(), provider.clone()) {
                panic!("operation id `{id}` is declared by both `{owner}` and `{provider}`");
            }
        }
        for event in document["events"].as_array().into_iter().flatten() {
            claim(
                "an event",
                event["service"].as_str().expect("an event service"),
                event["name"].as_str().expect("an event name"),
            );
        }
        for channel in document["channels"].as_array().into_iter().flatten() {
            claim(
                "a channel binding",
                channel["service"].as_str().expect("a channel service"),
                channel["name"].as_str().expect("a channel name"),
            );
        }
    }

    assert!(
        global.len() >= 835,
        "the catalogue is 835 operations; {} were seen",
        global.len()
    );
}

// ---------------------------------------------------------------------------------------------
// 6. No secret anywhere
// ---------------------------------------------------------------------------------------------

/// **No credential value reaches an input or an artifact — asserted at the positions one could
/// occupy, not by grepping the bytes.**
///
/// The declaration surface has no field a secret could live in: `env` and `user_env` name
/// environment-variable *keys*, a scheme names a header and a prefix, and the OAuth2 object is
/// closed against every registration value. That is a property of the types, and a property held by
/// construction is exactly the kind that stops holding quietly when a field is added — so it is
/// checked here over the emitted documents.
///
/// # Why this is not a grep
///
/// A text scan for token shapes was written first and it was wrong in the way that matters: it
/// reported `providers/slack.toml`'s *documentation* of the `xoxb-` prefix, Anthropic's `sk-ant-`
/// in a help string, and a vendor's own base64 example payload quoted inside a response schema.
/// Every one is documentation, and a gate that reports documentation is a gate that gets muted.
///
/// A credential value cannot be told from a plausible example by looking at it. What *can* be told
/// is **where** it would have to sit, so this walks those positions and requires each to hold the
/// kind of thing it is declared to hold. A description is not one of them.
#[test]
fn no_input_or_artifact_carries_a_credential_shaped_value() {
    let (workspace, plan) = full_plan();

    /// Keys the OAuth2 object may carry. A registration value — a client id, a client secret, a
    /// redirect URI an operator configured — is *unrepresentable* by design, and this is what makes
    /// "unrepresentable" a checked claim rather than a schema comment.
    const OAUTH2_KEYS: &[&str] = &[
        "endpoint",
        "token_endpoint",
        "authorize_path",
        "token_path",
        "scopes",
        "grants",
        "redirect",
        "public_client",
    ];

    let mut offences = Vec::new();
    for (provider, document) in documents(&workspace, &plan) {
        for credential in document["auth"].as_array().into_iter().flatten() {
            let name = credential["name"].as_str().unwrap_or("<unnamed>");

            // `env` and `user_env` are *keys*. An entry that is not shaped like an environment
            // variable name is the one way a value could have been written into this field.
            for field in ["env", "user_env"] {
                for entry in credential[field].as_array().into_iter().flatten() {
                    let key = entry.as_str().unwrap_or_default();
                    let shaped = !key.is_empty()
                        && key
                            .chars()
                            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                        && key.starts_with(|c: char| c.is_ascii_uppercase());
                    if !shaped {
                        offences.push(format!(
                            "{provider}: credential `{name}`'s `{field}` holds `{key}`, which is \
                             not an environment-variable name"
                        ));
                    }
                }
            }

            if let Some(oauth2) = credential["oauth2"].as_object() {
                for key in oauth2.keys() {
                    if !OAUTH2_KEYS.contains(&key.as_str()) {
                        offences.push(format!(
                            "{provider}: credential `{name}`'s OAuth2 object carries `{key}`, \
                             which the document has no field for. A registration value is the \
                             deployment's, never the catalogue's"
                        ));
                    }
                }
            }
        }

        // A field declared secret is a question a host asks an operator. A default or an example
        // for it is a value in the catalogue, whatever it is a value *of*.
        for field in document["config"].as_array().into_iter().flatten() {
            if field["secret"].as_bool() != Some(true) {
                continue;
            }
            let name = field["name"].as_str().unwrap_or("<unnamed>");
            for key in ["default", "example"] {
                if !field[key].is_null() {
                    offences.push(format!(
                        "{provider}: secret config field `{name}` carries a `{key}`"
                    ));
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "a credential value reached a position that must not hold one:\n  {}",
        offences.join("\n  ")
    );
}

/// `GET`, `POST`, … — the word a `METHOD /path` key is built from.
///
/// Matched exhaustively rather than derived from `Debug`: the key is compared for equality against
/// a diagnostic's own location string, and a derive attribute moving would silently make every
/// comparison false.
fn method_word(method: connector_spec::HttpMethod) -> &'static str {
    use connector_spec::HttpMethod;
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

// ---------------------------------------------------------------------------------------------
// 7. Spec-backed coverage
// ---------------------------------------------------------------------------------------------

/// **For every provider that declares spec ingest, coverage holds in both directions.**
///
/// The predecessor asserted this one provider at a time, with a hand-kept allow-list of gaps per
/// connector. Stated once, over every spec-backed provider, the rule has two halves that need no
/// list at all:
///
/// - **Nothing invented.** Every published operation's `METHOD /path` is one some vendored document
///   declares. This is what would catch a connector accreting a hand-authored operation beside a
///   spec-backed surface, where it would look reviewed and be unbacked.
/// - **Nothing silently dropped.** Every `[[patch.operations]]` entry names an `operationId` the
///   ingest really carries, and lands as a published operation — unless it declares `defer`, in
///   which case it must *not* be published. A selector that matches nothing is a coverage claim
///   that quietly failed, which is the failure the per-provider allow-lists existed to catch.
#[test]
fn spec_backed_coverage_holds_in_both_directions() {
    let workspace = Workspace::new(repo_root());
    let providers = catalog_build::discovery::discover(&workspace, None)
        .expect("the catalogue is discoverable");

    let mut spec_backed = 0usize;
    for provider in &providers {
        let inputs = catalog_build::seam::ProviderInputs::read(provider)
            .unwrap_or_else(|error| panic!("read `{}`: {error:#}", provider.name));
        let loaded = catalog_build::seam::load_full(&inputs)
            .unwrap_or_else(|error| panic!("load `{}`: {error:#}", provider.name));
        if loaded.ingested.is_empty() {
            continue;
        }
        spec_backed += 1;
        let name = &provider.name;

        // Everything the documents declare, ingest diagnostics included: an endpoint ingest could
        // not express is still an endpoint the vendor declared, and dropping it from the left-hand
        // side is how a coverage gate comes to compare a set against itself.
        let mut declared: BTreeSet<String> = BTreeSet::new();
        let mut operation_ids: BTreeSet<&str> = BTreeSet::new();
        for document in &loaded.ingested {
            for operation in &document.ingested.operations {
                declared.insert(format!(
                    "{} {}",
                    method_word(operation.method),
                    operation.path
                ));
                operation_ids.insert(operation.operation_id.as_str());
            }
            for diagnostic in &document.ingested.diagnostics {
                if let Some((head, path)) = diagnostic.location.split_once(' ') {
                    if path.starts_with('/') && head.chars().all(|c| c.is_ascii_uppercase()) {
                        declared.insert(diagnostic.location.clone());
                    }
                }
            }
        }

        // **Only the spec-derived operations.** A provider may be spec-backed *and* carry
        // hand-authored operations beside the ingest — `microsoft_graph` is the shipped case — and
        // those are reviewed as authored text rather than against a document. The join is the
        // connector's own provenance, which the loader fills on the patch path only, so an inline
        // operation cannot forge one.
        let invented: Vec<String> = loaded
            .connector
            .operations
            .iter()
            .filter(|operation| {
                loaded
                    .connector
                    .provenance
                    .operation_specs
                    .contains_key(&operation.id)
            })
            .map(|operation| format!("{} {}", method_word(operation.method), operation.path))
            .filter(|key| !declared.contains(key))
            .collect();
        assert!(
            invented.is_empty(),
            "`{name}` publishes spec-derived operations no vendored document declares: \
             {invented:?}"
        );

        // Which vendor `operationId` each published operation came from. Joining on this rather
        // than on the patch's `rename` is what makes the rule general: a connector may declare a
        // `[patch.naming]` rule instead of renaming one operation at a time, and the published id
        // is then *derived* — reproducing that derivation here would be a second implementation of
        // it, free to agree with the loader by luck.
        let published: BTreeSet<&str> = loaded
            .connector
            .provenance
            .operation_specs
            .values()
            .map(|source| source.operation_id.as_str())
            .collect();

        for patch in &loaded.patch.operations {
            assert!(
                operation_ids.contains(patch.select.as_str()),
                "`{name}` patches `{}`, which no vendored document declares. A selector that \
                 matches nothing is a coverage claim that quietly failed",
                patch.select
            );
            match &patch.defer {
                Some(reason) => assert!(
                    !published.contains(patch.select.as_str()),
                    "`{name}` defers `{}` — {reason} — and publishes it anyway",
                    patch.select
                ),
                None => assert!(
                    published.contains(patch.select.as_str()),
                    "`{name}` selects `{}` and publishes nothing derived from it",
                    patch.select
                ),
            }
        }
    }

    assert!(
        spec_backed >= 7,
        "only {spec_backed} providers declare spec ingest, so this asserted almost nothing"
    );
}

// ---------------------------------------------------------------------------------------------
// 8. Orphans
// ---------------------------------------------------------------------------------------------

/// **No committed file under an artifact root that the plan does not claim.**
///
/// The inverse of the fixed-point test: that one says every planned artifact matches the tree, this
/// says the tree holds nothing the plan forgot. A document whose provider was deleted, or a
/// rendering left behind by a dropped artifact class, is an orphan — a file that still validates,
/// still parses and describes nothing.
#[test]
fn a_full_build_leaves_no_orphaned_artifact() {
    let (workspace, plan) = full_plan();

    let orphans: Vec<String> = plan
        .orphans
        .iter()
        .map(|orphan| {
            format!(
                "  {} (under {})",
                workspace.display_path(&orphan.path).display(),
                workspace.display_path(&orphan.root).display()
            )
        })
        .collect();

    assert!(
        orphans.is_empty(),
        "committed files under an artifact root that no plan claims:\n{}",
        orphans.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// 9–12. The caller's contract (S-001)
// ---------------------------------------------------------------------------------------------

/// **Every operation stores the model-facing contract, and every parameter its symbol** (S-001).
///
/// The two measured cases that motivated the predecessor's C-552 are pinned by name: babelforce's
/// dotted `time.start` must carry the normalized symbol `time_start`, and `airtable-record-get`'s
/// contract description must be the one-line summary *extended* with the error-envelope sentence
/// — longer than the summary, stating where the vendor's error message lives.
#[test]
fn the_document_carries_the_callers_contract() {
    let (workspace, plan) = full_plan();
    let mut operations = 0;
    let mut symbols = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().into_iter().flatten() {
            operations += 1;
            let id = operation["id"].as_str().unwrap_or_default();
            let contract = &operation["contract"];
            // Present, not necessarily non-empty: an operation whose one-line summary is empty
            // and that declares no error envelope stores the empty description it has — the
            // contract carries what the declaration states, it does not invent prose.
            assert!(
                contract["description"].is_string(),
                "`{provider}`'s `{id}` stores no contract description"
            );
            let schema = &contract["input_schema"];
            assert_eq!(
                schema["type"].as_str(),
                Some("object"),
                "`{provider}`'s `{id}` stores a non-object contract input schema"
            );
            for param in operation["params"].as_array().into_iter().flatten() {
                symbols += 1;
                assert!(
                    !param["symbol"].as_str().unwrap_or_default().is_empty(),
                    "`{provider}`'s `{id}` parameter `{}` carries no symbol",
                    param["name"].as_str().unwrap_or_default()
                );
            }
        }
    }
    assert!(operations >= 835, "only {operations} operations checked");
    assert!(symbols >= 1518, "only {symbols} parameter symbols checked");

    let all = documents(&workspace, &plan);
    let babelforce = &all["babelforce"];
    let dotted = babelforce["operations"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|op| op["params"].as_array().into_iter().flatten())
        .find(|param| param["name"] == "time.start")
        .expect("babelforce declares the dotted `time.start`");
    assert_eq!(dotted["symbol"], "time_start");

    let airtable = &all["airtable"];
    let record_get = airtable["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|op| op["id"] == "airtable-record-get")
        .expect("airtable-record-get is shipped");
    let summary = record_get["description"].as_str().unwrap();
    let extended = record_get["contract"]["description"].as_str().unwrap();
    assert!(extended.starts_with(summary.trim_end_matches('.')));
    assert!(extended.len() > summary.len());
    assert!(extended.contains("A non-2xx response is returned as data"));
}

/// **The two places the document states a symbol agree** (S-001): the contract's `input_schema`
/// is keyed by exactly the declared params' symbols, its `required` list is those symbols in
/// declaration order, and no two parameters of one operation share a symbol. A lowering bug that
/// let the schema keys drift from the params would ship a contract a caller cannot satisfy.
#[test]
fn the_contract_and_the_params_state_the_same_symbols() {
    let (workspace, plan) = full_plan();
    let mut compared = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().into_iter().flatten() {
            let id = operation["id"].as_str().unwrap_or_default();
            let declared: Vec<&str> = operation["params"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|param| param["symbol"].as_str().unwrap_or_default())
                .collect();
            let unique: BTreeSet<&str> = declared.iter().copied().collect();
            assert_eq!(
                unique.len(),
                declared.len(),
                "`{provider}`'s `{id}` hands one symbol to two parameters"
            );
            let required: Vec<&str> = operation["contract"]["input_schema"]["required"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|value| value.as_str().unwrap_or_default())
                .collect();
            assert_eq!(
                required, declared,
                "`{provider}`'s `{id}` contract requires different symbols than its params declare"
            );
            let keys: BTreeSet<&str> = operation["contract"]["input_schema"]["properties"]
                .as_object()
                .map(|properties| properties.keys().map(String::as_str).collect())
                .unwrap_or_default();
            assert_eq!(
                keys, unique,
                "`{provider}`'s `{id}` contract is keyed by symbols its params do not declare"
            );
            compared += 1;
        }
    }
    assert!(compared >= 835, "only {compared} operations compared");
}

/// **A `format = "origin"` field's variable lands on exactly `["origin"]`** (S-001; the
/// predecessor's C-538 open question 3). The format promises "swap the whole authority"; a
/// variable bound inside a larger authority (`https://{v}.x/`) would lower to `host` instead, and
/// silently dropping Origin→Host is the failure this gate turns red. Asserted beside the loader's
/// own IR-layer refusal, over the artifact a consumer actually reads.
#[test]
fn every_format_origin_field_lowers_to_the_origin_slot() {
    let (workspace, plan) = full_plan();
    let mut checked = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for field in document["config"].as_array().into_iter().flatten() {
            if field["format"] != "origin" {
                continue;
            }
            let mut variables: Vec<&str> = Vec::new();
            for bound in std::iter::once(&field["binds"])
                .chain(field["also_binds"].as_array().into_iter().flatten())
            {
                let bound = bound.as_str().unwrap_or_default();
                if let Some(variable) = bound.strip_prefix("endpoint.") {
                    variables.push(variable);
                }
            }
            assert!(
                !variables.is_empty(),
                "`{provider}`'s origin field `{}` binds no endpoint variable",
                field["name"].as_str().unwrap_or_default()
            );
            for operation in document["operations"].as_array().into_iter().flatten() {
                let Some(endpoint) = operation["endpoint"].as_object() else {
                    continue;
                };
                for variable in &variables {
                    let Some(slots) = endpoint.get(*variable) else {
                        continue;
                    };
                    assert_eq!(
                        slots,
                        &serde_json::json!(["origin"]),
                        "`{provider}`'s `{}` lowers origin variable `{variable}` to {slots}",
                        operation["id"].as_str().unwrap_or_default()
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(
        checked > 0,
        "no origin binding was checked — the gate is blind"
    );
}

/// **The stored `credential_requirement` agrees with the effective `auth` list** (S-001):
/// `declared` exactly when the list is non-empty, and the empty side carries one of the two
/// distinction tokens. The distinction itself — declared-empty versus never-declared — is not
/// re-derivable from the document, which is the point; what is checkable is that the token and
/// the list never contradict each other.
#[test]
fn the_credential_requirement_agrees_with_the_auth_list() {
    let (workspace, plan) = full_plan();
    let mut checked = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().into_iter().flatten() {
            let id = operation["id"].as_str().unwrap_or_default();
            let token = operation["credential_requirement"]
                .as_str()
                .unwrap_or_default();
            let declared = !operation["auth"]
                .as_array()
                .map(Vec::is_empty)
                .unwrap_or(true);
            if declared {
                assert_eq!(
                    token, "declared",
                    "`{provider}`'s `{id}` authenticates but claims `{token}`"
                );
            } else {
                assert!(
                    token == "no-credential-required" || token == "no-credential",
                    "`{provider}`'s `{id}` has an empty auth list but claims `{token}`"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 835, "only {checked} operations checked");
}
