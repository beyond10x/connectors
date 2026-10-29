# `connectors` CLI contract

Local integration access through explicitly configured connector instances. After an upgrade, `connectors setup owner-stop` stops the owner an older binary started

This generated package installs an unavailable handler. Application behavior enters through the `Handler` seam; schema-selected calls also require `DynamicValidator`. See `help.txt` for process options and `binding.json` for resolved types and targets.

## Commands

### `approvals clock-check`

Check the configured authenticated clock without starting services

Callable: `approval-clock-check`. Input: `connectors.cli.ApprovalClockCheckInput`. Result: `connectors.cli.ApprovalClockCheckResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals issue`

Approve the exact reconstructed subject and publish a new protected proof file

Callable: `approval-issue`. Input: `connectors.cli.ApprovalIssueInput`. Result: `connectors.cli.ApprovalIssueResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals key-init`

Create a new approval-signing key in protected local custody

Callable: `approval-keys-init`. Input: `connectors.cli.ApprovalKeyInitInput`. Result: `connectors.cli.ApprovalKeysResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals key-recover`

Confirm and publish an exact staged approval-signing key

Callable: `approval-keys-recover`. Input: `connectors.cli.ApprovalKeyRecoverInput`. Result: `connectors.cli.ApprovalKeysResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals key-retire`

Delete exact noncurrent signing material while retaining public history

Callable: `approval-keys-retire`. Input: `connectors.cli.ApprovalKeyRetireInput`. Result: `connectors.cli.ApprovalKeysResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals key-revoke`

Revoke the current key and cancel its pending replacement

Callable: `approval-keys-revoke`. Input: `connectors.cli.ApprovalKeyRevokeInput`. Result: `connectors.cli.ApprovalKeysResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals key-rotate`

Publish a replacement for the exact current approval-signing key

Callable: `approval-keys-rotate`. Input: `connectors.cli.ApprovalKeyRotateInput`. Result: `connectors.cli.ApprovalKeysResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals key-status`

Observe public key metadata without starting services or reading secrets

Callable: `approval-keys-status`. Input: `connectors.cli.ApprovalKeyStatusInput`. Result: `connectors.cli.ApprovalKeysResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals policy-set`

Publish the exact permitted write-operation set under revision control

Callable: `approval-policy-set`. Input: `connectors.cli.ApprovalPolicySetInput`. Result: `connectors.cli.ApprovalPolicyResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals policy-status`

Inspect retained local write policy without starting services

Callable: `approval-policy-status`. Input: `connectors.cli.ApprovalPolicyStatusInput`. Result: `connectors.cli.ApprovalPolicyResult`.

Error `failure`: `connectors.cli.Failure`.

### `approvals prepare`

Resolve the exact approval subject without provider or credential access

Callable: `approval-prepare`. Input: `connectors.cli.ApprovalPrepareInput`. Result: `connectors.cli.ApprovalPrepareResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections connect`

Begin admitted local protected credential acquisition on <connector>/<instance>; --endpoint first writes that instance's endpoint into the configuration

Callable: `connections-connect`. Input: `connectors.cli.ConnectionConnectInput`. Result: `connectors.cli.ConnectionConnectResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections describe`

Inspect safe metadata for one connection

Callable: `connections-describe`. Input: `connectors.cli.ConnectionDescribeInput`. Result: `connectors.cli.ConnectionDescribeResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections launch`

Run a pinned consumer with one connection's credential on descriptor 3

Callable: `connections-launch`. Input: `connectors.cli.ConnectionLaunchInput`. Result: `connectors.cli.ConnectionLaunchResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections list`

List safe connection metadata

Callable: `connections-list`. Input: `connectors.cli.ConnectionListInput`. Result: `connectors.cli.ConnectionListResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections reconnect`

Publish a new credential on the exact connection without changing its identity; `connections repair` is its deprecated name

