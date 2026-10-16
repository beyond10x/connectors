# OpenRouter management through the catalog provider

The catalog provider reads an OpenRouter account's management key record, its
workspaces and its API keys from the pinned OpenRouter OpenAPI document, with the
management key held in the Connectors keyring. Nothing here is OpenRouter-specific
code: the pinned document is compiled into a bundle, a reviewed selection set
exposes three reads, and the engine described in
[the catalog provider guide](local-catalog-provider.md) binds and sends them.
Configuration, connection and invocation work as described there; this page covers
what differs for OpenRouter.

## Source and bundle

The pinned source is
[`adapters/openrouter/upstream/openrouter-openapi.json`](../adapters/openrouter/upstream/README.md),
SHA-256 `fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142`
(2,690,724 bytes): the document retrieved on 2026-10-16 from
<https://openrouter.ai/openapi.json> (upstream SHA-256
`8287890b719d375d64b5651ca4fa2bc34be3af84b24e8fceb8ab13ae9894fe12`, licence MIT),
with the publisher's example keys redacted by `connectors-build redact` (rule
`upstream-redaction/3`, described in the source README). It is OpenAPI `3.1.0`,
`info.version` `1.0.0`, and declares no `servers`, so the configuration names
`api_base` and every operation path carries `/api/v1`. The bundle is built with:

```sh
cargo run --locked -p connectors-build -- catalog \
  --provider openrouter \
  --source adapters/openrouter/upstream/openrouter-openapi.json \
  --directory adapters/catalog/generated/bundles \
  --auth-profile openrouter.api-key
```

It carries 150 operations, 2 reported unsupported: one write that declares its own
`servers`, and an array query parameter sent as one value as given; no shipped read
is among them. `adapters/catalog/tests/bundle_drift.rs` refuses a committed bundle
or index a fresh run would not reproduce byte for byte, and the repository gate
re-checks the redacted source against its record (`connectors-build source-hashes`).

## The shipped selection set

[`adapters/catalog/providers/openrouter/operations.json`](../adapters/catalog/providers/openrouter/operations.json)
exposes three reads and nothing else. `adapters/catalog/tests/openrouter.rs` pins
this exact id list with each id's `operationId`, method and path in the pinned
document.

| id | `operationId` | request | effect |
|---|---|---|---|
| `key.get` | `getCurrentKey` | `GET /api/v1/key` | read |
| `workspaces.list` | `listWorkspaces` | `GET /api/v1/workspaces`, paged by `offset` and `limit` | read |
| `keys.list` | `list` | `GET /api/v1/keys`, filtered by `workspace_id`, `include_disabled`, `include_expired`, paged by `offset` | read |

`key.get` is the management key's own record and also the connection's identity
read. `keys.list` lists keys by hash, label, limits and usage, never their secrets.
Every list returns one page per call; the provider does not walk pages.

## Authentication and configuration

