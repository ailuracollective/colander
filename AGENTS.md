# AGENTS.md

A domain-neutral **form core** in Rust behind a **C ABI**. The ABI, the error
codes and the canonical serialization are a **published contract**: changing any
of them is a breaking change, not a refactor.

## Hard invariants

- **No path dependency, no dev-dependency.** Cargo resolves one graph covering
  dev-dependencies, so a path dependency anywhere breaks every cargo command, not
  just `cargo test`. The crate builds and tests from a clean clone with nothing
  but crates.io.
- **The golden vectors are frozen against accidental drift.**
  `tests/golden/vectors/*.json` were recorded once from an external
  implementation and cannot be regenerated, so a vector that fails on its own
  means colander's behavior moved, not that the fixture is stale. A **decided**
  contract change is the one exception: move the affected expectation in the same
  commit, name the decision it implements (the `SPEC.md` clause or the triage
  entry) in the commit message, and leave every other vector untouched. No
  automatic guard covers them, so the affected cases are enumerated by hand.
- **`include/colander.h` is generated but committed.** Never hand-edit it, and
  regenerate it only when the ABI surface actually changes. It declares ten of
  the twelve exports on purpose
  ([why](docs/abi.md#the-generated-header-does-not-declare-every-export)).
- **Three direct runtime dependencies**: `fancy-regex`, `indexmap`, `sha2`,
  sixteen transitive crates in all. Adding one is a decision to raise, not a
  convenience.
- **No source file over 400 lines.** `src/json/parse.rs` is the only exception,
  at 546; do not widen the gap.
- **English** for every artifact — code, docs, comments, commits.
- **No commits, branches or remotes unless explicitly asked.** `origin` exists
  and `0.4.1` is the current published version;
  [docs/releases.md](docs/releases.md) is the authority on release state, not this
  file.

## Commands

`Makefile.toml` is the single source of truth for command lines: prefer
`cargo make <task>` and never restate a cargo command in another file.

- Build: `cargo build --release` → `target/release/libcolander.so`.
- Test: `cargo test`. One vector group: `cargo test --test golden_rules -- --nocapture`.
- Everything CI runs: `cargo make ci` (`fmt-check`, `lint`, `check`, `test`,
  `build`, `header-check`). The fast pre-commit set is `cargo make precommit`.
- Format: `cargo make fmt` (rustfmt + dprint), verified by `cargo make fmt-check`.
  Rust formatting is default rustfmt with no `rustfmt.toml`, so do not run a
  drive-by `cargo fmt` across the tree; Markdown options live in `dprint.json`.
- Regenerate the header: `cbindgen --config cbindgen.toml --crate colander
  --output include/colander.h`, then `cargo make header-check`.

## Conventions

- Flat modules: one file per domain, one submodule per concern (`src/json.rs` →
  `src/json/{parse,write}.rs`). No plugin layer; the single deliberate trait is
  the `Codec` seam in `src/codec.rs`, which the envelope boundary is generic over.
  A second use is what would justify extracting anything else.
- The JSON parser and writer are handwritten on purpose, to keep number spelling
  and key order intact. `fancy-regex` follows ECMA-262 for `pattern` as far as
  the engine supports (SPEC S-6).
- Tests live in `tests/`, one file per domain. `[workspace]` is declared with no
  members, so this crate is its own package.
- A behavior or contract change updates `docs/` in the same change.

## Where to look

- [docs/README.md](docs/README.md) indexes the documentation by task;
  [docs/gotchas.md](docs/gotchas.md) collects the surprising behavior.
- [CONTRIBUTING.md](CONTRIBUTING.md) holds the pull-request gates: linked issue,
  branch name, Conventional Commit title, one type label, template sections.
- [docs/releases.md](docs/releases.md) holds the release flow, Trusted Publishing
  and recovery.
- `odd/tasks/` and `.atl/skill-registry.md` are git-ignored project artifacts:
  read the relevant task file when a checkout carries one.