Callable: `connections-reconnect`. Input: `connectors.cli.ConnectionReconnectInput`. Result: `connectors.cli.ConnectionReconnectResult`.

Alias: `connections repair`.

Error `failure`: `connectors.cli.Failure`.

### `connections revalidate`

Revalidate the saved credential without re-entry

Callable: `connections-revalidate`. Input: `connectors.cli.ConnectionRevalidateInput`. Result: `connectors.cli.ConnectionRevalidateResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections revoke`

Commit terminal local revocation under the current fence

Callable: `connections-revoke`. Input: `connectors.cli.ConnectionRevokeInput`. Result: `connectors.cli.ConnectionRevokeResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections status`

Observe exactly one connection or acquisition

Callable: `connections-status`. Input: `connectors.cli.ConnectionStatusInput`. Result: `connectors.cli.ConnectionStatusResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections token`

Issue a current access token of one OAuth2 connection, printed only with --reveal

Callable: `connections-token`. Input: `connectors.cli.ConnectionTokenInput`. Result: `connectors.cli.ConnectionTokenResult`.

Error `failure`: `connectors.cli.Failure`.

### `instances describe`

Inspect one configured instance and its cached descriptor; `adapters describe` is its deprecated name

Callable: `adapters-describe`. Input: `connectors.cli.AdapterDescribeInput`. Result: `connectors.cli.AdapterDescribeResult`.

Alias: `adapters describe`.

Error `failure`: `connectors.cli.Failure`.

### `instances list`

List configured instances; `adapters list` is its deprecated name

Callable: `adapters-list`. Input: `connectors.cli.AdapterListInput`. Result: `connectors.cli.AdapterListResult`.

Alias: `adapters list`.

Error `failure`: `connectors.cli.Failure`.

### `instances status`

Observe the lifecycle owner of the process that serves one instance; `adapters status` is its deprecated name

Callable: `adapters-status`. Input: `connectors.cli.AdapterStatusInput`. Result: `connectors.cli.AdapterStatusResult`.

Alias: `adapters status`.

Error `failure`: `connectors.cli.Failure`.

### `instances stop`

Stop only the exact owned incarnation of the process that serves one instance; `adapters stop` is its deprecated name

Callable: `adapters-stop`. Input: `connectors.cli.AdapterStopInput`. Result: `connectors.cli.AdapterStopResult`.

Alias: `adapters stop`.

Error `failure`: `connectors.cli.Failure`.

### `invoke`

Invoke one operation of a connector's instance; the instance, connection, schema and revision default to the only ones that fit

Callable: `invoke`. Input: `connectors.cli.InvokeInput`. Result: `connectors.cli.InvokeResult`.

Error `failure`: `connectors.cli.Failure`.

### `list`

List every connector this build ships, with its auth profiles and the instances the configuration names; needs no configuration and starts no process

Callable: `connectors-list`. Input: `null (inputless)`. Result: `connectors.cli.ConnectorListResult`.

Error `failure`: `connectors.cli.Failure`.

### `operations describe`

Inspect an operation and its exact schema identity

Callable: `operations-describe`. Input: `connectors.cli.OperationDescribeInput`. Result: `connectors.cli.OperationDescribeResult`.

Error `failure`: `connectors.cli.Failure`.

### `operations list`

List operations from selected cached metadata

Callable: `operations-list`. Input: `connectors.cli.OperationListInput`. Result: `connectors.cli.OperationListResult`.

Error `failure`: `connectors.cli.Failure`.

### `setup check`

Check local configuration and prerequisites without authentication

Callable: `setup-check`. Input: `null (inputless)`. Result: `connectors.cli.SetupCheckResult`.

Error `failure`: `connectors.cli.Failure`.

### `setup checkpoints-enable`

Enable durable open checkpoints on the existing store; one-way, connectors 0.32.0 and earlier can no longer open it

Callable: `setup-checkpoints-enable`. Input: `connectors.cli.SetupCheckpointsEnableInput`. Result: `connectors.cli.SetupCheckpointsEnableResult`.

