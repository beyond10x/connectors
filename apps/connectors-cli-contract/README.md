# `connectors` CLI contract

Local integration access through explicitly configured adapters

This generated package installs an unavailable handler. Application behavior enters through the `Handler` seam; schema-selected calls also require `DynamicValidator`. See `help.txt` for process options and `binding.json` for resolved types and targets.

## Commands

### `adapters describe`

Inspect one configured adapter and cached descriptor

Callable: `adapters-describe`. Input: `connectors.cli.AdapterDescribeInput`. Result: `connectors.cli.AdapterDescribeResult`.

Error `failure`: `connectors.cli.Failure`.

### `adapters list`

List configured adapter entries

Callable: `adapters-list`. Input: `connectors.cli.AdapterListInput`. Result: `connectors.cli.AdapterListResult`.

Error `failure`: `connectors.cli.Failure`.

### `adapters status`

Observe the existing adapter lifecycle owner

Callable: `adapters-status`. Input: `connectors.cli.AdapterStatusInput`. Result: `connectors.cli.AdapterStatusResult`.

Error `failure`: `connectors.cli.Failure`.

### `adapters stop`

Stop only the exact owned adapter incarnation

Callable: `adapters-stop`. Input: `connectors.cli.AdapterStopInput`. Result: `connectors.cli.AdapterStopResult`.

Error `failure`: `connectors.cli.Failure`.

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

Begin admitted local protected credential acquisition

Callable: `connections-connect`. Input: `connectors.cli.ConnectionConnectInput`. Result: `connectors.cli.ConnectionConnectResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections describe`

Inspect safe metadata for one connection

Callable: `connections-describe`. Input: `connectors.cli.ConnectionDescribeInput`. Result: `connectors.cli.ConnectionDescribeResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections list`

List safe connection metadata

Callable: `connections-list`. Input: `connectors.cli.ConnectionListInput`. Result: `connectors.cli.ConnectionListResult`.

Error `failure`: `connectors.cli.Failure`.

### `connections repair`

Repair the exact connection without changing its identity

Callable: `connections-repair`. Input: `connectors.cli.ConnectionRepairInput`. Result: `connectors.cli.ConnectionRepairResult`.

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

### `operations describe`

Inspect an operation and its exact schema identity

Callable: `operations-describe`. Input: `connectors.cli.OperationDescribeInput`. Result: `connectors.cli.OperationDescribeResult`.

Error `failure`: `connectors.cli.Failure`.

### `operations invoke`

Invoke once under selected schema, revision and current admission

Callable: `operations-invoke`. Input: `connectors.cli.OperationInvokeInput`. Result: `connectors.cli.OperationInvokeResult`.

Error `failure`: `connectors.cli.Failure`.

### `operations list`

List operations from selected cached metadata

Callable: `operations-list`. Input: `connectors.cli.OperationListInput`. Result: `connectors.cli.OperationListResult`.

Error `failure`: `connectors.cli.Failure`.

### `setup check`

Check local configuration and prerequisites without authentication

Callable: `setup-check`. Input: `null (inputless)`. Result: `connectors.cli.SetupCheckResult`.

Error `failure`: `connectors.cli.Failure`.

### `setup init`

Create private local configuration without starting services

Callable: `setup-init`. Input: `null (inputless)`. Result: `connectors.cli.SetupInitResult`.

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
- handler:connections-list: implement the owner-qualified callable and its declared result/error contract
- handler:connections-repair: implement the owner-qualified callable and its declared result/error contract
- handler:connections-revalidate: implement the owner-qualified callable and its declared result/error contract
- handler:connections-revoke: implement the owner-qualified callable and its declared result/error contract
- handler:connections-status: implement the owner-qualified callable and its declared result/error contract
- handler:operations-describe: implement the owner-qualified callable and its declared result/error contract
- handler:operations-invoke: implement the owner-qualified callable and its declared result/error contract
- dynamic-validator:operations-invoke: resolve operation/schema identity and validate native input, result and errors; absent validator refuses
- handler:operations-list: implement the owner-qualified callable and its declared result/error contract
- handler:setup-check: implement the owner-qualified callable and its declared result/error contract
- handler:setup-init: implement the owner-qualified callable and its declared result/error contract
