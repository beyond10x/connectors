<!--
  generated from connectors v1
  model digest b0fd4c1db05804ca75838542993f1ffec6fc9e03d37d9fc92b7a2b217aab2e0f
  contract digest afc1c99e420e432371cb0bb48b200f48e3e2815b57363a1cb52f16cd394dcaad
  do not edit: regenerate with `ess synthesize --target clap`
-->
# Target notes — clap

For connectors v1. The `PLAN.md` beside this file is language-neutral and **byte-identical in every target's tree**; this document is what *this* target could not carry across it. Regenerate with `ess synthesize --target clap`.

2 weakening(s), 16 target refusal(s). A weakening is emitted code that holds less than the first target's; a target refusal is a capability the plan marks generated and this language cannot represent — a fact about the language, never about the specification.

## Weakened — emitted, with less than the first target holds

| the guarantee | what this target provides | capabilities affected |
| --- | --- | --- |
| a command's input arrives as its declared type | a handler receives `clap::ArgMatches`. The Rust target already emits every input as a type, and a fourth rendering of the type layer would be a fourth thing to keep in step — so this target emits the grammar and leaves the types where they are. | command contract |
| a shell completes every value a flag accepts | an enum-typed field completes its whole closed set; every other field completes as free text. A shell cannot enumerate a `String`, and offering a guess would complete values the system refuses. | command contract |

## Refused by this target — planned, not emitted

| capability | source | why |
| --- | --- | --- |
| command contract | `connectors.connection.ActivateCandidate` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.AuthorizeConnection` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.ConnectChannel` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.CreateConnectSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.FinishConnectSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.MaterializeObservation` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.ReauthorizeConnection` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.ReconnectChannel` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.RefreshObservation` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.RevokeConnection` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.StopChannel` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.SuperviseChannel` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.connection.VerifyConnection` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.runtime.InvokeOperation` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.runtime.SettleSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.runtime.TerminateSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
