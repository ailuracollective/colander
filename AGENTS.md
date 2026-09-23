# AGENTS.md

A domain-neutral **form core** in Rust behind a **C ABI**. The ABI, the error
codes and the canonical serialization are a **published contract**: changing any
of them is a breaking change, not a refactor.

## Read this before running anything

**`cargo build` and `cargo test` run from a clean clone with nothing but
crates.io.** This crate declares no path dependency and needs no sibling
checkout. It once did — a dev-dependency on `../slate-ai` broke _every_ cargo
command, not just `cargo test`, because cargo resolves one graph covering
dev-dependencies. Do not reintroduce one.

**The dependency edge points one way.** The separate `slate-ai` repository
depends on this crate; the reverse is a defect, not a convenience. Adding any
edge from here onto `slate-ai` is what caused the outage above.

**The repository has no commits and no remote.** `master` is unborn, so `git log`,
`git show` and `git diff HEAD` all fail. Do not create commits, branches or
remotes unless explicitly asked.

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
| Preview the next release version | `cog bump --dry-run --auto`                                                    |
| Create a release                 | `cog bump --auto` (see [Releases](#releases))                                  |

**`.github/workflows/ci.yml` runs `cargo make ci`.** The
crate is self-contained, so the workflow checks out one repository and needs no
sibling `slate-ai` checkout. It uses only GitHub's own actions and the
toolchain preinstalled on the runner, plus the `hk` and `dprint` binaries
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

- **`tests/golden/vectors/*.json` are frozen. Never edit one to make a test
  pass.** They were recorded once from an external implementation and cannot be
  regenerated. A failing vector means colander's behavior moved, not that the
  fixture is stale.
- Seven groups: `canonical`, `hash`, `semver`, `rules`, `validate`, `compile`,
  `errors`. The shared harness is `tests/common/mod.rs`. The eighth group, `ai`,
  lives in the separate `slate-ai` repository with its own harness subset.
- `tests/common/mod.rs::exclusions` has entries for `rules` and `semver` only.
  Any other group hits `panic!("no exclusions table for group '…'")`.
- **`include/colander.h` is generated but committed.** Never hand-edit it;
  regenerate it only when the ABI surface actually changes.
- The generated header declares only **nine** functions while the library exports
  **eleven**: `colander_alloc` and `colander_free_buffer` are deliberately absent,
  listed in `cbindgen.toml`'s `[export] exclude`, and a C caller declares them
  itself. `[export] include` only names the nine to force; it does not restrict
  the export set, which is why the `exclude` list is what keeps the pair out. Do
  not "fix" that by hand.

## Conventions that differ from the defaults

- Tests live in `tests/`, one file per domain.
- The stated limit is **no file over 400 lines**; `src/json/parse.rs` is already
  at 541. Do not widen that gap.
- Deliberately **no trait, no generic abstraction, no plugin layer**. A second
  use is what would justify extracting anything.
- Exactly **three runtime dependencies**: `indexmap`, `regex`, `sha2`. The JSON
  parser and writer are handwritten on purpose, to keep number spelling and key
  order intact. Adding a dependency is a decision to raise, not a convenience.
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

Releases are created locally on `master` with a clean tree: preview with
`cog bump --dry-run --auto`, then `cog bump --auto`. The pre-bump hooks
guard the generated header (`scripts/check-header.sh`) and bump the single
version source (`scripts/bump-version.sh`, Cargo.toml -> CARGO_PKG_VERSION);
the post-bump hooks push `master` and the `v*` tag. That tag push triggers
`.github/workflows/release.yml`, which verifies via `cargo make ci`, builds
the cdylib, publishes to crates.io (secret `CARGO_REGISTRY_TOKEN`), and
creates the GitHub Release with changelog notes and the artifact. Version
policy: `fix` -> patch, `feat` -> minor, breaking -> major;
`cog bump --auto` never auto-bumps 0.y.z to 1.0.0. See
[docs/releases.md](docs/releases.md) for the full flow.

## Where things are

`docs/README.md` indexes the documentation; `docs/gotchas.md` collects the
surprising behaviors. `odd/tasks/` records in-flight and completed work — read
the relevant task file before changing an area it covers. `odd/` and
`.atl/skill-registry.md` are project artifacts; the rest of `.atl/` is a local
tooling cache.
