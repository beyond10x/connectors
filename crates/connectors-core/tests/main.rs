//! This package's integration tests, built into one test binary. Each file is a module named
//! by its stem, so a test's name is the stem followed by the path it had as its own binary.
//! A new file here is built only once it is declared below.

mod domain_refusal_adversary;
mod inline_schema_adversary;
mod lossless_json;
mod operation_curation;
mod operation_metadata;
mod reason;
mod reason_adversary;
mod wire;
mod wire_v1alpha2;
mod wire_v1alpha2_adversary;
mod wire_v1alpha2_keyed;
