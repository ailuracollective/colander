# Releases

Releasing colander is a deterministic pipeline with two human inputs: the
**when** and, for now, the **crates.io publish**. You run `cog bump --auto`
locally on `master` with a clean tree; cocogitto computes the next version
from Conventional Commits, and every following step — header guard, version
bump, changelog, commit, tag, push, CI verification, GitHub Release — is
automation that either succeeds or fails loudly. Publishing to crates.io is
the one step that stays manual until the maintainer re-enables it in CI.

## Purpose

Conventions and the frozen contract make releases repeatable and
reviewable. Leaving the version in human hands after the first release
means drift: CHANGELOG sections that mismatch the tag, a Cargo.toml version
that disagrees with `CARGO_PKG_VERSION`, a stale generated header shipped in
the artifact. The human decides WHEN; automation does the deterministic
parts.

## The flow

Preview first: `cog bump --dry-run --auto` prints only the next version and
mutates nothing. When the version looks right, run the real bump:

| Step            | What happens                                                                                                                                                                           |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pre-bump hooks  | `scripts/check-header.sh` guards the generated header, `scripts/bump-version.sh <version>` rewrites the sole `[package]` version in Cargo.toml, then `cargo check` compiles the result |
| Changelog       | Cocogitto appends the new section to CHANGELOG.md                                                                                                                                      |
| Commit and tag  | A `chore(version): X.Y.Z` commit, then tag `vX.Y.Z`                                                                                                                                    |
| Post-bump hooks | `git push origin master` and `git push origin vX.Y.Z`                                                                                                                                  |
| CI              | The `v*` tag push triggers `.github/workflows/release.yml`                                                                                                                             |

Cocogitto aborts the bump when the tree is dirty or a pre-bump hook fails,
so a failed run leaves no half-released state behind (see Recovery).

## Why the header guard exists

`include/colander.h` is generated but committed, and the C ABI is part of
the frozen contract: a release must not ship a header that disagrees with
the sources. The pre-bump hook `scripts/check-header.sh` regenerates the
header with cbindgen, compares it byte-for-byte with the committed file and
aborts the bump on any difference. The header is never patched in place — to
regenerate it after an ABI change, run:

    cbindgen --config cbindgen.toml --crate colander --output include/colander.h

and commit the result before bumping.

## What CI does on the tag

`.github/workflows/release.yml` runs on the pushed `v*` tag and does, in
order:

1. `cargo make ci` — the full local CI set (fmt-check, lint, check, test,
   build), proving the tree is releasable.
2. `cargo build --release` — the cdylib `target/release/libcolander.so`
   that becomes the GitHub Release artifact.
3. `gh release create` — creates the GitHub Release with the CHANGELOG
   section for the tag (extracted by `scripts/release-notes.sh`) plus the
   cdylib.

The workflow does NOT publish to crates.io. Publishing is a deliberate
manual step (see below) so the automation can never push a version that the
maintainer did not explicitly decide to publish.

## Manual crates.io publish

crates.io versions are permanent and irreversible, so the publish is a human
decision run locally from a clean checkout of the tag, never from CI. To
publish the version behind a tag:

```sh
cargo login                 # one-time: token from https://crates.io/me (scope: publish)
git checkout vX.Y.Z
cargo make ci               # same proof CI ran on the tag
cargo publish --locked      # uploads the exact tagged tree
```

If the workflow's release step fails after a manual publish (or vice versa),
either half can be redone independently: the release is re-runnable with
`gh release create` and the publish is idempotent until the version is taken.

## Versioning policy

Cocogitto derives the bump from the closed Conventional Commit vocabulary:

| Commits since the last tag                  | Bump                       |
| ------------------------------------------- | -------------------------- |
| `fix:` only                                 | patch (`0.1.0` -> `0.1.1`) |
| `feat:` present                             | minor (`0.1.0` -> `0.2.0`) |
| Breaking change (`!` or `BREAKING CHANGE:`) | major (`0.1.0` -> `1.0.0`) |

`cog bump --auto` NEVER auto-bumps a 0.y.z crate to 1.0.0; going 1.0.0 is a
deliberate act (`cog bump --major`).

## First-release notes

The current 0.1.0 has no tag and will never be published: `from_latest_tag = true` makes cocogitto compute the first version from the accumulated
commits, so the first `cog bump --auto` produces the first tag, the first
CHANGELOG section and the first GitHub Release in one step. Publishing that
version to crates.io is a separate manual decision; published versions
cannot be deleted, so the first publish decides the crate's public history.

## Recovery

- A pre-bump hook failure aborts the bump and cocogitto stashes the partial
  changes under `cog_bump_<version>`; `git stash apply` restores them.
- Post-bump hooks have no rollback, so they only push: a failed push leaves
  the local commit and tag intact and recoverable.
- If the workflow's Release step fails, create the release manually with
  `gh release create` — the step is idempotent for a missing release.
- A failed manual `cargo publish` leaves no half-state: fix the cause and
  re-run; the only permanent case is a version that was already published,
  which is why `cargo make ci` runs before every publish attempt.
