---
format: aep.planning-md/1
id: decision-blocker:ess-upgrade-decision
kind: decision-blocker
status: open
title: Nobody has decided whether to upgrade the ESS pin to 0.18.0
relations:
- blocks: story:ess-pin-upgrade
revision: 1
---
## Question

Should the GitLab generation pin move from ESS 0.9.2 to 0.18.0, or stay at 0.9.2?

## What is known

- Pin: `crates/connectors-spec/src/v2.rs:8` = `ess 0.9.2`.
- Installed: `~/.cargo/bin/ess` = 0.18.0 and `~/.local/bin/ess` = 0.9.2 (observed 2026-09-08).
- Compatibility of 0.18.0 with the generator's CLI invocations and bundle output: not tested. I don't know.
- Cost of upgrading: regeneration, bundle review, live GitLab re-acceptance (`docs/gitlab-generation.md:18`).

## What clears this

An operator decision, recorded here on clearing: upgrade (then `story:ess-pin-upgrade` proceeds with a trial regeneration first) or stay (then that story is rejected and the docs state 0.9.2 as supported). Default on silence: stay at 0.9.2; `story:ess-executable-pin` makes that choice reproducible.
