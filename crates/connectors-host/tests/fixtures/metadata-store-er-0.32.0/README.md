# Metadata store written by Entity Runtime 0.32.0, with one read invoke

The three files `save` writes in
`crates/connectors-host/src/local/registry/store_cost_tests.rs`, made on 2026-11-23 by
`read_invoke_cost_by_store_size` in a checkout of tag `v0.74.2` (commit `50ba58849b0`,
unmodified), which builds on Entity Runtime 0.32.0 and Eventlog 0.9.4, the runtime pins
connectors 0.74.2 and 0.75.0 released with:

```sh
CONNECTORS_STORE_COST_STORES=<dir> CONNECTORS_STORE_COST_EVENTS=18 \
  cargo test --locked -p connectors-host --lib read_invoke_cost_by_store_size -- --ignored --nocapture
```

The files are that run's `<dir>/read-invokes-18/`. No network and no credential were used.

N = 18 is the smallest `CONNECTORS_STORE_COST_EVENTS` that leaves a read invoke in the store.
The store the test grows holds 17 recorded events once its connection is published (the same
command with `CONNECTORS_STORE_COST_EVENTS=1` saves that store and prints `events=17`), and the
test then adds read invokes until the store holds at least N. One read invoke appends 3, so this
store holds 20 recorded events: 1 `er.binding` and 19 `er.recorded_entry`, the last three of
them the `connectors.credential_evidence.ReadUse` of the one read invoke. It has durable open
checkpoints, as `setup init` creates them, and 67 blobs of 963,357 bytes.

`metadata.sqlite3` is a `VACUUM INTO` copy set to WAL mode, `reference` is the connection
reference the cost test published, and `registry-clock.floor` is the clock floor that run left
in the state directory.

Its `local_authority.owner_uid` is 1000, the uid that wrote it. Connectors refuses a store
another uid owns, so `a_store_the_previous_pin_wrote_a_read_into_takes_a_read_invoke` (in
`store_cost_tests.rs`) always rewrites that row in its copy to the uid running the test; the
fixture itself is never modified.

Its only binding is the synthetic `fixture-adapter` of `fixture-instance` with the provider
authority `https://fixture.invalid/fixed-authority`; it carries no credentials, no custody
material and no host path.

| File | Bytes | SHA-256 |
|---|---|---|
| `metadata.sqlite3` | 1,392,640 | `3489e90522e7978f76ee243bb8f2932b241103ea89c559fc3fc37d0c6063c77c` |
| `reference` | 70 | `095389d739fa80b005fd22de3cc140c322fe72b660233cddc78232fb76dd81ee` |
| `registry-clock.floor` | 30 | `699dfb0c88308623f653fd583becee96efcabb40be3451425940144b3510b807` |

Never regenerate it with a newer build: its value is that Entity Runtime 0.32.0 wrote it, with
every record carrying its entity definition (`er.record/2`, `/3` and `/4`), before 0.33.0
began to name the definition by digest.
