#!/usr/bin/env bash
# Prove that the deployed posture cannot carry the loopback plaintext Identity exception.
#
# `--features local-identity` lets `connectors serve hosted` resolve access tokens against a
# plaintext HTTP Identity. In a deployment that puts every access token, and the authority answer
# that admits it, on the network in the clear: anybody on the path reads a token and is then that
# principal. So the exception must not merely be off by default — a deployment build has to refuse
# to compile with it.
#
# Three facts are asserted here, none of which an environment variable, a configuration file or a
# request can change:
#
#   1. `cargo check --release --features local-identity` fails. A release profile clears
#      `debug_assertions`, and `crates/identity-http/src/lib.rs` raises `compile_error!` in exactly
#      that combination.
#   2. No manifest turns `debug-assertions` back on for the release profile, which is the one way
#      rule 1 could be defeated without editing the refusal itself.
#   3. Every build that produces a shipped binary passes no `--features` at all and builds
#      `--release`, so what ships is the default feature set even before rule 1 applies. There are
#      two such builds: the image, and the release workflow that attaches CLI archives to a tag.
#
# The runtime half of the guard — a feature build refusing to serve anything but a loopback
# listener resolving a loopback Identity — is asserted by `identity-http`'s own unit tests and by
# `connectors-runtime`'s pre-bind check, not here.
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

refuse() {
  printf 'local-identity guard failed: %s\n' "$*" >&2
  exit 1
}

# 1. The deployment profile refuses to compile the feature.
output=$(mktemp "${TMPDIR:-/tmp}/b10x-local-identity-guard.XXXXXXXX")
trap 'rm -f -- "$output"' EXIT
if cargo check --manifest-path crates/connectors-cli/Cargo.toml --locked --release \
  --features local-identity >"$output" 2>&1; then
  refuse 'cargo check --release --features local-identity succeeded; the release build must refuse it'
fi
if ! grep -q 'local-identity' "$output"; then
  printf '%s\n' "$(cat "$output")" >&2
  refuse 'the release build failed for some reason other than the local-identity refusal'
fi

# 2. Nothing re-enables debug assertions in a release build, which is what rule 1 keys on.
for manifest in crates/connectors-cli/Cargo.toml crates/connectors-runtime/Cargo.toml \
  crates/identity-http/Cargo.toml; do
  if grep -qE '^[[:space:]]*debug[-_]assertions' "$manifest"; then
    refuse "$manifest sets debug-assertions; the release refusal keys on that profile flag"
  fi
done

# 3. The image build never selects a feature, so the shipped binary cannot contain the path.
if grep -n -- '--features' Dockerfile; then
  refuse 'Dockerfile selects a Cargo feature; the released image must build the default set'
fi
if ! grep -q 'cargo build --manifest-path crates/connectors-cli/Cargo.toml --locked --release' \
  Dockerfile; then
  refuse 'Dockerfile no longer builds the product binary with --release; rule 1 no longer covers the image'
fi

# 3b. Same rule for the release workflow, which is the other place a shipped binary is produced.
# The image was the only one when this guard was written; a tag now attaches CLI archives for five
# targets, and a `--features local-identity` slipped into that matrix would ship the plaintext
# Identity path to anyone who downloads one.
workflow=.github/workflows/release.yml
if [ -f "$workflow" ]; then
  if grep -n -- '--features' "$workflow"; then
    refuse "$workflow selects a Cargo feature; released archives must build the default set"
  fi
  if ! grep -q 'cargo build --manifest-path "\$CLI_MANIFEST" --locked --release' "$workflow"; then
    refuse "$workflow no longer builds the product binary with --locked --release; rule 1 no longer covers the archives"
  fi
else
  refuse "$workflow is missing; the released archives are built by something this guard cannot see"
fi

printf 'local-identity is refused by the release profile and absent from the image and archive builds\n'
