# Releases

Releasing colander is a deterministic pipeline with deliberate human
decision points. The **version bump and tag** are automated: every push to
`master` runs `.github/workflows/bump.yml`, where cocogitto computes the
next version from Conventional Commits, guards the generated header, bumps
`Cargo.toml`, writes `CHANGELOG.md`, commits `chore(version): X.Y.Z` and
pushes the tag. The two steps that stay human decisions are the **GitHub
Release** (created only on demand, never from the tag push alone) and, for
now, the **crates.io publish**. `cargo make bump` is the same pipeline run
locally, and `cargo make bump-dry-run` previews it without touching
anything.

## Purpose

Conventions and the frozen contract make releases repeatable and
reviewable. Leaving the version in human hands after the first release
means drift: CHANGELOG sections that mismatch the tag, a Cargo.toml version
that disagrees with `CARGO_PKG_VERSION`, a stale generated header shipped in
the artifact. Computing the version is mechanical and derives from commits
that CI has already verified, so automation does it; publishing stays in
human hands, because a published version is effectively permanent.

## The flow

Preview first: `cargo make bump-dry-run` prints only the next version and
mutates nothing. On a push to `master` CI runs the same pipeline
unattended; the local command is the manual equivalent.

| Step            | What happens                                                                                                                                                                           |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pre-bump hooks  | `scripts/check-header.sh` guards the generated header, `scripts/bump-version.sh <version>` rewrites the sole `[package]` version in Cargo.toml, then `cargo check` compiles the result |
| Changelog       | Cocogitto appends the new section to CHANGELOG.md                                                                                                                                      |
| Commit and tag  | A `chore(version): X.Y.Z` commit, then tag `vX.Y.Z`                                                                                                                                    |
| Post-bump hooks | `git push origin master` and `git push origin vX.Y.Z`                                                                                                                                  |
| CI              | The `v*` tag push runs the full CI set (`ci.yml`) as verification only — it never creates a Release                                                                                    |
| GitHub Release  | Created only when you run the `Release` workflow on demand (see below)                                                                                                                 |

Cocogitto aborts the bump when the tree is dirty or a pre-bump hook fails,
so a failed run leaves no half-released state behind (see Recovery).

## What the automation does and does not do

`.github/workflows/bump.yml` runs on every push to `master`, and on demand
via `workflow_dispatch` with two inputs: `dry_run` (preview only) and
`version` (force a specific version). `master` is the only branch that
releases; cocogitto's own branch whitelist enforces it.

- A push whose commits are only `chore`, `docs`, `refactor`, `test`, `ci`,
  `build` or `perf` produces **no tag** and a **green** run: cocogitto
  prints "No conventional commits for your repository that required a bump"
  and exits 0 without touching anything. There is no guard step predicting
  bump-worthiness, because cocogitto is the only source of truth for it.
- The `chore(version): X.Y.Z` commit that the bump pushes back to `master`
  re-triggers the workflow once. That second run finds a chore-only range,
  so the run ends green without creating anything. The automation cannot
  loop.
- The workflow installs the Rust toolchain, cargo-make, dprint, cocogitto
  7.0.0 and cbindgen 0.29.4 (the pre-bump header guard runs it), then
  builds on the tip of `master` rather than on whichever commit happened to
  trigger the run. Two pushes never bump at once (a concurrency group).
- What it never does: create a GitHub Release, publish to crates.io, or
  force a version on its own. `version` is empty on a push, so a push can
  only ever bump by the rules below.

## Why the header guard exists

`include/colander.h` is generated but committed, and the C ABI is part of
the frozen contract: a release must not ship a header that disagrees with
the sources. The pre-bump hook `scripts/check-header.sh` regenerates the
header with cbindgen, compares it byte-for-byte with the committed file and
aborts the bump on any difference. The header is never patched in place — to
regenerate it after an ABI change, run:

    cbindgen --config cbindgen.toml --crate colander --output include/colander.h

and commit the result before bumping.

## Creating the GitHub Release (only when you say so)

