# Releases

Releasing colander has exactly **one** human decision in it: merging the
release pull request. Everything before that merge is computed from commits CI
has already verified; everything after it is driven by the GitHub Release the
merge produces.

## The flow

1. A commit lands on `master`. `.github/workflows/release-please.yml` runs.
2. release-please reads the Conventional Commits since the last release,
   computes the next version, and opens (or updates) a **release pull
   request** that bumps `Cargo.toml` (`package.version`) and `Cargo.lock` and
   adds the new `CHANGELOG.md` section.
3. **A maintainer reviews and merges it. This is the release gate.** Nothing
   exists outside the repository until this happens.
4. release-please creates the `vX.Y.Z` tag **and** the GitHub Release from the
   merged commit, in the same transaction.
5. `.github/workflows/publish.yml` triggers on `release: published`: it checks
   out the tag, checks it against the `[package] version` in `Cargo.toml`,
   builds `target/release/libcolander.so`, publishes to crates.io through
   Trusted Publishing with provenance, and attaches the cdylib to the Release.

| Step          | What happens                                                                                                       |
| ------------- | ------------------------------------------------------------------------------------------------------------------ |
| Planning      | `release-please.yml` on every push to `master`; `release-type: rust` derives the version from commits              |
| Release PR    | Bumps `Cargo.toml` and `Cargo.lock`, writes the `CHANGELOG.md` section, title `chore(master): release 0.2.0`       |
| **Merge**     | **The human gate.** Branch protection, the approving review, and the `policy.yml` gates all apply                  |
| Tag + Release | release-please writes `vX.Y.Z` and creates the GitHub Release from the merged commit                               |
| `publish.yml` | Verifies the tag against `Cargo.toml`, builds the cdylib, publishes to crates.io, attaches the artifact            |
| `ci.yml`      | A release merge skips the heavy `ci` job on the `push` leg only; the `pull_request` leg already gated that content |

A range whose commits are only `chore`, `docs`, `refactor`, `test`, `ci`,
`build` or `perf` produces no release pull request and a green run. There is
no guard step predicting bump-worthiness: release-please is the only source of
truth for it, and a no-op run is a pass.

## What changed and why

The previous pipeline used cocogitto and kept the **tag** and the **GitHub
Release** as two separate steps on purpose (recorded decision 2026-09-23:
_"debería ser tag y solo release cuando yo dice"_). release-please cannot
separate them — creating the tag creates the Release, one step in the tool —
so the two-step decision collapses into **one** human action: merging the
release pull request.

Say plainly what is lost and what is not. Nothing reaches the world without a
maintainer action either way; it just moved from "run the Release workflow" to
"merge the release PR". What is lost is the ability to **tag now and release
later**. Everything else survives.

The second change is why the move was worth it. cocogitto had to push
**straight to `master`** with the `BUMP_TOKEN` admin secret, because the branch
gate requires `<username>/<type>/<short-description>` whose first segment matches
the pull request author, and Git forbids `[` in a ref — so **no branch a workflow
creates can satisfy it**. That gate is the `branch-validation` job in
`.github/workflows/policy.yml` today. release-please opens a pull request a human
opens, reviews and merges, so the gate is satisfiable, branch protection stays
on, and no admin credential exists anywhere in the pipeline. The gate exempts
the account `RELEASE_PLEASE_TOKEN` belongs to — `AiluraKitty` — for exactly the
reason above: its pull requests have a head ref of
`release-please--branches--master--components--colander`, which has no type
segment and cannot pass by construction. The exemption is about the bot's naming,
not about trust, and nothing a person wrote is exempted by it. The reasoning is
recorded in full in the header of `.github/workflows/release-please.yml`.

## Configuration

Two JSON files at the repository root.

- `.release-please-manifest.json` seeds the last released version as
  `"." : "0.1.0"`, matching the existing `v0.1.0` tag and `Cargo.toml`.
  Without it, the first release pull request would plan from a baseline that
  does not exist.
- `release-please-config.json` carries `release-type: rust`,
  `bump-minor-pre-major: true`, the title pattern, and **one** entry in
  `packages`. There is no `separate-pull-requests`, no
  `include-component-in-tag`, no `tag-separator` and no `sequential-calls`:
  those are monorepo options and this is a single crate, so the tag shape
  stays the default `vX.Y.Z`.

That `packages` entry is not optional, and its absence **fails silently**:

```json
"packages": { ".": {} }
```

`parseConfig` in release-please's `src/manifest.ts` builds the entire release
plan with `for (const path in config.packages)`. Without it the loop runs
zero times, `repositoryConfig` stays `{}`, and every consumer downstream
iterates that empty map. The run then plans **zero releases and exits green**:
a pipeline that looks healthy and publishes nothing, forever. The root-level
`release-type` and `bump-minor-pre-major` are only defaults merged into paths
that exist, so they would be dead too. The published JSON schema agrees —
`packages` is its single required property. The key is `"."` because that is
`ROOT_PROJECT_PATH`, and it must match the manifest key exactly.

`CHANGELOG.md` does not exist yet; the Rust strategy creates it on the first
release pull request. `dprint.json` already excludes it, because dprint
rejects the generated layout and the `md-check` hook would then reject every
release merge.

