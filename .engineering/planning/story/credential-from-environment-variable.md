---
format: aep.planning-md/3
id: story:credential-from-environment-variable
kind: story
status: active
title: A connection's credential is read from an environment variable, for op run
relations:
- decomposes: epic:business-teams-google-and-hubspot
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-12T16:07:28Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-12T16:07:29Z", actor: "human:operator", revision: 3}
---
## Need

Finance keeps the HubSpot key in 1Password and passes it with `op run`, which sets an environment
variable for one child process. `connections connect` and `reconnect` take a credential from
`--credential-file`, `--credential-stdin` or `--credential-prompt` only
(`apps/connectors/spec/cli.yaml:496-497,519-520`); none reads an environment variable, and stdin
expects the profile's JSON object (`{"token": ...}`).

## Acceptance

- `connections connect` and `reconnect` accept `--credential-env <NAME>`, exclusive with the other
  credential inputs. For a profile with one entry field the variable may hold the bare value; else
  it holds the JSON object the other inputs take.
- `op run --env-file <f> -- connectors connections connect hubspot --credential-env HUBSPOT_TOKEN`
  is the documented form, in the HubSpot page and the CLI reference.
- The value is never printed: an unset, empty or oversized variable is refused with the variable's
  name only; tests pin that the value is absent from stdout, stderr and the refusal.