Pushing the tag never creates a GitHub Release. The Release is a separate,
deliberate step; run `.github/workflows/release.yml` when you want it, from
the Actions UI (Actions -> Release -> Run workflow -> tag) or with:

```sh
gh workflow run release.yml -f tag=vX.Y.Z
```

The workflow does, in order:

1. Checks out the requested tag and confirms it exists.
2. `cargo make ci` — the full local CI set (fmt-check, lint, check, test,
   build), proving the tree is releasable.
3. `cargo build --release` — the cdylib `target/release/libcolander.so`
   that becomes the GitHub Release artifact.
4. `gh release create` — creates the GitHub Release with the CHANGELOG
   section for the tag (extracted by `scripts/release-notes.sh`) plus the
   cdylib.

The workflow does NOT publish to crates.io. Publishing is a deliberate
manual step (see below) so the automation can never push a version that the
maintainer did not explicitly decide to publish.

## Manual crates.io publish

A published crates.io version is discouraged from deletion and can be removed
only in narrow cases (no dependents, very low downloads), so the publish is a
human decision run locally from a clean checkout of the tag, never from CI. To
publish the version behind a tag:

```sh
cargo login                 # one-time: token from https://crates.io/me (scope: publish)
git checkout vX.Y.Z
cargo make ci               # same proof CI ran on the tag
cargo publish --locked      # uploads the exact tagged tree
```

If the workflow's release step fails after a manual publish (or vice versa),
either half can be redone independently: re-run the workflow with
`gh workflow run release.yml -f tag=vX.Y.Z` (idempotent for a missing
release) or create the release manually with `gh release create`, and the
publish is idempotent until the version is taken.

## Versioning policy

Cocogitto derives the bump from the Conventional Commit vocabulary. Note
that the type must be the _only_ thing that matters: `fix:` is a patch,
`feat:` is a minor, and a breaking change is a major. A `perf:` commit
does **not** bump anything, so a release made only of `perf:` commits
produces no tag.

| Commits since the last tag                  | Bump                       |
| ------------------------------------------- | -------------------------- |
| `fix:` only                                 | patch (`0.1.0` -> `0.1.1`) |
| `feat:` present                             | minor (`0.1.0` -> `0.2.0`) |
| Breaking change (`!` or `BREAKING CHANGE:`) | major (`0.1.0` -> `1.0.0`) |

The bump NEVER auto-bumps a 0.y.z crate to 1.0.0; going 1.0.0 is a
deliberate act (the `version` input of the workflow, or a forced
`cog bump --major`).

## First-release notes

An earlier `v0.1.0` was cut prematurely, never finished, and its
publication has been deleted; its tag pointed 37 commits behind `master`
and has been removed from both the local clone and origin. crates.io
therefore has **no** `colander` version at all, and the GitHub Release
`v0.1.0` is gone with it.

The next bump is the real first release. With no tag in the repository,
cocogitto treats the whole history as unreleased and produces the initial
version `0.1.0`, so the changelog covers every commit since the initial
commit. `from_latest_tag = true` makes every version after that derive
from the commits accumulated since the last tag, which is what the
automation relies on. Creating a GitHub Release (on demand) and publishing
each new version to crates.io remain separate manual decisions; crates.io
discourages deleting a published version and permits it only in narrow
cases, so treat every publish as lasting.

## Recovery

- A pre-bump hook failure aborts the bump and cocogitto stashes the partial
  changes under `cog_bump_<version>`; `git stash apply` restores them. In CI
  the abort happens inside the runner, so `master` is untouched: fix the
  cause and re-run the workflow (or run `cargo make bump` locally).
- Post-bump hooks have no rollback, so they only push. In CI a failed push
  leaves the commit and tag in the runner only, where they vanish with the
  job: re-run the workflow, or run `cargo make bump` locally. Locally, a
  failed push leaves the local commit and tag intact and recoverable.
- If the workflow's Release step fails, create the release manually with
  `gh release create` — the step is idempotent for a missing release.
- A failed manual `cargo publish` leaves no half-state: fix the cause and
  re-run; the only permanent case is a version that was already published,
  which is why `cargo make ci` runs before every publish attempt.
