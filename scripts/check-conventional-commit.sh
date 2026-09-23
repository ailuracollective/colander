#!/bin/sh
# Validates a commit-message file against Conventional Commits.
# The local commit-msg hook (hk.pkl) and the CI pull-request title check
# run this same script.
#
# Usage: sh scripts/check-conventional-commit.sh <file>
#
# Rules:
#   1. Format and closed type vocabulary: delegated to
#      `hk util check-conventional-commit --allowed-types ...`, with the
#      vocabulary read from conventional-types.txt (hk skips fixup!/squash!/
#      amend! messages, which then pass the two checks below trivially).
#   2. The header (first line) must be at most 72 characters.
#   3. A scope, when present, must be lower-case.

set -u

script_dir=$(dirname "$0")
vocabulary="${script_dir}/conventional-types.txt"
file=${1:-}

status=0

fail() {
    echo "- $1" >&2
    status=1
}

if [ "$#" -ne 1 ]; then
    echo "error: usage: sh scripts/check-conventional-commit.sh <file>" >&2
    exit 1
fi

if [ ! -f "$vocabulary" ]; then
    echo "error: vocabulary file not found: ${vocabulary}" >&2
    exit 1
fi

if ! command -v hk >/dev/null 2>&1; then
    echo "error: hk is not on PATH; install hk (jdx/hk) to validate commit messages" >&2
    exit 1
fi

if [ ! -f "$file" ]; then
    echo "error: commit message file not found: ${file}" >&2
    exit 1
fi

# Build the comma-separated --allowed-types list from the vocabulary file.
# Skipping blank lines and the comment header keeps the file self-documenting.
types=$(awk 'NF && $0 !~ /^#/ { printf "%s%s", sep, $0; sep = "," } END { print "" }' "$vocabulary")

if [ -z "$types" ]; then
    echo "error: vocabulary file ${vocabulary} contains no types" >&2
    exit 1
fi

# Gate 1: format and closed vocabulary (also skips fixup!/squash!/amend!).
if ! hk util check-conventional-commit --allowed-types "$types" "$file"; then
    fail "commit message is not a conventional commit with an allowed type (${types})"
fi

# Gate 2: header length, first line only.
header_len=$(awk 'NR == 1 { print length }' "$file")
if [ "$header_len" -gt 72 ]; then
    fail "header must be at most 72 characters (got ${header_len})"
fi

# Gate 3: scope lower-case, first line only; empty when there is no scope.
# The sed prints the first parenthesised group when it is followed by the
# optional breaking-change marker and a colon. In POSIX BRE the capture
# group is \(...\) (zero-width delimiters) and literal parentheses are plain
# ( ) characters.
scope=$(sed -n '1s/^[^()]*(\([^)]*\))[!]*:.*/\1/p' "$file")
if [ -n "$scope" ]; then
    case "$scope" in
        *[[:upper:]]*)
            fail "scope must be lower-case (got \"${scope}\")"
            ;;
    esac
fi

exit "$status"