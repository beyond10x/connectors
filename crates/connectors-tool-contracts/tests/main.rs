//! This package's integration tests, built into one test binary. Each file is a module named
//! by its stem, so a test's name is the stem followed by the path it had as its own binary.
//! A new file here is built only once it is declared below.

mod publish;
mod tavily_conformance;
