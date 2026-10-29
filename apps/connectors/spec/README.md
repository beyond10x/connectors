# Local CLI presentation

[cli.yaml](cli.yaml) is the authored ESS presentation binding for the selected
`setup`, `instances`, `connections`, `operations`, `approvals` and `invoke`
commands. Types and behavior come from the
[CLI contract](../../../contracts/cli/v1alpha1/semantics.md) and
[shared ESS values](../../../ess/domains/cli.yaml).

Generate the separate contract-fixture package with:

```sh
cargo run --locked -p connectors-build -- cli
cargo run --locked -p connectors-build -- cli --check
```

The generated package lives at `apps/connectors-cli-contract`, beside the runtime
package. It is excluded from the enclosing Cargo workspace and consumed by
conformance tests and the production setup/inventory handlers. Nesting an independent workspace inside a member package
conflicts with Cargo's workspace membership rules.

The generated package uses recording handlers in conformance tests and an
unavailable handler in its standalone binary. The production `connectors` installs
its own handlers and refuses protected capture before an acquisition coordinator
is available. See the [implemented foundation](../../../docs/local-runtime-foundation.md).
Local keyring storage, connection management,
supervision and provider effects remain required production implementation work.
The runtime must also enforce selector relationships, operation schema identity,
current descriptor revisions, configured limits and all cross-record predicates;
structural type validation does not establish those facts.