An OpenRouter management key, sent as `Authorization: Bearer <key>`, under the
profile `openrouter.api-key`. The identity is the key record the connect read
returns: kind `openrouter.key`, subject `/data/label`. The key never appears in the
configuration.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "openrouter",
  "provider": "openrouter",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://openrouter.ai/api/v1",
  "auth": {
    "profile": "openrouter.api-key",
    "header": "Authorization",
    "bearer": true,
    "label": "OpenRouter management key",
    "identity": {"path": "key", "kind": "openrouter.key", "subject_pointer": "/data/label"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/openrouter/operations.json"
}
```

## Bind, connect and invoke

Bind the provider to the local CLI as [the catalog provider guide](local-catalog-provider.md#bind-the-provider-to-the-local-cli)
describes, with `private_protocol = "connectors-private/2"`,
`profiles = ["openrouter.api-key"]` and
`operations = ["key.get", "workspaces.list", "keys.list"]`, here under the adapter
alias `router`. Enter the management key through the protected credential prompt
on your controlling terminal; it never goes in chat, an argument or the
environment:

```sh
connectors --output json connections connect --adapter router --profile openrouter.api-key --credential-prompt
```

The prompt reads the key from the terminal. The
commands below were run in this tree against a local HTTPS fixture that plays
OpenRouter with a fictional key (the ignored test `serve_the_fixture` in
`adapters/catalog/tests/openrouter.rs`), so the key was supplied from an owner-only
file with `--credential-file` instead of the prompt, and every body is the
fixture's. Connect read the fixture's `GET /api/v1/key` and named its `label` as
the subject:

```sh
connectors --output json connections connect --adapter router --profile openrouter.api-key --credential-file /owner-only/openrouter.json
```

```json
{"ok":true,"result":{"connection":{"external_identity":{"kind":"openrouter.key","subject":"fixture-management-key"},"observed_at_ms":1792138503681,"source":"authority","stale":false,"summary":{"adapter":"router","connection":"conn_33da5ec3f2a4497b96d2b94fe09c4067_5be7ecc5b52c4e619b94774b2938760b","instance_id":"fixture-openrouter","profile":"openrouter.api-key","revision":"00520724-5059-4573-8045-82f550bf2945","state":"ready"},"valid_until_ms":1792138562854}}}
```

Each read takes `--schema` and `--revision` from `operations describe`
(`connectors --output json operations describe --adapter router --operation key.get`).
Connect evidence lasts 60 seconds; revalidate before reading after that.

The management key's own record:

```sh
connectors --output json operations invoke --adapter router --connection "$connection" \
  --operation key.get --schema "$schema" --revision "$revision" --input-json '{}'
```

```json
{"ok":true,"result":{"adapter":"router","operation":"key.get","result":{"body":{"data":{"is_free_tier":false,"is_management_key":true,"label":"fixture-management-key","limit":100,"limit_remaining":74.5,"usage":25.5}},"provenance":{"instance":"fixture-openrouter","observed_at_unix_ms":1792138519516,"resource":"/api/v1/key","source_revision":"fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142"},"status":200},"revision":"d1d435a539acd78d46e4d4dd6adc33a50f34f9a6ca3d0c4cbf03a89d384b4caf"}}
```

The second workspace, one page at offset 1:

```sh
connectors --output json operations invoke --adapter router --connection "$connection" \
  --operation workspaces.list --schema "$schema" --revision "$revision" \
  --input-json '{"offset":1,"limit":1}'
```

```json
{"ok":true,"result":{"adapter":"router","operation":"workspaces.list","result":{"body":{"data":[{"created_at":"2026-09-15T15:45:00Z","id":"ws-two","name":"beta","slug":"beta"}],"total_count":2},"provenance":{"instance":"fixture-openrouter","observed_at_unix_ms":1792138520578,"resource":"/api/v1/workspaces","source_revision":"fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142"},"status":200},"revision":"d1d435a539acd78d46e4d4dd6adc33a50f34f9a6ca3d0c4cbf03a89d384b4caf"}}
```

The keys of one workspace, from offset 1:

```sh
connectors --output json operations invoke --adapter router --connection "$connection" \
  --operation keys.list --schema "$schema" --revision "$revision" \
  --input-json '{"workspace_id":"ws-two","offset":1}'
```

```json
{"ok":true,"result":{"adapter":"router","operation":"keys.list","result":{"body":{"data":[{"created_at":"2026-09-15T15:45:00Z","disabled":false,"hash":"hash-two","label":"second","name":"second","workspace_id":"ws-two"}]},"provenance":{"instance":"fixture-openrouter","observed_at_unix_ms":1792138521604,"resource":"/api/v1/keys","source_revision":"fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142"},"status":200},"revision":"d1d435a539acd78d46e4d4dd6adc33a50f34f9a6ca3d0c4cbf03a89d384b4caf"}}
```

## Limits

- Verified against the local HTTPS fixture only, with synthetic records. Nothing
  has run against a live OpenRouter account, so what a management key may read,
  and OpenRouter's own error answers, are as the pinned document states them.
- The provider does not walk pages and does not bound `limit`.

## Creating a key

`key.create` (`createKeys`, `POST /api/v1/keys`) creates an OpenRouter API key with
the management key. It is a required-approval write. OpenRouter answers the new key
in `key`, once ("The plaintext `key` is returned only in this response", the pinned
document); Connectors never returns it. The answer's key goes straight into custody
as a new connection of a second configured alias, and the caller receives only that
connection's reference and the key's `hash` (design
[§8a](../adapters/catalog/design.md#8a-writes-that-create-a-credential),
[created credentials](local-mutation-ledger.md#created-credentials)).

The body is closed to these keys; any other, such as the partner `external` object
(which carries a partner-supplied key) or `creator_user_id`, is refused as
`invalid_input` before any request:

| key | meaning |
|---|---|
| `name` | required, a non-empty string |
| `limit` | the key's spending limit in USD, or `null` |
| `limit_reset` | `daily`, `weekly`, `monthly` or `null` |
| `include_byok_in_limit` | a boolean |
| `expires_at` | an ISO 8601 UTC timestamp with seconds, or `null` |
| `workspace_id` | the workspace the key belongs to |

### The inference alias

The created key is an ordinary OpenRouter API key. The pinned document declares one
security scheme for every operation, `apiKey` (`http` `bearer`), and its current-key
read, `GET /api/v1/key`, answers "the API key associated with the current
authentication session", whichever key that is. So the alias that holds created
keys runs the same provider under the same profile, `openrouter.api-key`, with its
own `instance`; connecting a created key reads its own record and names its own
`label` as the subject. Grant it only `key.get`: the management reads answer a
created key `403`.

```json
{
  "format": "connectors-catalog-local/2",
  "instance": "openrouter-inference",
  "provider": "openrouter",
  "bundle_directory": "/absolute/path/adapters/catalog/generated/bundles",
  "api_base": "https://openrouter.ai/api/v1",
  "auth": {
    "profile": "openrouter.api-key",
    "header": "Authorization",
    "bearer": true,
    "label": "OpenRouter inference key",
    "identity": {"path": "key", "kind": "openrouter.key", "subject_pointer": "/data/label"}
  },
  "operations_file": "/absolute/path/adapters/catalog/providers/openrouter/operations.json"
}
```

Both aliases load the same shipped selection file, `operations.json`, and that is
deliberate: the alias's `permissions.operations` is what narrows it. The inference
alias grants `key.get` alone, so its `operations list` shows that one operation and
`operations describe --operation key.create` on it is refused `forbidden` at
`admission`. It declares no custody target and never needs one, because the ledger
refuses a credential-creating operation only on an alias that is granted it. A
separate reads-only selection file would add nothing but a second file to keep in
step; the run below connects the inference alias and reads through it with the
shared file. Do not grant `key.create` to the inference alias: a target holds no
custody of its own, and an operation that creates a credential with no custody target is refused `invalid_configuration`.

### Configure both aliases

The commands below were run in this tree with the debug `target/debug/connectors`
against the same local HTTPS fixture as above (the ignored test `serve_the_fixture`
in `adapters/catalog/tests/openrouter.rs`, which now also serves a disposable Secret
Service and two synthetic Roughtime sources). The fixture's instance ids are
`fixture-openrouter` and `fixture-openrouter-inference`, and its disposable tree is
named with `--config` and `--state-dir`, which the commands omit; a path under the
operator's home is shown as `~`. `secret_service_socket` and the clock sources are the
fixture's: a real install omits the socket and names real sources
([local clock](local-clock.md)).

Print each native configuration's revision, then write both adapter entries into the
configuration `setup init` created, with the executable's SHA-256 and absolute paths:

```sh
connectors-catalog-provider --local-config /absolute/path/catalog.json --print-local-bootstrap
connectors-catalog-provider --local-config /absolute/path/inference.json --print-local-bootstrap
sha256sum /absolute/path/connectors-catalog-provider
```

```toml
[adapters.router]
instance_id = "openrouter"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[adapters.router.permissions]
profiles = ["openrouter.api-key"]
operations = ["key.get", "workspaces.list", "keys.list", "key.create"]

[adapters.router.permissions.custody]
'key.create' = { adapter = "inference", profile = "openrouter.api-key" }

[adapters.router.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/catalog.json"]

[adapters.inference]
instance_id = "openrouter-inference"
adapter_id = "catalog"
configuration_revision = "REPLACE_FROM_BOOTSTRAP"
protocol = "v1alpha1"
private_protocol = "connectors-private/2"
startup = "on-demand"
restart = "never"

[adapters.inference.permissions]
profiles = ["openrouter.api-key"]
operations = ["key.get"]

[adapters.inference.executable]
path = "/absolute/path/connectors-catalog-provider"
sha256 = "REPLACE_WITH_EXECUTABLE_SHA256"
args = ["--local-config", "/absolute/path/inference.json"]
```

`[adapters.router.permissions.custody]` names, per operation, the alias and profile
that hold each created key (see
[created credentials](local-mutation-ledger.md#created-credentials)). The writes need
`private_protocol = "connectors-private/2"`, an `[approval_clock]` table and an
issuer key, which the steps below create.

### Enter the management key

The protected prompt reads the key from the controlling terminal (`/dev/tty`), so a
pipe is not enough. The run gave it a pseudo-terminal with `script` and typed the
fictional fixture key, which the terminal does not echo:

```sh
connectors --output json connections connect --adapter router --profile openrouter.api-key --credential-prompt
```

```text
OpenRouter management key: 
{"ok":true,"result":{"connection":{"external_identity":{"kind":"openrouter.key","subject":"fixture-management-key"},"observed_at_ms":1792184897877,"source":"authority","stale":false,"summary":{"adapter":"router","connection":"conn_790fd41e80cf486eaa6919ea9daa0105_549d79b5f900499d82c414defb741eb0","instance_id":"fixture-openrouter","profile":"openrouter.api-key","revision":"478277f7-d88e-4477-9852-934d7f0dfbdf","state":"ready"},"valid_until_ms":1792184957167}}}
```

The inference alias needs no `connect` of its own: custody connects each created key.

### Approve and create

A required-approval write needs the signing issuer, a published policy naming the
operation, and one proof for exactly the input sent. Initialize the issuer and publish
the policy once:

```sh
connectors --output json approvals key-init --adapter router
```

```json
{"ok":true,"result":{"adapter":"router","issuer":{"audience":"connectors.approval/29407db5-9390-4d2f-8617-295ff6d1abaa","issuer":"connectors.local-issuer/29407db5-9390-4d2f-8617-295ff6d1abaa","keys":[{"key_id":"b891675e-23e3-4cb3-82b1-4b28e8aad677","public_key":"V0Bho7Kcc0nfUOoYn8EISlxL_7jgARs_X18DcZHxhMw","state":"active"}],"revision":"463c0f9f-7ae4-4ec3-ae1f-1d930522b7a5"}}}
```

```sh
echo '{"operations":["key.create"]}' > policy.json
connectors --output json approvals policy-set --adapter router --input-file policy.json
```

```json
{"ok":true,"result":{"adapter":"router","policy":{"instance_id":"fixture-openrouter","issuer_id":"29407db5-9390-4d2f-8617-295ff6d1abaa","operations":["key.create"],"owner_uid":1000,"policy_id":"9ff23174-883e-4db5-aadf-7cd72f5cbafe","revision":1,"selection":{"clock_configuration_sha256":"b2f9bee758ffe79f9dc66183e97e8fa1b589eb48105ab7206e57dbdf4afaad72","configuration_revision":"59e3148d63ef3c1542d006cd8d13f4cda2bde8c772cedf8f9fd64dcec5aa2644","descriptor_revision":"f6c0eff9ac3a2d424e8962bdbf1c9a7ee6e1c2ee038a7c7b331c5e4bfd9d527d","executable_selection":"8975aba1ae5267274c37d585897e74a059423873ed70b2d682b78f6f9b64e6b1"}}}}
```

Describe the operation for its `schema` and `revision`:

```sh
connectors --output json operations describe --adapter router --operation key.create
```

```json
{"ok":true,"result":{"adapter":"router","operation":{"contract":"operations/v1alpha1","description":"Create one API key from body; OpenRouter requires body.name. The new key, shown once, is held in custody as a new connection of the configured target alias and never returned: the result is that connection and the key's hash. Unguarded: nothing exists to compare before a create, so the same input approved and sent again creates a second key. body.limit sets its spending limit in USD, body.limit_reset its reset period, body.expires_at its expiry and body.workspace_id its workspace","id":"key.create","input_schema":{"additionalProperties":false,"properties":{"body":{"additionalProperties":false,"properties":{"expires_at":{},"include_byok_in_limit":{"type":"boolean"},"limit":{},"limit_reset":{},"name":{"minLength":1,"type":"string"},"workspace_id":{"type":"string"}},"required":["name"],"type":"object"}},"required":["body"],"type":"object"},"output_schema":{"additionalProperties":false,"properties":{"connection":{"minLength":1,"type":"string"},"custody":{"enum":["stored"],"type":"string"},"hash":{"type":"string"}},"required":["hash"],"type":"object"},"profile":"mutation"},"revision":"f6c0eff9ac3a2d424e8962bdbf1c9a7ee6e1c2ee038a7c7b331c5e4bfd9d527d","schema":"9b875b53edfa72596596818a9229bf6508b738b8b9c4c0268c48d5f3c93cf85f","source":"cached","stale":true}}
```

Prepare the exact input; the answer's `subject_sha256` is what the operator approves.
Every `approvals` and `operations invoke` command of the create takes the same target
arguments:

```sh
connectors --output json approvals prepare --adapter router --connection "$connection" \
  --operation key.create --schema "$schema" --revision "$revision" \
  --input-json '{"body":{"name":"agent-inference","limit":5,"limit_reset":"monthly"}}'
```

```json
{"ok":true,"result":{"adapter":"router","preparation":{"approval_policy":{"audience":"connectors.approval/29407db5-9390-4d2f-8617-295ff6d1abaa","issuer":"connectors.local-issuer/29407db5-9390-4d2f-8617-295ff6d1abaa","max_lifetime_seconds":300,"mode":"required"},"presentation":{"connection":"conn_790fd41e80cf486eaa6919ea9daa0105_549d79b5f900499d82c414defb741eb0","descriptor_revision":"f6c0eff9ac3a2d424e8962bdbf1c9a7ee6e1c2ee038a7c7b331c5e4bfd9d527d","instance":"fixture-openrouter","operation":"key.create"},"subject":{"approval_mode":"required","authority":{"current_authority":{"id":"connectors.local-policy/9ff23174-883e-4db5-aadf-7cd72f5cbafe/1","sha256":"32cf98a7ac740d291255693017f724612618df494cc49f46563eafbda0a9d56e"},"executor":null,"scope":{"caller":"connectors.local-owner/29407db5-9390-4d2f-8617-295ff6d1abaa/1000","executor":null,"realm":null,"tenant":null}},"canonicalization":"adapter-v1-canonical-json","format":"connectors.approval-subject/v1","input_sha256":"22e969eadebe3f2ae03c307cfbe16fb81b71a2a318f78dc079a2145e91594600","origin":{"authority_ref":"fixture-openrouter","kind":"direct"},"route":null,"target":{"configuration_revision":"59e3148d63ef3c1542d006cd8d13f4cda2bde8c772cedf8f9fd64dcec5aa2644","connection":"conn_790fd41e80cf486eaa6919ea9daa0105_549d79b5f900499d82c414defb741eb0","connection_revision":"478277f7-d88e-4477-9852-934d7f0dfbdf","contract":"operations/v1alpha1","descriptor_revision":"f6c0eff9ac3a2d424e8962bdbf1c9a7ee6e1c2ee038a7c7b331c5e4bfd9d527d","instance":"fixture-openrouter","operation":"key.create","profile":"mutation"}},"subject_sha256":"2471a53c18995f3444509c3edbb2d5372b4e8df240bb98aec3ac39a10b3d575b"}}}
```

Issue the proof for that digest into a new file inside a private directory:

```sh
connectors --output json approvals issue --adapter router --connection "$connection" \
  --operation key.create --schema "$schema" --revision "$revision" \
  --input-json '{"body":{"name":"agent-inference","limit":5,"limit_reset":"monthly"}}' \
  --approve-subject 2471a53c18995f3444509c3edbb2d5372b4e8df240bb98aec3ac39a10b3d575b \
  --proof-output proof.json
```

```json
{"ok":true,"result":{"adapter":"router","disposition":"published","reference":"95ce827fcb802a874d960798f5cf3fcbcc93bafb9b26123400128ba7c274749d","subject_sha256":"2471a53c18995f3444509c3edbb2d5372b4e8df240bb98aec3ac39a10b3d575b"}}
```

Create one inference key, with an idempotency key and the proof:

```sh
connectors --output json operations invoke --adapter router --connection "$connection" \
  --operation key.create --schema "$schema" --revision "$revision" \
  --input-json '{"body":{"name":"agent-inference","limit":5,"limit_reset":"monthly"}}' \
  --idempotency-key agent-inference-1 --approval-file proof.json
```

```json
{"ok":true,"result":{"adapter":"router","mutation":{"attempt":{"id":"dd6a29f9-c185-4e68-9853-b85ede63782e","instance":"fixture-openrouter"},"cause":null,"classification":"applied","original_request_id":"4dcde91f-6f3a-410b-b514-34703492833f","replayed":false},"operation":"key.create","request_id":"4dcde91f-6f3a-410b-b514-34703492833f","result":{"connection":"conn_790fd41e80cf486eaa6919ea9daa0105_998895dc177a43d18290423a9f9cca78","custody":"stored","hash":"fixture-created-key-hash"},"revision":"f6c0eff9ac3a2d424e8962bdbf1c9a7ee6e1c2ee038a7c7b331c5e4bfd9d527d","source_audit":{"audit_ref":"4146bcf4-2e31-499b-9a0f-4638cdb86561","audit_status":"complete","instance":"fixture-openrouter"}}}
```

The result is the new connection of the `inference` alias and the key's `hash`; the key
itself is in no output. The inference alias lists that connection and reads through it
with the created key:

```sh
connectors --output json connections list --adapter inference
```

```json
{"ok":true,"result":{"connections":[{"adapter":"inference","connection":"conn_790fd41e80cf486eaa6919ea9daa0105_998895dc177a43d18290423a9f9cca78","instance_id":"fixture-openrouter-inference","profile":"openrouter.api-key","revision":"2782e465-2658-46ca-993c-eead1bff2b2f","state":"ready"}],"observed_at_ms":1792184927086,"source":"authority","stale":false,"valid_until_ms":1792184979729}}
```

```sh
connectors --output json operations invoke --adapter inference --connection "$inference_connection" \
  --operation key.get --schema "$schema" --revision "$revision" --input-json '{}'
```

```json
{"ok":true,"result":{"adapter":"inference","operation":"key.get","result":{"body":{"data":{"is_free_tier":false,"is_management_key":false,"label":"fixture-inference-key","limit":5,"limit_remaining":5,"usage":0}},"provenance":{"instance":"fixture-openrouter-inference","observed_at_unix_ms":1792184933383,"resource":"/api/v1/key","source_revision":"fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142"},"status":200},"revision":"245b9a6afdcf9639a7e0ce7e44d4fed76a22f44f575acd356c6ce9b67acf677d"}}
```

An approval proof is for one input; the same input sent again with the same
idempotency key replays the stored answer, and a second approved create makes a second
key (the description above says so).

### Moving an existing install

An install that already has the `router` alias with the three reads adds, in this
order:

1. Update to a build whose `operations.json` carries `key.create`, then print the
   bootstrap again: the selection changed, so the `configuration_revision` did, and
   the executable's SHA-256 changed with the build.
2. Write the inference configuration shown above and reprint its bootstrap.
3. Edit the configuration: the new revision and SHA-256, `key.create` in
   `operations`, the custody table and the `inference` alias. The run applied exactly
   this change to a configuration that held the three reads and one connected
   management key (`before.toml`, then the edited file):

```diff
@@ -8,7 +8,7 @@
 [adapters.router]
 instance_id = "fixture-openrouter"
 adapter_id = "catalog"
-configuration_revision = "a15b43ba4aee78e9c949184007408c1430fc853795ca22f130bd7d609e4ebafa"
+configuration_revision = "59e3148d63ef3c1542d006cd8d13f4cda2bde8c772cedf8f9fd64dcec5aa2644"
 protocol = "v1alpha1"
 private_protocol = "connectors-private/2"
 startup = "on-demand"
@@ -16,12 +16,33 @@
 
 [adapters.router.permissions]
 profiles = ["openrouter.api-key"]
-operations = ["key.get", "workspaces.list", "keys.list"]
+operations = ["key.get", "workspaces.list", "keys.list", "key.create"]
+
+[adapters.router.permissions.custody]
+'key.create' = { adapter = "inference", profile = "openrouter.api-key" }
 
 [adapters.router.executable]
 path = "/absolute/path/connectors-catalog-provider"
 sha256 = "b45e4d41bf3c526449ac8849c40a51dc3a7f22545a0fe2a1617437a6c167badb"
-args = ["--local-config", "/absolute/path/old/catalog.json"]
+args = ["--local-config", "/absolute/path/catalog.json"]
+
+[adapters.inference]
+instance_id = "fixture-openrouter-inference"
+adapter_id = "catalog"
+configuration_revision = "1986ba353d311b41b78ca5c550f213c82b007900a6883424990bfd9739b35ba4"
+protocol = "v1alpha1"
+private_protocol = "connectors-private/2"
+startup = "on-demand"
+restart = "never"
+
+[adapters.inference.permissions]
+profiles = ["openrouter.api-key"]
+operations = ["key.get"]
+
+[adapters.inference.executable]
+path = "/absolute/path/connectors-catalog-provider"
+sha256 = "b45e4d41bf3c526449ac8849c40a51dc3a7f22545a0fe2a1617437a6c167badb"
+args = ["--local-config", "/absolute/path/inference.json"]
```

   (paths shortened to `/absolute/path`). The saved management key is kept, but the
   changed configuration revision leaves its connection `pending`:

```sh
connectors --output json connections list --adapter router
```

```json
{"ok":true,"result":{"connections":[{"adapter":"router","connection":"conn_b802d3c6c9ee41de8d2eda605f618259_614ae248d90e4b3d81a4033e83843ff9","instance_id":"fixture-openrouter","profile":"openrouter.api-key","revision":"785a0197-e051-4c63-b7ff-8b694f60af2d","state":"pending"}],"observed_at_ms":1792184964335,"source":"authority","stale":true,"valid_until_ms":1792184964335}}
```

4. Renew it without re-entering the key:

```sh
connectors --output json connections revalidate --adapter router \
  --connection "$connection" --expected-revision "$revision"
```

```json
{"ok":true,"result":{"connection":{"external_identity":{"kind":"openrouter.key","subject":"fixture-management-key"},"observed_at_ms":1792184965705,"source":"authority","stale":false,"summary":{"adapter":"router","connection":"conn_b802d3c6c9ee41de8d2eda605f618259_614ae248d90e4b3d81a4033e83843ff9","instance_id":"fixture-openrouter","profile":"openrouter.api-key","revision":"08a5e6dc-c9e6-44be-8108-ad7a75b51626","state":"ready"},"valid_until_ms":1792185025615}}}
```

   `operations list --adapter router` now names `key.get`, `workspaces.list`,
   `keys.list` and `key.create`.
5. Run `approvals key-init` and `approvals policy-set` as above when the alias has no
   issuer or policy yet; an existing policy is republished with `--expected-revision`,
   because a changed configuration, descriptor or executable invalidates it. Then
   create as above.
