# colander

A portable form runtime in Rust, exposed as a C ABI shared library.

colander owns the deterministic half of forms: schema compilation, the rule
engine, response validation and a JSON Schema subset validator. It does no I/O,
holds no state, has no async runtime and ships no schema of its own. Documents,
and the JSON Schemas that describe them, belong to the caller's domain and travel
with every request.

The crate declares no path or development dependency, so a clean clone builds and
tests with nothing but crates.io.

## Build

```bash
cargo build --release   # target/release/libcolander.so
cargo test              # unit tests plus the frozen golden vectors
```

The release profile uses `lto = true` and `strip = true`. The library exports these symbols:

```
colander_compile              colander_describe_form       colander_evaluate_rules
colander_validate_response    colander_validate_schema     colander_content_hash
colander_next_version         colander_version_info        colander_abi_version
colander_free_string          colander_alloc               colander_free_buffer
```

`colander_alloc` and `colander_free_buffer` exist for callers that cannot
allocate inside the library's heap. A caller with its own allocator can pass its
own buffer and ignore them.

## Tasks

`Makefile.toml` defines every command; run it through `cargo make`. These tools
are prerequisites and ship prebuilt binaries:

| Tool                                      | Version | Releases                                             |
| ----------------------------------------- | ------- | ---------------------------------------------------- |
| `cargo-make`, which provides `cargo make` | 0.37.24 | <https://github.com/sagiegurari/cargo-make/releases> |
| `hk`, which provides `hk`                 | 2.0.1   | <https://github.com/jdx/hk/releases>                 |
| `dprint`, which provides `dprint`         | 0.57.4  | <https://github.com/dprint/dprint/releases>          |

Everyday tasks:

| Task                   | What it does                                                                 |
| ---------------------- | ---------------------------------------------------------------------------- |
| `cargo make fmt`       | Formats Rust with rustfmt and Markdown with dprint.                          |
| `cargo make fmt-check` | Checks both languages and writes nothing.                                    |
| `cargo make lint`      | Runs clippy across all targets with warnings denied.                         |
| `cargo make check`     | Type-checks all targets.                                                     |
| `cargo make test`      | Runs the unit tests and the frozen golden-vector suite.                      |
| `cargo make build`     | Builds the release library.                                                  |
| `cargo make ci`        | The full set: `fmt-check`, `lint`, `check`, `test`, `build`, `header-check`. |
| `cargo make precommit` | `fmt-check`, `lint` and `header-check`.                                      |
| `cargo make prepush`   | `test`.                                                                      |
| `cargo make clean`     | Removes `target/`.                                                           |

Run `cargo make` with no task to list every task, including the granular
`rust-fmt`, `md-fmt` and check-only variants.

## Continuous integration and hooks

`.github/workflows/ci.yml` runs `cargo make ci` on pushes to `master` or `main`
and on every pull request. It checks out this repository only, uses GitHub's own
actions and the preinstalled toolchain, and fetches `dprint` and `cbindgen` from
their releases for the header check. Local and CI runs share the same task
definitions, so a change that passes locally is expected to pass there.

`hk.pkl` configures two hooks after `hk install`; every step is a `cargo make`
task:

- `pre-commit` checks changed `*.rs` files with `rust-fmt-check` and `lint`, and
  changed `*.md` files with `md-check`, so a Markdown-only commit is still
  checked.
- `pre-push` runs `test` and is skipped when no `*.rs` file changed.

The configured steps are check-only, so a failure reports and blocks instead of
rewriting files. Set `HK=0` to bypass the hooks for one command.

## Documentation

`docs/README.md` indexes the pages by task and gives three reading paths. The
pages are [getting-started](docs/getting-started.md),
[concepts](docs/concepts.md), [glossary](docs/glossary.md), [abi](docs/abi.md),
[entry-points](docs/entry-points.md),
[entry-points-payload](docs/entry-points-payload.md),
[documents](docs/documents.md), [ide-schemas](docs/ide-schemas.md),
[rules](docs/rules.md), [validation](docs/validation.md),
[json-schema](docs/json-schema.md), [json](docs/json.md),
[codecs](docs/codecs.md), [gotchas](docs/gotchas.md),
[dependencies](docs/dependencies.md), [vectors](docs/vectors.md),
[releases](docs/releases.md) and [conformance](docs/conformance.md).

Start with [SPEC.md](SPEC.md) for the contract of record, or
[docs/getting-started.md](docs/getting-started.md) for the ten-minute path to a
first call.

## The ABI

Every entry point takes one NUL-terminated UTF-8 JSON request string (or `NULL`)
and returns a NUL-terminated UTF-8 JSON envelope, which the caller releases with
`colander_free_string`. Never pass the result to `free()`, and never free it
twice.

```json
{"ok":true,"result":{…}}
{"ok":false,"error":{"kind":"invalid_request|validation|panic","message":"…"}}
```

`invalid_request` means the envelope itself was unusable: a null pointer,
non-UTF-8 input, or JSON that is not an object. `validation` means the payload was
rejected. `panic` means a bug in colander, caught at the boundary and returned as
that envelope. No `char *`-returning entry point returns `NULL`.

```c
#include "colander.h"

char *response = colander_compile("{\"formSchemaJson\":\"{\\\"fields\\\":[]}\"}");
puts(response);
colander_free_string(response);
```

The full contract, including ownership and panic behaviour, is in
[docs/abi.md](docs/abi.md).

## Header

`include/colander.h` is generated by cbindgen from the crate and committed. The
header declares ten of the twelve exports: `colander_alloc` and
`colander_free_buffer` are deliberately excluded ([why](docs/abi.md#the-generated-header-does-not-declare-every-export)).
Regenerate it only when the ABI surface changes:

```bash
cbindgen --config cbindgen.toml --crate colander --output include/colander.h
```

## Layout

The tree is flat: one file per domain, and inside a domain one file per concern.

```
src/
  json.rs      JSON DOM, parser and writer
  rules.rs     rule analyzer and engine
  validate.rs  response validation and page scope
  schema.rs    JSON Schema subset
  compile.rs   triple compilation
  ffi.rs       the C ABI
  describe.rs  the field index
  semver.rs  hash.rs  index.rs  keys.rs  error.rs  codec.rs  semantic.rs
  pattern.rs  lib.rs
tests/         one file per domain, plus the golden-vector suite
include/       the generated header
schemas/       base JSON Schema templates for authoring
docs/          the documentation indexed by docs/README.md
```

Tests never sit inside a source file. Apart from `src/json/parse.rs`, which is
deliberately above the limit at 546 lines, no source file exceeds 400 lines.

The single deliberate trait is the `Codec` seam in `src/codec.rs`, which the
envelope boundary is generic over so the wire format is injectable. There is no
plugin layer; a second use is what would justify extracting anything else.

## Testing

`tests/` replays 254 frozen golden vectors across eight groups, making 1524
checks through the C ABI. Payloads are compared canonically, with object keys
sorted before comparison, and error cases by their `SCREAMING_SNAKE` code only;
message wording is colander's own and is not part of the contract. Five entries
are not replayed, each with its reason in `tests/common/mod.rs::exclusions`.

The vectors were recorded once from an external implementation and cannot be
regenerated. A failing vector means colander's behavior moved, not that the
fixture is stale. A decided contract change is the only exception: update the
affected expectation in the same commit, name the decision it implements, and
leave every other case untouched. See [docs/vectors.md](docs/vectors.md).
