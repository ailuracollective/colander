# AGENTS.md

A domain-neutral **form core** in Rust behind a **C ABI**. The ABI, the error
codes and the canonical serialization are a **published contract**: changing any
of them is a breaking change, not a refactor.

## Read this before running anything

**`cargo build` and `cargo test` run from a clean clone with nothing but
crates.io.** This crate declares no path dependency and no dev-dependency. Keep
it that way: Cargo resolves one graph covering dev-dependencies, so any path
dependency would break every cargo command, not just `cargo test`.

**The repository has an `origin` remote and commits.** It currently has
**no tags at all**, and **no `colander` version is published on crates.io** —
an earlier `v0.1.0` was cut prematurely and its publication was deleted, so
`docs/releases.md` is the authority on release state, not this file. Do not
create commits, branches or remotes unless explicitly asked.

## Commands

| Purpose                          | Command                                                                        |
| -------------------------------- | ------------------------------------------------------------------------------ |
| Build native                     | `cargo build --release` → `target/release/libcolander.so`                      |
| Test                             | `cargo test`                                                                   |
| Lint                             | `cargo clippy --all-targets -- -D warnings` (or `cargo make lint`)             |
| Format Rust                      | `cargo make rust-fmt` (rustfmt)                                                |
| Format Markdown                  | `cargo make md-fmt` (dprint)                                                   |
| Full local CI set                | `cargo make ci` — `fmt-check`, `lint`, `check`, `test`, `build`                |
| List every task                  | `cargo make --list-all-steps`                                                  |
| One vector group                 | `cargo test --test golden_rules -- --nocapture`                                |
| Regenerate the header            | `cbindgen --config cbindgen.toml --crate colander --output include/colander.h` |
| Preview the next release version | `cargo make bump-dry-run`                                                      |
| Create a release                 | `cargo make bump` (see [Releases](#releases))                                  |

**`.github/workflows/ci.yml` runs `cargo make ci`.** The
crate is self-contained, so the workflow checks out only this repository. It uses
only GitHub's own actions and the toolchain preinstalled on the runner, plus the
`hk` and `dprint` binaries
fetched from their GitHub releases for the `conventional-commits` and `ci` jobs
(no third-party actions).
`Makefile.toml` is the single source of truth for command lines; `hk.pkl` wires
those same tasks into the git hooks. Prefer `cargo make <task>` over restating a
cargo command, and never duplicate a command line into a file. Formatting is
`rustfmt` for Rust and `dprint` for Markdown: `cargo make fmt` and
`cargo make fmt-check` cover both, while the granular `rust-fmt`/`md-fmt` and
`rust-fmt-check`/`md-check` tasks stay available. There is no `rustfmt.toml` and
no clippy config, so Rust formatting is default runtime — do not run a drive-by
`cargo fmt` across the tree. Markdown options live in `dprint.json`; `dprint` is
not part of the Rust toolchain, so install its prebuilt binary into
`~/.cargo/bin` alongside `cargo-make` and `hk`.

`cbindgen` **0.29.4 is installed** (via `cargo install cbindgen --locked`), so the
header-regeneration command above works. Do not work around a header mismatch by
hand-editing the header.

## The frozen contract

- **`tests/golden/vectors/*.json` are frozen against accidental drift.** They
  were recorded once from an external implementation and cannot be regenerated, so
  a vector that fails on its own means colander's behavior moved, not that the
  fixture is stale. Never edit one to make an unintended failure pass. A
  **decided** contract change is the one exception: move the affected expectation
  in the same commit, name the decision it implements (the `SPEC.md` clause or the
  triage entry) in the commit message, and leave every other vector untouched.
  Payload cases are compared byte-for-byte, and an error case is compared by its
  `SCREAMING_SNAKE` code only, so a decided change usually moves one code and
  nothing else. No automatic guard covers the vectors, so the affected cases are
  enumerated by hand before the change lands.
- Eight groups: `canonical`, `hash`, `semver`, `rules`, `validate`, `compile`,
  `errors`, `describe`. The shared harness is `tests/common/mod.rs`; all eight
  groups are part of this repository. Seven were recorded from an external
  implementation; `describe` covers an operation with no external counterpart, so
  it was recorded from colander and is frozen the same way.
- `tests/common/mod.rs::exclusions` has entries for `rules` and `semver` only.
  Any other group hits `panic!("no exclusions table for group '…'")`.
- **`include/colander.h` is generated but committed.** Never hand-edit it;
  regenerate it only when the ABI surface actually changes.
- The generated header declares only **ten** functions while the library exports
  **twelve**: `colander_alloc` and `colander_free_buffer` are deliberately absent,
  listed in `cbindgen.toml`'s `[export] exclude`, and a C caller declares them
  itself. `[export] include` only names the ten to force; it does not restrict
  the export set, which is why the `exclude` list is what keeps the pair out. Do
  not "fix" that by hand.

## Conventions that differ from the defaults

- Tests live in `tests/`, one file per domain.
- The stated limit is **no file over 400 lines**; `src/json/parse.rs` is already
  at 540 and `docs/entry-points.md` at 510. Do not widen either gap.
- Deliberately flat and free of a plugin layer; the single deliberate trait is
  the `Codec` seam in `src/codec.rs`, which the envelope boundary is generic
  over so the wire format is injectable. A second use is what would justify
  extracting anything else.
- Exactly **three direct runtime dependencies**: `fancy-regex`, `indexmap`,
  `sha2`, which resolve to sixteen transitive crates (nineteen in all).
  `fancy-regex` follows ECMA-262 for `pattern` as far as the engine supports
  (SPEC S-6). The JSON parser and writer are handwritten on purpose, to keep
  number spelling and key order intact. Adding a dependency is a decision to
  raise, not a convenience.
- Flat modules: one file per domain, one submodule per concern
  (`src/json.rs` → `src/json/{parse,write}.rs`).
- `[workspace]` is declared but has no members, so this crate is its own package
  and nothing is fanned out across a workspace.
- Every artifact — code, docs, comments, commits — is **English**.

## Contributing

- Issue titles and labels: `fix: `/`fix`, `feat: `/`feat`, `test: `/`test`,
  `chore: `/`chore`. `config.yml` sets `blank_issues_enabled: false`, so issues
  must use a template.
- PRs link an issue (`Closes #`) and must **paste real command output**, never
  "tests pass".
- Review budget is **~400 changed lines**.
- A behavior or contract change updates `docs/` in the same change.

### Commit messages and branches

- Commit messages follow Conventional Commits — `type(scope): description` —
  with the type restricted to the closed vocabulary `feat`, `fix`, `docs`,
  `chore`, `refactor`, `test`, `ci`, `build`, `perf` (single source of truth:
  `scripts/conventional-types.txt`).
- The header must be **at most 72 characters** and a scope, when present, must
  be **lower-case**.
- Branches follow `<username>/<type>/<short-description>`, with the username
  matching the pull request author.
- Enforced locally by the `commit-msg` hook (`hk.pkl` →
  `scripts/check-conventional-commit.sh`) and, on pull requests, by the
  `conventional-commits` CI job (branch name and PR title; the squash-merge
  subject is the title, so one check covers both).

## Releases

Every push to `master` runs the `Bump` workflow
(`.github/workflows/bump.yml`), which bumps without asking: the pre-bump
hooks guard the generated header (`scripts/check-header.sh`) and bump the
single version source (`scripts/bump-version.sh`, Cargo.toml ->
CARGO_PKG_VERSION), then cocogitto writes the changelog and commits
`chore(version): vX.Y.Z` and pushes it to `master` with the
**`BUMP_TOKEN`** repository secret. `master` is protected (pull request, one
approving review, two required checks) and the `GITHUB_TOKEN` is not an
admin, so it cannot push; `enforce_admins` is `false`, so a token owned by
an admin can. That push is the release commit, and the workflow's `tag` job
then creates `vX.Y.Z` on the tip. The tag job does not listen for a
pull request being merged, because events caused by `GITHUB_TOKEN` do not
start workflow runs and `pull_request` with the `closed` activity is not one
of the exceptions. An earlier version landed the bump as a pull request; it
was abandoned because `scripts/check-branch-name.sh` requires the branch's
username segment to match the pull request author, and no branch a workflow
creates can satisfy that. `cargo make bump` is the identical local path (its
post-bump hooks push `master` directly, so it needs an account that can
bypass the protection), and `cargo make bump-dry-run` previews it. A range
whose commits are only
`chore`/`docs`/`refactor`/`test`/`ci`/`build`/`perf` produces no pull
request and a green run: cocogitto is the only source of truth for what
deserves a bump. The tag push runs the full CI verification only
(`ci.yml`); it NEVER creates a GitHub Release. To create the Release, run
the `Release` workflow yourself (Actions UI or
`gh workflow run release.yml -f tag=vX.Y.Z`) — it verifies again, builds the
cdylib, and creates the GitHub Release with changelog notes and the
artifact. Publishing to crates.io is a manual step for now (see
[docs/releases.md](docs/releases.md)). Version policy: `fix` -> patch,
`feat` -> minor, breaking -> major (a `perf:` commit does NOT bump); the
bump never auto-bumps 0.y.z to 1.0.0. See [docs/releases.md](docs/releases.md)
for the full flow.

## Where things are

`docs/README.md` indexes the documentation; `docs/gotchas.md` collects the
surprising behaviors. `odd/tasks/` records in-flight and completed work — read
the relevant task file before changing an area it covers. `odd/` and
`.atl/skill-registry.md` are project artifacts; the rest of `.atl/` is a local
tooling cache.
