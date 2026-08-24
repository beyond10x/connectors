#!/usr/bin/env bash
# The repository gate: every Cargo workspace's locked tests, the catalog lock
# verifier, and the documentation checks. Green here is the bar for main.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for workspace in \
  . \
  crates/connectors-runtime \
  crates/connectors-cli \
  crates/driver-audio \
  crates/driver-speech \
  crates/driver-cdp \
  crates/driver-sip \
  crates/driver-sql \
  crates/rtvbp-voice-endpoint \
  crates/voice-local-audio \
  crates/voice-runtime; do
  printf 'gate: [%s] cargo test --workspace --locked\n' "$workspace"
  (cd "$root/$workspace" && cargo test --workspace --locked)
done
(cd "$root" && cargo run --locked --offline -p catalog-cli -- check)
python3 "$root/scripts/check-links.py"
python3 "$root/scripts/check-stories.py"
printf 'gate: green\n'
