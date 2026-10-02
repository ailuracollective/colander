#!/bin/sh
# Contract guard for the generated header: the generated-but-committed header must match the
# Rust sources. The C ABI is frozen, so a header that disagrees with the sources is a contract
# change that never got reviewed.
#
# The guard is a CI gate (a step of the `ci` job in .github/workflows/ci.yml, reached through
# `cargo make header-check`, which is part of `cargo make ci`) and a local pre-commit check
# (`cargo make precommit`). It needs cbindgen 0.29.4 on PATH.
#
# Usage: sh scripts/check-header.sh
#
# Regenerates include/colander.h with cbindgen into a temporary file and
# compares it with the committed header. On any mismatch the script prints
# the exact regeneration command (see AGENTS.md) to stderr and exits 1,
# which fails the CI job or the commit; the header is never "fixed" in place.

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
    echo "Regenerate it and commit the change, then re-run the checks:" >&2
    echo "  cbindgen --config cbindgen.toml --crate colander --output include/colander.h" >&2
    exit 1
fi

echo "include/colander.h matches the Rust sources"