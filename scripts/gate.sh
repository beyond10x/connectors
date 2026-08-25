#!/usr/bin/env bash
# The repository gate: every Cargo workspace's locked tests, the catalog lock
# verifier, and the documentation checks. Green here is the bar for main.
#
# `bash scripts/gate.sh` with no argument runs everything, which is what a
# developer wants and what this script has always done. The three flags exist so
# CI can shard the same work without keeping a second copy of the workspace
# list: the eleven workspaces do not share a `target/` directory, and building
# all of them on one runner needs about 39 GB, which no hosted runner has. A
# second list in a workflow file would be a list that drifts.
#
#   --list-workspaces   print the workspaces, one per line, for a CI matrix
#   --workspace <path>  run one workspace's locked tests
#   --final             the catalog lock verifier and the documentation checks
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

workspaces=(
  .
  crates/connectors-runtime
  crates/connectors-cli
  crates/driver-audio
  crates/driver-speech
  crates/driver-cdp
  crates/driver-sip
  crates/driver-sql
  crates/rtvbp-voice-endpoint
  crates/voice-local-audio
  crates/voice-runtime
)

test_workspace() {
  printf 'gate: [%s] cargo test --workspace --locked\n' "$1"
  (cd "$root/$1" && cargo test --workspace --locked)
}

final_checks() {
  (cd "$root" && cargo run --locked --offline -p catalog-cli -- check)
  python3 "$root/scripts/check-links.py"
  python3 "$root/scripts/check-stories.py"
}

case "${1-}" in
  --list-workspaces)
    printf '%s\n' "${workspaces[@]}"
    ;;
  --workspace)
    [ $# -eq 2 ] || { printf '%s: --workspace needs exactly one path\n' "$0" >&2; exit 2; }
    for workspace in "${workspaces[@]}"; do
      if [ "$workspace" = "$2" ]; then
        test_workspace "$2"
        printf 'gate: [%s] green\n' "$2"
        exit 0
      fi
    done
    printf '%s: `%s` is not one of the gate workspaces; run --list-workspaces\n' "$0" "$2" >&2
    exit 2
    ;;
  --final)
    final_checks
    printf 'gate: final checks green\n'
    ;;
  "")
    for workspace in "${workspaces[@]}"; do
      test_workspace "$workspace"
    done
    final_checks
    printf 'gate: green\n'
    ;;
  *)
    printf '%s: unknown argument `%s`; expected --list-workspaces, --workspace <path> or --final\n' \
      "$0" "${1-}" >&2
    exit 2
    ;;
esac