## Versioning policy

The type must be the _only_ thing that matters: `fix:` is a patch, `feat:` is a
minor, a breaking change is a major.

| Commits since the last release                                      | Bump                       |
| ------------------------------------------------------------------- | -------------------------- |
| `fix:` only                                                         | patch (`0.1.0` -> `0.1.1`) |
| `feat:` present                                                     | minor (`0.1.0` -> `0.2.0`) |
| Breaking change (`!` or `BREAKING CHANGE:`)                         | major (`0.1.0` -> `1.0.0`) |
| `perf:` only, or only `chore`/`docs`/`refactor`/`test`/`ci`/`build` | no release                 |

The bump **never** auto-bumps a 0.y.z crate to 1.0.0. That is what
`bump-minor-pre-major: true` guarantees, and it is why a `feat` here produces
`0.2.0`. Going 1.0.0 remains a deliberate act.

The release pull request title is `chore(master): release <version>`, which
satisfies `pull-request-policy`'s `enable-title-conventional` check: `chore` is
one of the nine allowed types, `master` is a lower-case scope and the header is
well under `title-max: 72`. That check therefore still runs in substance on
release-please pull requests — but it is not what exempts them. The
`AiluraKitty` entry in `skip-actors` exempts the whole `pull-request-policy`
job for that account, because the action has no per-check escape hatch and a
release pull request cannot satisfy the other checks either: it links no issue,
its only label is release-please's own `autorelease: pending`, and its body is a
generated changelog rather than any type's template. The action reports an
exempt pull request as skipped, not passed. A human still reviews and merges
it.

## Trusted Publishing setup (one-time, maintainer action)

There is no long-lived crates.io credential in this repository and no
`cargo login`: `publish.yml` exchanges an OIDC token for a short-lived
registry token. This needs a **one-time registration per crate**, and until it
exists the publish step fails:

1. <https://crates.io/settings> → Trusted Publishing → **Add**.
2. For `colander`: owner `ailuracollective`, repository `colander`,
   **workflow filename `publish.yml`**.

The filename must match exactly. `colander` 0.1.0 is already published, so
the "publish once manually first" prerequisite is satisfied.

Also required: the `RELEASE_PLEASE_TOKEN` repository secret, or
`release-please.yml` fails and no release pull request is ever opened. It is
about **authorship**, not permissions — release-please creates the commit
through the GitHub API and attributes it to the calling token, so with
`GITHUB_TOKEN` every release commit is `github-actions[bot]`.

## The header guard

`include/colander.h` is generated but committed, and the C ABI is frozen, so a
release must not ship a header that disagrees with the sources.
`scripts/check-header.sh` regenerates it with cbindgen into a temporary file,
compares byte-for-byte and fails on any difference. It is a step of the `ci`
job through `cargo make header-check`, a dependency of both `cargo make ci` and
`cargo make precommit`.

The header is never patched in place. To regenerate it after an ABI change:

    cbindgen --config cbindgen.toml --crate colander --output include/colander.h

and commit the result in the same change that changed the ABI.

## Recovery

- **No release pull request after a push to `master`.** Either the range does
  not deserve a bump (only `chore`/`docs`/`refactor`/`test`/`ci`/`build`/
  `perf` — a correct no-op), or the workflow failed, most often because
  `RELEASE_PLEASE_TOKEN` is missing or expired. The run log says which.
- **The release pull request is wrong.** Close it, fix the underlying commit
  messages, and let the next push re-plan. release-please updates an existing
  pull request rather than opening a second one, so closing it without fixing
  anything just gets you the same plan. Nothing is tagged until the merge.
- **The publish job fails.** Re-run it from the Actions UI. Re-run is the first
  move: nothing local is involved, and no version was consumed unless the
  failure happened after `cargo publish` already succeeded. If it did reach
  crates.io, the version is taken and the only repair is a new patch release.
- **The Release has no artifact.** Re-run `publish.yml`. The upload step is
  guarded: it lists the release's existing assets and skips `libcolander.so`
  if it is already attached, so a re-run after a successful upload is a no-op
  rather than a failure.
- **The version and the tag disagree.** The check in `publish.yml` stops the
  run before anything is built or uploaded. The tag was moved or cut from a
  different commit than the version it claims: delete the tag and the Release,
  and release again through a fresh pull request.

A published crates.io version **cannot be deleted**, only yanked. Yanking
hides it from resolution and tells dependents to stop, but the number stays
taken forever. Treat every publish as lasting.

## Release state

`0.1.0` is the only `colander` version on crates.io: tag `v0.1.0` at
`cb99815`, and that exact tree was published. Two earlier tags never produced
a publication and are gone — a first `v0.1.0` that pointed 37 commits behind
`master` and was published then deleted, and a `v1.0.0` cut after that one was
abandoned, which pointed two commits behind and was missing
`colander_describe_form` (issue #26).

The first release pull request after the migration re-derives the changelog
from the commits since `v0.1.0`. The history was rebuilt, so older sections
are not recoverable and the first `CHANGELOG.md` will be short. That is
expected, not a bug.
