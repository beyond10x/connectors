# Local CLI presentation

[cli.yaml](cli.yaml) is the authored ESS presentation binding for the selected
`setup`, `adapters`, `connections` and `operations` commands. Types and behavior
come from the [CLI contract](../../../contracts/cli/v1alpha1/semantics.md) and
[shared ESS values](../../../ess/domains/cli.yaml).

The [compatibility inventory](compatibility.json) keeps existing `describe`,
`invoke` and `serve` routes explicit. They stay implemented by the current runtime.
They are not aliases of grouped commands, and their successful raw JSON output
does not acquire the new grouped-command envelope.

Generate the separate contract-fixture package with:

```sh
cargo run --locked -p connectors-build -- cli
cargo run --locked -p connectors-build -- cli --check
```

The generated package lives at `apps/connectors-cli-contract`, beside the runtime
package. It is excluded from the enclosing Cargo workspace and consumed only as a
conformance dependency. Nesting an independent workspace inside a member package
conflicts with Cargo's workspace membership rules.

The generated package uses recording handlers in conformance tests and an
unavailable handler in its standalone binary. It is not an installed replacement
for the current `connectors` executable. Local keyring storage, management,
supervision and provider effects remain required production implementation work.
The runtime must also enforce selector relationships, operation schema identity,
current descriptor revisions, configured limits and all cross-record predicates;
structural type validation does not establish those facts.
