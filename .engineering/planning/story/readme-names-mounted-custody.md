---
format: aep.planning-md/3
id: story:readme-names-mounted-custody
kind: story
status: draft
title: The README names the mounted-secret custody binding
summary: README.md:199-203 says one fixed binding per platform; mounted_secret is selectable on Linux
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/connectors-docs/src/vocabulary.rs
revision: 2
---
## Outcome

The README says what the custody contract and the CLI guide already say: on Linux a configuration
can select the read-only `mounted_secret` binding, which is how Connectors runs in a container with
no keyring.

## Today

- `README.md:199-203`: "Credential custody has one binding per platform, fixed when the binary is
  built", the Secret Service on Linux.
- That is false since story:mounted-secret-custody (implemented): a `[custody]` table selects
  `mounted_secret` (`contracts/auth/custody/v1alpha1/semantics.md` §5.2;
  `website/docs/guides/set-up-the-local-cli.md:83-89`).
- A reader of the README concludes Connectors needs a keyring in a container. The Loom Kubernetes
  agent design did (its question Q3, citing `README.md:198-200`).

## Scope

- `README.md:199-203`: name both Linux bindings, what `mounted_secret` refuses (OAuth refresh,
  approval signing; `docs/local-mounted-secret-custody.md:124-126`) and link
  `docs/local-mounted-secret-custody.md`.
- `CHANGELOG.md`.

## Acceptance (named conformance scenarios)

- `crates/connectors-docs/src/vocabulary.rs::the_readme_names_every_custody_binding`: fails while
  the README names one Linux binding; passes when it names `mounted_secret` and links the guide.

## Order

Independent of the other stories under epic:connectors-with-claude.
story:mounted-custody-kind-secret-volume proves the binding on a real Secret volume; this story only
corrects the README.
