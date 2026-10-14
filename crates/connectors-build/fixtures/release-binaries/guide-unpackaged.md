# Install the adapters

Prose naming `connectors-loki` is not a command.

```sh
install -m 0755 "connectors-$version-$target/connectors-sql" ~/.local/bin/
connectors-kubernetes --config kubernetes.yaml
cargo build --release --locked -p connectors -p connectors-tavily
target/release/connectors-tavily --local-config tavily.json
RUST_LOG=debug connectors --version && connectors-grafana --help
```

```json
{"format": "connectors-sql-local/1"}
```

```console
$ connectors-prometheus --version
```
