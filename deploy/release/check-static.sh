#!/bin/sh
# Checks of the static musl release binaries, shared by the release workflow and the pull-request
# workflow `musl-static.yml`.
#
#   check-static.sh check <dir> <version> <binary>...
#       Each binary in <dir> is statically linked: `file` says statically linked (or static-pie
#       linked), it has no INTERP program header and its dynamic section, if any, names no
#       shared library (NEEDED). Then each one, run in a plain `alpine:3` container with no
#       network and <dir> mounted read-only, prints "<binary> <version>" for `--version`.
#   check-static.sh self-test
#       The static check refuses a dynamically linked binary of the host (`ls`).
set -eu

image=alpine:3

static() {
	path=$1
	kind=$(file -b "$path")
	case $kind in
	*"statically linked"* | *"static-pie linked"*) ;;
	*)
		echo "$path is not statically linked: $kind" >&2
		return 1
		;;
	esac
	if readelf -lW "$path" | grep -q '^[[:space:]]*INTERP[[:space:]]'; then
		echo "$path names a program interpreter (INTERP)" >&2
		return 1
	fi
	if readelf -dW "$path" | grep -q '(NEEDED)'; then
		echo "$path needs a shared library:" >&2
		readelf -dW "$path" | grep '(NEEDED)' >&2
		return 1
	fi
	echo "static: $path ($kind)"
}

pull() {
	for attempt in 1 2 3; do
		if docker pull --quiet "$image"; then
			return 0
		fi
		echo "docker pull $image failed (attempt $attempt of 3)" >&2
		sleep 10
	done
	return 1
}

check() {
	[ $# -ge 3 ] || {
		echo "usage: check-static.sh check <dir> <version> <binary>..." >&2
		exit 2
	}
	dir=$(cd "$1" && pwd)
	version=$2
	shift 2
	for binary in "$@"; do
		static "$dir/$binary"
	done
	pull
	for binary in "$@"; do
		answer=$(docker run --rm --network none -v "$dir:/b:ro" "$image" "/b/$binary" --version)
		if [ "$answer" != "$binary $version" ]; then
			echo "$binary in $image answered '$answer', not '$binary $version'" >&2
			return 1
		fi
		echo "$image: $answer"
	done
}

self_test() {
	dynamic=$(command -v ls)
	if static "$dynamic" 2>/dev/null; then
		echo "self-test: the static check accepted the dynamically linked $dynamic" >&2
		return 1
	fi
	echo "self-test: the static check refuses the dynamically linked $dynamic"
}

case ${1:-} in
check)
	shift
	check "$@"
	;;
self-test) self_test ;;
*)
	echo "usage: check-static.sh check <dir> <version> <binary>... | self-test" >&2
	exit 2
	;;
esac
