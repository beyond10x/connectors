# Tavily websearch through the local CLI

The `tavily` connector answers the [websearch family](../contracts/datasources/websearch/v1alpha1/semantics.md):
`websearch.search`, `websearch.fetch` and `websearch.crawl`, each a read Tavily serves as a POST.
The mapping, bounds and error table are in
[the Tavily profile](../adapters/tavily/contracts/websearch/v1alpha1/semantics.md). It requires Linux
x86_64 and the [qualified Secret Service binding](local-secret-service.md), like every saved
credential.

Connector, instance and connection are the terms of the [vocabulary](../README.md#vocabulary).

```sh
CARGO_BUILD_JOBS=2 cargo build --release --locked -p connectors -p connectors-tavily
target/release/connectors --output json setup init
target/release/connectors --output json setup check
```

## Configure the native target

Create an owner-only native JSON file, in an admitted directory without symlinks:

```json
{
  "format": "connectors-tavily-local/1",
  "instance": "tavily-local",
  "api_base": "https://api.tavily.com/"
}
```

`api_base` may be left out; it defaults to `https://api.tavily.com/` and must be HTTPS. The API key is
never part of this file, of the TOML below, of executable arguments or of the environment.

Inspect the native bootstrap and hash the built executable:

```sh
target/release/connectors-tavily --local-config /absolute/path/tavily.json --print-local-bootstrap
sha256sum target/release/connectors-tavily
```

Copy `configuration_revision` from that output verbatim. Tavily is not a catalog connector, so
the configuration pins its instance. Add the instance to the configuration created by setup:
change its `format` line to `format = "connectors-local/5"`, which instances need, and add the
instance `tavily` of the connector `tavily`:

```toml
[connectors.tavily]
enabled = true

[connectors.tavily.instances.tavily]
alias = "tavily"
instance_id = "tavily-local"
adapter_id = "tavily"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/1"
startup = "on-demand"
restart = "never"

[connectors.tavily.instances.tavily.permissions]
profiles = ["tavily.api-key"]
operations = ["websearch.search", "websearch.fetch", "websearch.crawl"]

[connectors.tavily.instances.tavily.executable]
path = "/absolute/path/target/release/connectors-tavily"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/tavily.json"]
```

## Connect and read

```sh
target/release/connectors --output json connections connect --connector tavily --instance tavily --profile tavily.api-key --credential-prompt
target/release/connectors --output json operations describe --connector tavily --instance tavily --operation websearch.search
target/release/connectors --output json operations invoke --connector tavily --instance tavily --connection CONNECTION --operation websearch.search --schema SCHEMA --revision REVISION --input-json '{"query":"vector database release","max_results":5,"time_range":"week","content":"full"}'
```

For automation, `--credential-stdin` or `--credential-file /absolute/private/file.json` carries the
complete native `{"api_key":"..."}` document. Connect and reconnect check the key with `GET /usage`,
which spends no search credit.

## What the connection is bound to

**Tavily exposes no account identifier.** The connection's identity is the kind `tavily.api-key` and,
where Tavily reports one, the account's plan name. Two keys are not told apart, and a reconnect cannot
detect that a key belongs to another account.

**A key grants no scopes and exposes no expiry.** The profile declares no required scopes;
`credential_expires_at_ms` is absent, meaning not observed. A revoked key is reported as
`unauthorized` when it is next used; `connections revalidate` then finds it invalid, and the
connection lists as `stale` until `connections reconnect` publishes a working key.

**A read spends credits.** Every invocation is a provider call; nothing is cached, and two
invocations are two charges.

## Replacing the executable

The instance pins the executable's SHA-256, and the host serves the bootstrap it cached at the last
admitted launch. After rebuilding `connectors-tavily`, update `sha256`, stop the running
incarnation exactly, then reconnect the connection, which performs the admitted launch under the same
connection id. Until then `operations describe` answers `description_unavailable`;
`connections revalidate` performs the admitted launch too:

```sh
target/release/connectors --output json instances status --connector tavily --instance tavily
target/release/connectors --output json instances stop --connector tavily --instance tavily --expected-revision CONFIGURATION_REVISION --host-incarnation HOST --child-incarnation CHILD
target/release/connectors --output json connections reconnect --connector tavily --instance tavily --connection CONNECTION --expected-revision REVISION --credential-stdin
```

## Evidence lifetime

Validation evidence lasts 60 seconds. After it the connection lists as `pending` and a read is
refused with `not_granted` at admission until `connections revalidate` renews it; renewing calls
`GET /usage` and spends no credit. A scheduled caller revalidates before reading, and once more
when a read is refused because the evidence lapsed between the two.
