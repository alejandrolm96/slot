#!/bin/sh
# Fetch the pinned rcheevos source. Nothing is compiled here: slot-cheevos'
# build.rs does that with cc, so cargo picks the target and the C is rebuilt
# when it should be. This script only puts the right source on disk.
#
# rcheevos has no usable release tags. The newest, v9.2.0, points at a commit
# from 2020 whose source tree predates rc_client entirely, so a commit is the
# only honest pin. That matches cores/mgba and cores/gpsp, which pin commits
# too.
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
repo="https://github.com/RetroAchievements/rcheevos"

usage() {
	echo "usage: $0 stamp COMMIT | fetch COMMIT DEST" >&2
	exit 2
}

sha256() {
	if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi |
		cut -d' ' -f1
}

stamp() {
	echo "commit=$1"
	echo "source=$repo/tree/$1"
	for p in "$here"/*.patch; do
		[ -e "$p" ] || continue
		echo "patch=$(basename "$p") sha256:$(sha256 "$p")"
	done
}

fetch() {
	commit="$1" dest="$2"

	mkdir -p "$dest"
	[ -d "$dest/.git" ] || git init -q "$dest"
	git -C "$dest" cat-file -e "$commit^{commit}" 2>/dev/null ||
		git -C "$dest" fetch -q --depth 1 "$repo" "$commit"
	git -C "$dest" checkout -q --force --detach "$commit"
	git -C "$dest" clean -q -fdx
	for p in "$here"/*.patch; do
		[ -e "$p" ] || continue
		git -C "$dest" apply "$p"
	done

	# A fetch that leaves the tree without the files build.rs compiles is a
	# failure worth reporting here, not a confusing cc error later.
	for f in include/rc_client.h src/client/rc_client.c src/libretro/rc_libretro.c; do
		if [ ! -f "$dest/$f" ]; then
			echo "rcheevos $commit has no $f: wrong commit, or the source tree moved" >&2
			exit 1
		fi
	done

	stamp "$commit" >"$dest/.slot-stamp"
}

case "${1:-}" in
stamp)
	[ $# -eq 2 ] && [ -n "$2" ] || usage
	stamp "$2"
	;;
fetch)
	[ $# -eq 3 ] && [ -n "$2" ] && [ -n "$3" ] || usage
	fetch "$2" "$3"
	;;
*)
	usage
	;;
esac
