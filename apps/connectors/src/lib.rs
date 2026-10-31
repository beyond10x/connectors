//! The `connectors` command's help, shared by the binary and the documentation generator. The
//! command tree is generated from the ESS CLI binding (`connectors-cli-contract`); [`help`] presents
//! it with what the binding format cannot declare. [`shape`] is the result shaping `invoke`
//! prints through, a pure function the MCP server and walks reuse. [`walk`] is the loop that
//! reads a declared walk's pages, which `invoke --walk` and the MCP server run.
pub mod help;
pub mod shape;
pub mod walk;
