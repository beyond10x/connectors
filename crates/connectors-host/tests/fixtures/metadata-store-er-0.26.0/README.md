# Metadata store written by Entity Runtime 0.26.0

`metadata.sqlite3` is a local metadata store written on 2026-10-07 by `seed` in
`tests/metadata_store_previous_pin.rs`, built at connectors commit `49f076b1d`
against Entity Runtime 0.26.0 and Eventlog revision `6983cc25`, the runtime pins
connectors 0.31.0 released with. The store was then copied with SQLite
`VACUUM INTO`. `registry-clock.floor` is the clock floor that run left in the
state directory, and `references` lists the acquisition reference `seed`
returned for each instance.

Its `local_authority.owner_uid` is 1000, the uid that wrote it. Connectors
refuses a store another uid owns, so the test's restore rewrites that row in
its copy to the uid running the test; the fixture itself is never modified.

It holds 15 recorded events, 21 tables, 20 indexes and no triggers. Its only
binding is the synthetic `fixture-adapter` with the provider authority
`https://fixture.invalid/fixed-authority`; it carries no credentials, no
custody material and no host path.

| File | SHA-256 |
|---|---|
| `metadata.sqlite3` | `0b84ebbd19b8954eb3dba3b6385d9a8fdb5d1c067680c74fc09bff37fdf4f8de` |
| `references` | `30a539b7304369673fa78da3ceb900266f934e50cbe5b46879bacc8aae632e3b` |
| `registry-clock.floor` | `40abb8a2aa4d7602be912895b00c1450f25fa89179d5f3279160cac682e60044` |

Never regenerate it with a newer build: its value is that an older runtime wrote
it. A later pin move adds a sibling fixture written by the pin being replaced.