Error `failure`: `connectors.cli.Failure`.

### `setup init`

Create private local configuration without starting services

Callable: `setup-init`. Input: `null (inputless)`. Result: `connectors.cli.SetupInitResult`.

Error `failure`: `connectors.cli.Failure`.

### `setup migrate`

Rewrite each entry of the legacy configuration format that maps to a shipped connector as that connector's instance, keeping its instance id, revision and selection; --mode dry-run (or --dry-run) prints the new file, --mode apply (or --apply) writes it and keeps the old one as config.toml.before-migrate-TIMESTAMP (UTC)

Callable: `setup-migrate`. Input: `connectors.cli.SetupMigrateInput`. Result: `connectors.cli.SetupMigrateResult`.

Error `failure`: `connectors.cli.Failure`.

### `setup owner-stop`

Stop this state directory's running owner, of any build, within 30 seconds; run it after an upgrade when a command answers next_action stop_owner

Callable: `setup-owner-stop`. Input: `null (inputless)`. Result: `connectors.cli.SetupOwnerStopResult`.

Error `failure`: `connectors.cli.Failure`.

## Runtime obligations

- handler:adapters-describe: implement the owner-qualified callable and its declared result/error contract
- handler:adapters-list: implement the owner-qualified callable and its declared result/error contract
- handler:adapters-status: implement the owner-qualified callable and its declared result/error contract
- handler:adapters-stop: implement the owner-qualified callable and its declared result/error contract
- handler:approval-clock-check: implement the owner-qualified callable and its declared result/error contract
- handler:approval-issue: implement the owner-qualified callable and its declared result/error contract
- handler:approval-keys-init: implement the owner-qualified callable and its declared result/error contract
- handler:approval-keys-recover: implement the owner-qualified callable and its declared result/error contract
- handler:approval-keys-retire: implement the owner-qualified callable and its declared result/error contract
- handler:approval-keys-revoke: implement the owner-qualified callable and its declared result/error contract
- handler:approval-keys-rotate: implement the owner-qualified callable and its declared result/error contract
- handler:approval-keys-status: implement the owner-qualified callable and its declared result/error contract
- handler:approval-policy-set: implement the owner-qualified callable and its declared result/error contract
- handler:approval-policy-status: implement the owner-qualified callable and its declared result/error contract
- handler:approval-prepare: implement the owner-qualified callable and its declared result/error contract
- handler:connections-connect: implement the owner-qualified callable and its declared result/error contract
- handler:connections-describe: implement the owner-qualified callable and its declared result/error contract
- handler:connections-launch: implement the owner-qualified callable and its declared result/error contract
- handler:connections-list: implement the owner-qualified callable and its declared result/error contract
- handler:connections-reconnect: implement the owner-qualified callable and its declared result/error contract
- handler:connections-revalidate: implement the owner-qualified callable and its declared result/error contract
- handler:connections-revoke: implement the owner-qualified callable and its declared result/error contract
- handler:connections-status: implement the owner-qualified callable and its declared result/error contract
- handler:connections-token: implement the owner-qualified callable and its declared result/error contract
- handler:connectors-list: implement the owner-qualified callable and its declared result/error contract
- handler:invoke: implement the owner-qualified callable and its declared result/error contract
- dynamic-validator:invoke: resolve operation/schema identity and validate native input, result and errors; absent validator refuses
- handler:operations-describe: implement the owner-qualified callable and its declared result/error contract
- handler:operations-list: implement the owner-qualified callable and its declared result/error contract
- handler:setup-check: implement the owner-qualified callable and its declared result/error contract
- handler:setup-checkpoints-enable: implement the owner-qualified callable and its declared result/error contract
- handler:setup-init: implement the owner-qualified callable and its declared result/error contract
- handler:setup-migrate: implement the owner-qualified callable and its declared result/error contract
- handler:setup-owner-stop: implement the owner-qualified callable and its declared result/error contract
