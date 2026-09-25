## What this does

<!-- One short paragraph. Link the issue this closes. -->

Closes #

## Changes

<!-- The decisions a reviewer cannot infer from the diff. -->

-

## Layers touched

- [ ] Rust core (`src/`)
- [ ] C ABI (`src/ffi/` and `include/colander.h`)
- [ ] Documentation (`docs/`, `README.md`)
- [ ] Tests (`tests/`)

## Contract

- [ ] The C ABI surface is unchanged.
- [ ] `include/colander.h` was regenerated from `cbindgen.toml`, if the ABI surface did change.
- [ ] No file under `tests/golden/vectors/` was modified. Those fixtures are frozen and cannot be regenerated, so a failing vector means behaviour moved, not that the fixture is stale.

## What was run

Paste each command and what it printed.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
```

## Checklist

- [ ] The output is pasted above, not summarised as "tests pass".
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0.
- [ ] Documentation was updated alongside any behaviour or contract change.
