//! The `connectors` command's own argument definitions, shared by the binary and the
//! documentation generator. The grouped command tree is generated from the ESS CLI binding
//! (`connectors-cli-contract`); only the explicit service commands are declared here, and
//! [`help`] presents the generated tree with what the binding format cannot declare.
pub mod compat;
pub mod help;
