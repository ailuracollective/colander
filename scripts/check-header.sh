#!/bin/sh
# Contract guard for the cocogitto pre-bump hook: the generated-but-committed
# header must match the Rust sources before a release is tagged.
#
# Usage: sh scripts/check-header.sh
#
# Regenerates include/colander.h with cbindgen into a temporary file and
# compares it with the committed header. On any mismatch the script prints
# the exact regeneration command (see AGENTS.md) to stderr and exits 1,
# which aborts `cog bump`; the header is never "fixed" in place.

set -eu

tmp=
cleanup() {
    if [ -n "$tmp" ]; then
        rm -f "$tmp"
    fi
}
trap cleanup EXIT HUP INT TERM

if ! command -v cbindgen >/dev/null 2>&1; then
    echo "error: cbindgen is not on PATH; install it with 'cargo install cbindgen --locked'" >&2
    exit 1
fi

tmp=$(mktemp "${TMPDIR:-/tmp}/colander-header.XXXXXX")
cbindgen --config cbindgen.toml --crate colander --output "$tmp"

if ! cmp -s "$tmp" include/colander.h; then
    echo "error: include/colander.h is out of date with the Rust sources." >&2
    echo "Regenerate it and commit the change, then retry the bump:" >&2
    echo "  cbindgen --config cbindgen.toml --crate colander --output include/colander.h" >&2
    exit 1
fi

echo "include/colander.h matches the Rust sources"