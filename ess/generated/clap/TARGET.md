<!--
  generated from connectors v2
  model digest b46e08fdaea05ec1169f3d75212c80578b59fd21a94fe1d523f21714bddb663c
  contract digest d01ebb2ef8f96762b184433c1df72babf3565039aef3600d49dc1cd99b931c9b
  do not edit: regenerate with `ess synthesize --target clap`
-->
# Target notes — clap

For connectors v2. The `PLAN.md` beside this file is language-neutral and **byte-identical in every target's tree**; this document is what *this* target could not carry across it. Regenerate with `ess synthesize --target clap`.

2 weakening(s), 16 target refusal(s). A weakening is emitted code that holds less than the first target's; a target refusal is a capability the plan marks generated and this language cannot represent — a fact about the language, never about the specification.

## Weakened — emitted, with less than the first target holds

| the guarantee | what this target provides | capabilities affected |
| --- | --- | --- |
| a command's input arrives as its declared type | a handler receives `clap::ArgMatches`. The Rust target already emits every input as a type, and a fourth rendering of the type layer would be a fourth thing to keep in step — so this target emits the grammar and leaves the types where they are. | command contract |
| a shell completes every value a flag accepts | an enum-typed field completes its whole closed set; every other field completes as free text. A shell cannot enumerate a `String`, and offering a guess would complete values the system refuses. | command contract |

## Refused by this target — planned, not emitted

| capability | source | why |
| --- | --- | --- |
| command contract | `connectors.endpoint.ActivateCandidate` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.AuthorizeEndpoint` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.ConnectEventReceiver` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.CreateSetupSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.FinishSetupSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.MaterializeObservation` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.ReauthorizeEndpoint` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.ReconnectEventReceiver` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.RefreshObservation` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.RevokeEndpoint` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.StopEventReceiver` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.SuperviseEventReceiver` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.endpoint.VerifyEndpoint` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.runtime.InvokeOperation` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.runtime.SettleSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
| command contract | `connectors.runtime.TerminateSession` | no component declaring `reached_by: command_line` accepts this command, so no tree places it and there is no word to type |
