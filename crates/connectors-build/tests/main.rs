//! This package's integration tests, built into one test binary. Each file is a module named
//! by its stem, so a test's name is the stem followed by the path it had as its own binary.
//! A new file here is built only once it is declared below.

mod adversary_docs_audit_literal;
mod catalog_amendment_model;
mod catalog_binary_model;
mod catalog_guard_model;
mod catalog_identity_model;
mod catalog_walk_model;
mod cli_spec_mapping_adversary;
mod cli_spec_mapping_adversary_pass2;
mod connection_vocabulary_authored_execution_adversary;
mod fresh_checkout_regeneration;
mod mcp_composition_provenance;
mod mcp_domain_model_adversary;
mod mcp_domain_model_adversary_pass2;
mod mcp_domain_model_census;
mod mcp_inbound_capability_projection;
mod mcp_inbound_local_binding;
mod mcp_inbound_local_binding_adversary;
mod mcp_inbound_local_binding_adversary_pass2;
mod mcp_inbound_mutation_replay;
mod mcp_outbound_auth_lifecycle;
mod mcp_outbound_connection_lifecycle;
mod mcp_outbound_connection_lifecycle_adversary;
mod mcp_outbound_connection_lifecycle_adversary_pass2;
mod mcp_outbound_invocation_results;
mod mcp_profile_selection_matrix;
mod mcp_profile_selection_matrix_adversary;
mod mcp_profile_selection_matrix_adversary_pass2;
mod mcp_specification_pin_adversary;
mod mcp_specification_pin_adversary_pass2;
mod metadata_conformance_fixture_adversary;
mod oauth2_declaration_mirrors_ess;
mod oauth2_refresh_metadata_authority_adversary;
