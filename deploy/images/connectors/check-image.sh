#!/usr/bin/env bash
# Checks of the published connectors image, shared by the release workflow and its pull-request path.
#
#   check-image.sh platforms <inspect-output> [<digest>]
#       The `docker buildx imagetools inspect` output lists linux/amd64 and linux/arm64 whatever its
#       padding, and, when a digest is given, its top-level Digest equals it.
#   check-image.sh revision <inspect-json> <commit>
#       The image config JSON (`imagetools inspect --format '{{json .Image}}'`) carries the
#       org.opencontainers.image.revision label, every occurrence equals the commit.
#       Exit 0 equal, 3 differs or absent.
#   check-image.sh self-test
#       Runs `platforms` over the captured 0.70.0 output and `revision` over small samples.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

platforms() {
  local file="$1" digest="${2:-}" arch
  if [ -n "$digest" ]; then
    test "$(awk '/^Digest:/ {print $2; exit}' "$file")" = "$digest" \
      || { echo "digest of the inspected image is not $digest" >&2; return 1; }
  fi
  for arch in amd64 arm64; do
    grep -Eq "^[[:space:]]*Platform:[[:space:]]+linux/${arch}\$" "$file" \
      || { echo "no linux/${arch} platform in $file" >&2; return 1; }
  done
}

revision() {
  local file="$1" commit="$2" found
  found="$(jq -r '[.. | objects | .["org.opencontainers.image.revision"]? | strings] | unique | join(" ")' "$file")"
  test "$found" = "$commit" || { echo "image revision is '${found}', not ${commit}" >&2; return 3; }
}

must_fail() { if "$@"; then echo "expected failure: $*" >&2; return 1; fi; }

self_test() {
  local fixture="$here/testdata/inspect-0.70.0.txt" tmp digest
  digest=sha256:00606821462e2324e129f799511c94ca4092ae8ab1535b8ba6519a429036ee4b
  platforms "$fixture" "$digest"
  must_fail platforms "$fixture" sha256:0000 2>/dev/null
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  grep -v 'linux/arm64' "$fixture" > "$tmp/no-arm64"
  must_fail platforms "$tmp/no-arm64" 2>/dev/null
  printf '%s\n' '{"linux/amd64":{"config":{"Labels":{"org.opencontainers.image.revision":"abc"}}},"linux/arm64":{"config":{"Labels":{"org.opencontainers.image.revision":"abc"}}}}' > "$tmp/same"
  printf '%s\n' '{"linux/amd64":{"config":{"Labels":{"org.opencontainers.image.revision":"abc"}}},"linux/arm64":{"config":{"Labels":{"org.opencontainers.image.revision":"def"}}}}' > "$tmp/mixed"
  printf '%s\n' '{"linux/amd64":{"config":{"Labels":{}}}}' > "$tmp/none"
  revision "$tmp/same" abc
  must_fail revision "$tmp/same" def 2>/dev/null
  must_fail revision "$tmp/mixed" abc 2>/dev/null
  must_fail revision "$tmp/none" abc 2>/dev/null
  echo "check-image self-test ok"
}

case "${1:-}" in
  platforms) shift; platforms "$@" ;;
  revision) shift; revision "$@" ;;
  self-test) self_test ;;
  *) echo "usage: check-image.sh platforms <inspect-output> [<digest>] | revision <inspect-json> <commit> | self-test" >&2; exit 2 ;;
esac
