#!/bin/sh
# Which CI lanes a change needs (~/beyond10x/AGENTS.md, "Planning-only pull requests skip
# the tests"). The one path list of this repository is the three functions below.
#
#   changed-scope.sh classify <code|docs>  paths on stdin, one per line; prints run or skip
#   changed-scope.sh each <code|docs>      paths on stdin; prints "<run|skip> <path>" per path
#   changed-scope.sh github <code|docs>    the CI step: appends run=true or run=false to
#                                          $GITHUB_OUTPUT
#
# Lane `code` is every job that builds or tests the Rust workspace. Lane `docs` is the
# documentation site's build. A lane is skipped only on a pull request whose every changed
# path is skippable for it; every other event, an empty list, or a list that cannot be read
# runs it.
#
# .github/ci/changed-scope-test.sh holds the cases for both sides of the list, and fails
# when a Rust source names a path the list would skip.

set -eu

# Planning and prose files that a build, a test or the gate reads. Each one runs the lanes.
read_by_code() {
	case $1 in
	.engineering/planning/decision-blocker/*) return 0 ;;
	.engineering/planning/story/mcp-inbound-local-binding.md) return 0 ;;
	.engineering/planning/story/mcp-outbound-connection-lifecycle.md) return 0 ;;
	AGENTS.md | CHANGELOG.md | README.md) return 0 ;;
	docs/*) return 0 ;;
	esac
	return 1
}

# Planning and prose: the planning store, and Markdown outside the code, the contracts and
# the documentation site. Skips both lanes.
planning_or_prose() {
	read_by_code "$1" && return 1
	case $1 in
	.engineering/*) return 0 ;;
	adapters/* | apps/* | contracts/* | crates/* | deploy/* | ess/* | examples/* | spec-kinds/* | website/*) return 1 ;;
	*.md | *.mdx) return 0 ;;
	esac
	return 1
}

# Documentation-site files that no build or test of the code reads. Skips lane `code`;
# lane `docs` runs. The site's pages, examples and data are read by the gate and by tests.
docs_site_without_code() {
	case $1 in
	website/docs/* | website/examples/* | website/data/* | website/static/examples/*) return 1 ;;
	website/product.json | website/publication.json) return 1 ;;
	website/*) return 0 ;;
	esac
	return 1
}

skippable() {
	planning_or_prose "$2" && return 0
	[ "$1" = code ] && docs_site_without_code "$2" && return 0
	return 1
}

lane() {
	case ${1:-} in
	code | docs) ;;
	*)
		echo "changed-scope: lane must be code or docs, not '${1:-}'" >&2
		exit 2
		;;
	esac
}

classify() {
	seen=0
	while IFS= read -r path || [ -n "$path" ]; do
		[ -n "$path" ] || continue
		seen=1
		if ! skippable "$1" "$path"; then
			echo "changed-scope: $path needs lane $1" >&2
			echo run
			return 0
		fi
	done
	if [ "$seen" = 1 ]; then echo skip; else echo run; fi
}

each() {
	while IFS= read -r path || [ -n "$path" ]; do
		[ -n "$path" ] || continue
		if skippable "$1" "$path"; then echo "skip $path"; else echo "run $path"; fi
	done
}

github() {
	out=${GITHUB_OUTPUT:?GITHUB_OUTPUT is not set}
	if [ "${GITHUB_EVENT_NAME:-}" != pull_request ]; then
		echo "changed-scope: event '${GITHUB_EVENT_NAME:-}' always runs lane $1"
		echo run=true >>"$out"
		return 0
	fi
	if ! git rev-parse -q --verify 'HEAD^2' >/dev/null; then
		echo "changed-scope: HEAD is not the pull request's merge commit; lane $1 runs"
		echo run=true >>"$out"
		return 0
	fi
	if ! changed=$(git -c core.quotePath=false diff --no-renames --name-only 'HEAD^1' HEAD); then
		echo "changed-scope: git diff failed; lane $1 runs"
		echo run=true >>"$out"
		return 0
	fi
	echo "changed-scope: files this pull request changes against its base:"
	printf '%s\n' "$changed" | sed 's/^/  /'
	decision=$(printf '%s\n' "$changed" | classify "$1")
	if [ "$decision" = skip ]; then
		echo "changed-scope: only planning or prose files; lane $1 is skipped"
		echo run=false >>"$out"
	else
		echo "changed-scope: lane $1 runs"
		echo run=true >>"$out"
	fi
}

command=${1:-}
lane "${2:-}"
case $command in
classify) classify "$2" ;;
each) each "$2" ;;
github) github "$2" ;;
*)
	echo "usage: changed-scope.sh classify|each|github code|docs" >&2
	exit 2
	;;
esac
