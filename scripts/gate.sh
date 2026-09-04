#!/usr/bin/env bash
# The repository gate: every Cargo workspace's locked tests, the catalog lock
# verifier, and the documentation checks. Green here is the bar for main.
#
# `bash scripts/gate.sh` with no argument runs everything, which is what a
# developer wants and what this script has always done. The three flags exist so
# CI can shard the same work without keeping a second copy of the workspace
# list: the twelve workspaces do not share a `target/` directory, and building
# all of them on one runner needs about 41 GB, which no hosted runner has. A
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
  crates/connectors-console
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
  if [ "$1" = crates/connectors-runtime ]; then
    printf 'gate: [%s] cargo test --workspace --locked --no-default-features\n' "$1"
    (cd "$root/$1" && cargo test --workspace --locked --no-default-features)
  fi
}

# The ESS specification and the artifacts projected from it.
#
# `ess` is not a workspace dependency and is not built here: it is an installed tool, the way
# `python3` above is. A gate step that silently skipped when the tool was missing would exit 0
# without checking anything, which is indistinguishable from a green run — so a missing `ess` is a
# refusal, not a skip.
#
# Only the emitted crate and the two notes are committed. `ess generate synthesize` also writes
# `plan.json` and `target.json`, and `json-schemas.toml` admits a tracked JSON document only as a
# registered schema, a document validated against one, or vendored source. No schema exists for the
# ESS report shape, so committing either would be a JSON file this repository cannot classify;
# `crates/catalog-build/tests/main/json_governance.rs` is what says so.
#
# `ess` has to be on the runner's path for this to run at all. It is not a workspace dependency and
# `cargo fetch` does not bring it, so a job that calls `--final` installs it first; see the
# `Catalog lock verifier and documentation checks` step of `.github/workflows/release.yml`.
ess_checks() {
  if ! command -v ess >/dev/null 2>&1; then
    printf 'gate: `ess` is not on PATH; it is what projects ess/system into ess/generated/clap\n' >&2
    return 1
  fi
  printf 'gate: [ess] specify validate --path ess/system\n'
  (cd "$root" && ess specify validate --path ess/system)

  # Byte-identity, and why it is here rather than in a per-workspace lane. It needs the `ess`
  # binary, which only the `checks` job installs — a matrix runner has none, and a lane that
  # skipped would exit 0 without checking anything. What it adds over the per-lane check is the
  # part `crates/connectors-cli/tests/cli_surface.rs` cannot see without the tool: the two digest
  # lines, `handler.rs`, `main.rs`, the emitted manifest and `PLAN.md`. The word-for-word
  # comparison of the command tree itself — every group and every `--help` line against
  # `ess/system/components.yaml` — runs in the `crates/connectors-cli` lane on every push, so a
  # stale tree does not wait for a release cut to be refused.
  printf 'gate: [ess] the committed clap tree is what the specification generates\n'
  local fresh status=0
  fresh=$(mktemp -d)
  (cd "$root" && ess generate synthesize --path ess/system --target clap --out "$fresh" >/dev/null) || status=$?
  # Cleaned up on the failing path too. `set -e` leaves a function the moment `diff` reports a
  # difference, so a `trap ... RETURN` would never fire and every stale-contract run would leave a
  # copy of the generated tree behind.
  if [ "$status" -eq 0 ]; then
    rm -f "$fresh/plan.json" "$fresh/target.json"
    diff -ru "$root/ess/generated/clap" "$fresh" || status=$?
  fi
  rm -rf "$fresh"
  return "$status"
}

final_checks() {
  (cd "$root" && cargo run --locked --offline -p catalog-cli -- check)
  python3 "$root/scripts/check-links.py"
  python3 "$root/scripts/check-stories.py"
  ess_checks
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
