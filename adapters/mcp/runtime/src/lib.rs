//! Native MCP protocol mechanics. Host admission and credential custody are ports
//! supplied by application composition, never authority inferred from wire input.

pub mod configuration;
pub mod framing;
pub mod json;
pub mod launch;
pub mod lease;
pub mod names;
pub mod results;
pub mod supervision;
