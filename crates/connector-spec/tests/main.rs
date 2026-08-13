//! **One integration-test binary for this crate** (C-533).
//!
//! Every `.rs` file directly under `tests/` is its own crate, which Cargo links into its own
//! executable carrying the entire dependency graph — measured at 179 files, 792 executables and
//! 30 GB in `target/debug/deps`, and the disk exhaustion that failed the v0.21.0 cut. The files
//! under `tests/main/` are therefore modules of this single test target. Each one is the same
//! documented argument it always was; only the linkage changed. Run one of them with
//! `cargo test -p <package> --test main <module>::`.
//!
//! The `#[path]` attribute on every declaration is load-bearing: this file is a crate root, and a
//! crate root resolves a bare `mod x;` in its **own** directory (`tests/`) — the same rule that
//! makes `tests/common/mod.rs` reachable from every root here — never in `tests/main/`.

// Shared test scaffolding, declared once for the whole binary and reached as
// `crate::<module>` from the test modules.
#[path = "../../connector-address/tests/fixtures/origin_corpus.rs"]
mod origin_corpus;
#[path = "support/shipped_provider.rs"]
mod shipped_provider;

#[path = "main/auth_archetypes.rs"]
mod auth_archetypes;
#[path = "main/auth_hazard.rs"]
mod auth_hazard;
#[path = "main/auth_prefix.rs"]
mod auth_prefix;
#[path = "main/auth_workarounds.rs"]
mod auth_workarounds;
#[path = "main/channel_bindings.rs"]
mod channel_bindings;
#[path = "main/config_choices.rs"]
mod config_choices;
#[path = "main/config_fields.rs"]
mod config_fields;
#[path = "main/constant_headers.rs"]
mod constant_headers;
#[path = "main/credential_paths.rs"]
mod credential_paths;
#[path = "main/credential_response.rs"]
mod credential_response;
#[path = "main/credential_subject.rs"]
mod credential_subject;
#[path = "main/determinism.rs"]
mod determinism;
#[path = "main/execution_facts.rs"]
mod execution_facts;
#[path = "main/graphs.rs"]
mod graphs;
#[path = "main/ir_roundtrip.rs"]
mod ir_roundtrip;
#[path = "main/legacy_default_service.rs"]
mod legacy_default_service;
#[path = "main/lockfile.rs"]
mod lockfile;
#[path = "main/oauth2_acquisition.rs"]
mod oauth2_acquisition;
#[path = "main/oauth_token_endpoint.rs"]
mod oauth_token_endpoint;
#[path = "main/openapi_ingest.rs"]
mod openapi_ingest;
#[path = "main/operation_selection.rs"]
mod operation_selection;
#[path = "main/operation_spec_source.rs"]
mod operation_spec_source;
#[path = "main/operator_pinned_origin.rs"]
mod operator_pinned_origin;
#[path = "main/param_omission.rs"]
mod param_omission;
#[path = "main/produces_credential.rs"]
mod produces_credential;
#[path = "main/provider_schema.rs"]
mod provider_schema;
#[path = "main/provider_toml.rs"]
mod provider_toml;
#[path = "main/provider_toml_errors.rs"]
mod provider_toml_errors;
#[path = "main/repeatability_condition_elision.rs"]
mod repeatability_condition_elision;
#[path = "main/response_schema_coverage.rs"]
mod response_schema_coverage;
#[path = "main/semantic_effects.rs"]
mod semantic_effects;
#[path = "main/service_partition.rs"]
mod service_partition;
#[path = "main/service_roles.rs"]
mod service_roles;
#[path = "main/service_tags.rs"]
mod service_tags;
#[path = "main/services.rs"]
mod services;
#[path = "main/shared_endpoint_slot.rs"]
mod shared_endpoint_slot;
#[path = "main/shipped_providers.rs"]
mod shipped_providers;
#[path = "main/spec_backed_provider.rs"]
mod spec_backed_provider;
#[path = "main/strict_fields.rs"]
mod strict_fields;
#[path = "main/vendored_specs.rs"]
mod vendored_specs;
#[path = "main/verification_conformance.rs"]
mod verification_conformance;
