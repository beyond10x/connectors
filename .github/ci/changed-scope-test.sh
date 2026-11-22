#!/bin/sh
# Both sides of the path list in changed-scope.sh, the CI step's event handling on a real
# merge commit, and a guard: no Rust source names a path that the list would let a pull
# request change without building or testing. No network; well under a second.

set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
script=$root/.github/ci/changed-scope.sh
failures=0

fail() {
	echo "changed-scope-test: FAIL $*" >&2
	failures=$((failures + 1))
}

scratch_base=$root/.local/tmp
mkdir -p "$scratch_base"
scratch=$(mktemp -d "$scratch_base/changed-scope.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

# One path each: "<lane> <run|skip> <path>". Both sides of every entry of the list.
cat >"$scratch/single" <<'EOF'
code skip .engineering/planning/verification-report/connectors-v0741-release-scope.md
docs skip .engineering/planning/verification-report/connectors-v0741-release-scope.md
code skip .engineering/planning/story/some-story.md
code skip .engineering/evidence/story/some-story/review.md
code skip .engineering/project.yaml
code skip VISION.md
code skip .github/pull_request_template.md
docs skip VISION.md
code run crates/connectors-core/src/lib.rs
docs run crates/connectors-core/src/lib.rs
code run Cargo.lock
code run .github/workflows/rust-gate.yml
docs run .github/workflows/pages.yml
code run .github/ci/changed-scope.sh
code run crates/connectors-build/README.md
code run contracts/catalog/v1alpha1/semantics.md
code run adapters/sql/README.md
code run .engineering/planning/decision-blocker/some-blocker.md
code run .engineering/planning/story/mcp-inbound-local-binding.md
code run .engineering/planning/story/mcp-outbound-connection-lifecycle.md
code run README.md
code run CHANGELOG.md
code run AGENTS.md
code run docs/local-clock.md
docs run docs/local-clock.md
docs run website/docs/getting-started.md
code run website/docs/getting-started.md
code run website/examples/components.yaml
code run website/data/status.json
code run website/product.json
docs run website/src/css/custom.css
code skip website/src/css/custom.css
code skip website/sidebars.ts
EOF
for lane in code docs; do
	grep "^$lane " "$scratch/single" | cut -d' ' -f2- >"$scratch/want"
	cut -d' ' -f2 "$scratch/want" | sh "$script" each "$lane" >"$scratch/got"
	diff "$scratch/want" "$scratch/got" >"$scratch/diff" || fail "lane $lane, want (<) and got (>):
$(cat "$scratch/diff")"
done

# expect <lane> <run|skip> <path>...: one decision for a whole list.
expect() {
	lane=$1 want=$2
	shift 2
	got=$(printf '%s\n' "$@" | sh "$script" classify "$lane" 2>/dev/null)
	[ "$got" = "$want" ] || fail "lane $lane: want $want, got $got for: $*"
}
pr275=.engineering/planning/verification-report/connectors-v0741-release-scope.md
expect code skip "$pr275" .engineering/planning/epic/some-epic.md VISION.md
expect code run "$pr275" crates/connectors-core/src/lib.rs
expect docs run "$pr275" website/docs/concepts/contracts.md
expect code skip "$pr275" website/src/css/custom.css
expect code run
expect docs run

# The CI step on real merge commits and on other events.
repo=$scratch/repo
export GIT_AUTHOR_NAME=changed-scope GIT_AUTHOR_EMAIL=changed-scope@example.invalid
export GIT_COMMITTER_NAME=changed-scope GIT_COMMITTER_EMAIL=changed-scope@example.invalid
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p "$repo/.engineering/planning/story" "$repo/crates/a/src"
echo base >"$repo/crates/a/src/lib.rs"
git -C "$repo" init -q -b main
git -C "$repo" add -A
git -C "$repo" commit -q -m base
git -C "$repo" branch planning
git -C "$repo" branch code
echo main-moved >>"$repo/crates/a/src/lib.rs"
git -C "$repo" commit -q -am main-moved
git -C "$repo" switch -q planning
echo plan >"$repo/.engineering/planning/story/s.md"
git -C "$repo" add -A
git -C "$repo" commit -q -m planning
git -C "$repo" switch -q code
echo change >"$repo/crates/a/src/other.rs"
git -C "$repo" add -A
git -C "$repo" commit -q -m code
git -C "$repo" switch -q --detach main
git -C "$repo" merge -q --no-ff -m merge-planning planning
git -C "$repo" tag merge-planning
git -C "$repo" switch -q --detach main
git -C "$repo" merge -q --no-ff -m merge-code code
git -C "$repo" tag merge-code

# step <event> <revision> <lane>: what the CI step writes to GITHUB_OUTPUT.
step() {
	git -C "$repo" switch -q --detach "$2"
	: >"$scratch/output"
	(cd "$repo" && GITHUB_EVENT_NAME=$1 GITHUB_OUTPUT=$scratch/output sh "$script" github "$3" >/dev/null 2>&1)
	cat "$scratch/output"
}
[ "$(step pull_request merge-planning code)" = run=false ] || fail "a planning-only merge commit did not skip lane code"
[ "$(step pull_request merge-planning docs)" = run=false ] || fail "a planning-only merge commit did not skip lane docs"
[ "$(step pull_request merge-code code)" = run=true ] || fail "a merge commit with a code file skipped lane code"
[ "$(step push merge-planning code)" = run=true ] || fail "a push skipped lane code"
[ "$(step workflow_dispatch merge-planning docs)" = run=true ] || fail "a dispatch skipped lane docs"
[ "$(step pull_request planning code)" = run=true ] || fail "a commit that is not a merge commit skipped lane code"

# Guard: every tracked path a Rust source names under the planning store, the documentation
# site, or as Markdown, runs lane code. A name with a format placeholder stands for its
# directory. Names of fewer than three parts under .engineering/ and of one part elsewhere
# are prose about a whole tree, not a read.
cd "$root"
git ls-files >"$scratch/tracked"
{
	git grep -ohP -e '[\w./{}-]*(?:\.engineering|website)/[\w./{}-]*' -e '[\w./{}-]*\.mdx?' -- '*.rs' || :
} | sed -E 's#^(\./|\.\./)+##; s#\{.*##; s#/+$##' | sort -u >"$scratch/named"
awk '
	NR == FNR {
		file[$0] = 1
		n = split($0, part, "/")
		if (n > 1) {
			dir = part[1]
			for (i = 2; i < n; i++) { directory[dir] = 1; dir = dir "/" part[i] }
			directory[dir] = 1
		}
		next
	}
	{
		parts = split($0, part, "/")
		if ($0 ~ /^\.engineering\// && parts < 3) next
		if (parts < 2) { if ($0 in file) print $0; next }
		if ($0 in file) print $0
		else if ($0 in directory) print $0 "/changed-scope-probe.md"
	}
' "$scratch/tracked" "$scratch/named" >"$scratch/read"
checked=$(wc -l <"$scratch/read")
[ "$checked" -gt 0 ] || fail "the guard found no path a Rust source names"
sh "$script" each code <"$scratch/read" | sed -n 's/^skip //p' >"$scratch/unread"
while IFS= read -r path; do
	fail "a Rust source reads $path, which lane code skips; add it to read_by_code"
done <"$scratch/unread"

if [ "$failures" -gt 0 ]; then
	echo "changed-scope-test: $failures failed" >&2
	exit 1
fi
echo "changed-scope-test: all cases pass; guard checked $checked paths Rust sources name"
