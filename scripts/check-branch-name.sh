#!/bin/sh
# Validates a branch name against the <username>/<type>/<short-description>
# convention and, when an author is given, matches the username segment to it.
# POSIX port of the reference check-branch-name.ts (cactu-care).
#
# Usage: sh scripts/check-branch-name.sh <branch> [author]
#
# The type must come from the closed vocabulary in conventional-types.txt;
# on success the script prints nothing. Violations accumulate and go to
# stderr, and the exit code is non-zero when any of them fired.

set -u

script_dir=$(dirname "$0")
vocabulary="${script_dir}/conventional-types.txt"
branch=${1:-}
author=${2:-}

status=0

fail() {
    echo "- $1" >&2
    status=1
}

if [ "$#" -lt 1 ]; then
    echo "error: usage: sh scripts/check-branch-name.sh <branch> [author]" >&2
    exit 1
fi

if [ ! -f "$vocabulary" ]; then
    echo "error: vocabulary file not found: ${vocabulary}" >&2
    exit 1
fi

# Build the alternation for the type segment from the vocabulary file.
types=$(awk 'NF && $0 !~ /^#/ { printf "%s%s", sep, $0; sep = "|" } END { print "" }' "$vocabulary")

if [ -z "$types" ]; then
    echo "error: vocabulary file ${vocabulary} contains no types" >&2
    exit 1
fi

regex="^[a-z0-9][a-z0-9-]*/(${types})/[a-z0-9][a-z0-9._-]*$"

if ! printf '%s\n' "$branch" | grep -Eq "$regex"; then
    fail "invalid branch name \"${branch}\""
    fail "expected format: <username>/<type>/<short-description>"
    fail "example: siddharthagf/feat/child-vaccination-reminders"
    fail "allowed types: ${types}"
fi

if [ -n "$author" ]; then
    username=$(printf '%s\n' "$branch" | sed 's#/.*##')
    username_lc=$(printf '%s\n' "$username" | tr '[:upper:]' '[:lower:]')
    author_lc=$(printf '%s\n' "$author" | tr '[:upper:]' '[:lower:]')
    if [ "$username_lc" != "$author_lc" ]; then
        fail "branch user \"${username}\" does not match the pull request author \"${author}\""
    fi
fi

exit "$status"