---
format: aep.planning-md/3
id: story:explicit-service-commands-help-and-completion
kind: story
status: active
title: describe, invoke and serve are reachable through help and completion
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: apps/connectors/src/legacy.rs
- confidence: cited
  path: apps/connectors/src/main.rs
- confidence: inferred
  path: apps/connectors/tests/explicit_commands_help_completion.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T07:53:30Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T07:53:31Z", actor: "human:timo", revision: 3}
---
## Outcome

The explicit service commands `describe`, `invoke` and `serve` are reachable through every route the
root help advertises: `connectors help <command>` prints the same usage as `<command> --help`, and
the Bash completion `connectors completions bash` emits completes them as root words with their own
flags (`--endpoint`, `--token-file`, `--allow-plaintext`, `--operation`, `--input`, `--config`), not
the flags of `operations describe` or `adapters describe`.

## Source

Read-only CLI audit of 2026-10-07, reproduced on 0.32.0 and 0.33.0: `connectors help describe` exits 2
with `cli_parse`; the completion script has no root case for `describe`, `invoke` or `serve`.

## Acceptance

- `connectors help describe|invoke|serve` exit 0 and print the usage `describe --help` prints.
- The completion script completes `describe`, `invoke` and `serve` at the root, and `connectors describe
  --<TAB>` offers exactly the compatibility command's flags (test sources the script in bash).
- The generated command tree, its help and its parse refusals are unchanged (`connectors-build cli --check`).
