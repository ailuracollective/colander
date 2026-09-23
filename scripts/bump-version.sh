#!/bin/sh
# Updates the single version source: the [package] version line in Cargo.toml.
# Cocogitto runs this as a pre-bump hook so that CARGO_PKG_VERSION always
# agrees with the release tag and the CHANGELOG.md section.
#
# Usage: sh scripts/bump-version.sh <version>
#
# Only the anchored `^version = ` line inside [package] is rewritten. The
# dependencies use inline `name = "x"` entries with no `version =` key, so a
# plain `version =` substitution can never touch them.

set -eu

if [ "$#" -ne 1 ]; then
    echo "error: usage: sh scripts/bump-version.sh <version>" >&2
    exit 1
fi

version=$1

case "$version" in
    *'"'*)
        echo "error: invalid version \"${version}\"" >&2
        exit 1
        ;;
esac

tmp=$(mktemp "${TMPDIR:-/tmp}/colander-cargo-version.XXXXXX")
trap 'rm -f "$tmp"' EXIT HUP INT TERM

if ! sed -e "s/^version = .*/version = \"${version}\"/" Cargo.toml > "$tmp"; then
    echo "error: failed to rewrite the version in Cargo.toml" >&2
    exit 1
fi

mv "$tmp" Cargo.toml
tmp=
echo "Cargo.toml version updated to ${version}"