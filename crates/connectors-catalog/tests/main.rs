//! This package's integration tests, built into one test binary. Each file is a module named
//! by its stem, so a test's name is the stem followed by the path it had as its own binary.
//! A new file here is built only once it is declared below.

mod authored;
mod authored_adversarial;
mod bundle;
mod bundle_adversary;
mod bundle_adversary_2;
mod coverage;
mod coverage_adversarial;
mod definition;
mod definition_auth;
mod definition_base_path;
mod definition_feed;
mod definition_import;
mod definition_layout;
mod definition_paging;
mod definition_present_adversary;
mod definition_published_features;
mod definition_published_features_adversary;
mod definition_writes;
mod definition_writes_adversary;
mod discovery;
mod discovery_adversary_pass1;
mod discovery_adversary_pass2;
mod ingest;
mod inventory;
mod path_correction_adversary;
mod pipeline;
mod pipeline_adversarial;
mod server_path_adversary;
mod swagger;
mod template;
mod template_adversarial;
mod template_adversarial_two;
mod walk_import_adversary;
