---
format: aep.planning-md/3
id: story:test-binary-per-package-rest
kind: story
status: draft
title: The other packages' integration tests build into one binary each
relations:
- decomposes: epic:zero-open-issues-and-store-defects
revision: 1
---
## Finding

story:test-binary-per-package built adapters/catalog's 118 integration test files into one `main`
target (120 test executables to 4; derived_instance_adversary stays separate because it sets
CONNECTORS_INSTALLATION process-wide). Every other package still builds one binary per file under
`tests/`: apps/connectors, crates/connectors-host, crates/connectors-build, the other adapters.

## Outcome

The remaining packages' integration tests build into one target per package, the same way, with a
test that keeps every file declared; a file that changes process-wide state keeps its own target
and says why.

## Acceptance

- Test executables per package before and after, from `cargo test -p <package> --no-run`, recorded
  here; the `-- --list --include-ignored` name sets equal before and after (module prefixes aside).
- crates/connectors-build/src/ignored.rs classifies every ignored test as before.
