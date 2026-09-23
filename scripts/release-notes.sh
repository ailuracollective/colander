#!/bin/sh
# Prints the CHANGELOG.md section for one version to stdout; the GitHub
# Release step in release.yml feeds the output into `gh release create`.
#
# Usage: sh scripts/release-notes.sh <version>    # "v0.1.1" or "0.1.1"
#
# A leading `v` is stripped, then the section whose header starts with
# `## ` followed by an optional `[`, `v?<version>` and a `]` or a space is
# printed: from that header up to, but excluding, the next `## ` header. This
# matches both the cocogitto remote template (`## [v0.1.0](url) - date`) and
# a plain header (`## 0.1.0 - date`). When no section matches, nothing is
# printed and the exit code is 0.

set -eu

if [ "$#" -ne 1 ]; then
    echo "error: usage: sh scripts/release-notes.sh <version>" >&2
    exit 1
fi

version=$1
version=${version#v}

awk -v v="$version" '
    /^## / {
        if (found) exit
        if ($0 ~ "^## (\\[)?v?" v "(\\]| )") {
            found = 1
            print
        }
        next
    }
    found && /^- - -$/ { exit }
    found { print }
' CHANGELOG.md