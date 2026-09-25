# colander

A portable **form runtime** in Rust, exposed as a C-ABI shared library.

colander owns the pure, deterministic half of forms: schema compilation, the
rule engine, response validation and a JSON-Schema subset validator. It does no
I/O, holds no state, has no async runtime and **ships no schema of its own** —
documents and the JSON Schemas that describe them belong to the caller's domain
and travel with each request.

The crate is self-contained: it declares no path or development dependency, and a
clean clone builds and tests with nothing but crates.io.

## Build

```bash
cargo build --release     # target/release/libcolander.so
cargo test                # unit tests + the golden-vector suite
```

The release profile uses `lto = true` and `strip = true`, and
`libcolander.so` exports these symbols:

```
colander_compile              colander_validate_schema     colander_content_hash
colander_evaluate_rules       colander_next_version        colander_version_info
colander_validate_response    colander_abi_version         colander_free_string
colander_alloc                colander_free_buffer
```

`colander_alloc` and `colander_free_buffer` exist for callers that cannot allocate
inside the library's heap. A caller with its own allocator can pass its own
buffer and ignore them.

## Tasks

`Makefile.toml` wraps the build, test and formatting commands as `cargo make`
tasks. These tools are prerequisites and ship prebuilt binaries:

| Tool                                      | Version | Release artifacts                                    |
| ----------------------------------------- | ------- | ---------------------------------------------------- |
| `cargo-make`, which provides `cargo make` | 0.37.24 | <https://github.com/sagiegurari/cargo-make/releases> |
| `hk`, which provides `hk`                 | 2.0.1   | <https://github.com/jdx/hk/releases>                 |
| `dprint`, which provides `dprint`         | 0.57.4  | <https://github.com/dprint/dprint/releases>          |

| Task                        | What it does                                                            |
| --------------------------- | ----------------------------------------------------------------------- |
| `cargo make fmt`            | Formats both languages: `rust-fmt` and `md-fmt`.                        |
| `cargo make fmt-check`      | Checks both languages: `rust-fmt-check` and `md-check`; writes nothing. |
| `cargo make rust-fmt`       | Formats the Rust tree with rustfmt.                                     |
| `cargo make rust-fmt-check` | Fails when the Rust tree is not formatted; writes nothing.              |
| `cargo make md-fmt`         | Formats the Markdown tree with dprint.                                  |
| `cargo make md-check`       | Fails when the Markdown tree is not formatted; writes nothing.          |
| `cargo make lint`           | Runs clippy across all targets, with warnings denied.                   |
| `cargo make check`          | Type-checks all targets.                                                |
| `cargo make test`           | Runs the unit tests and the frozen golden-vector suite.                 |
| `cargo make build`          | Builds the release library.                                             |
| `cargo make clean`          | Removes `target/`.                                                      |
| `cargo make ci`             | The full set: `fmt-check`, `lint`, `check`, `test`, `build`.            |
| `cargo make precommit`      | `fmt-check` then `lint`.                                                |
| `cargo make prepush`        | `test`.                                                                 |
| `cargo make`                | With no task name, lists every task.                                    |

Every task runs from a clean clone with nothing but crates.io.

## Continuous integration

`.github/workflows/ci.yml` runs `cargo make ci`, on pushes to `master` or `main`
and on every pull request.

It checks out this repository only, because the crate is self-contained and
declares no dev-dependency. It uses GitHub's own actions plus the toolchain
preinstalled on the runner — no third-party action is pulled in.

It runs the same tasks you run locally, so a change that passes `cargo make ci`
here is expected to pass there.

## Git hooks

`hk.pkl` configures two hooks, installed once with `hk install`. Every hook step
is a `cargo make` task; no cargo command line is repeated there.

- `pre-commit` runs `cargo make rust-fmt-check` and `cargo make lint` against
  changed `*.rs` files, and `cargo make md-check` against changed `*.md` files,
  so a Markdown-only commit is still checked.
- `pre-push` runs `cargo make test`, and is skipped when no `*.rs` file changed.

`hk`'s `pre-commit` is fix-capable: for a step that declares a fix command it
stashes unstaged work first, applies the fix, and stages the result, so the
working tree and the index can change during a commit. The steps configured
here are check-only, so the hook reports a problem and fails the commit instead
of rewriting files; adding a fix command to a step opts into the stash-and-stage
behaviour. Set `HK=0` to bypass the hooks for one command.

## Documentation

The explanatory documentation lives under `docs/`, split by task, and the
normative contract lives at the root:

| Document                                           | What it covers                                                               |
| -------------------------------------------------- | ---------------------------------------------------------------------------- |
| [SPEC.md](SPEC.md)                                 | The contract of record: every requirement and whether the crate meets it.    |
| [docs/README.md](docs/README.md)                   | Documentation index and reading paths.                                       |
| [docs/getting-started.md](docs/getting-started.md) | The ten-minute path to a first call.                                         |
| [docs/concepts.md](docs/concepts.md)               | The mental model: request/response, the three documents, `id` versus `code`. |
| [docs/entry-points.md](docs/entry-points.md)       | Full reference for every exported symbol.                                    |
| [docs/documents.md](docs/documents.md)             | The form, UI and rules schemas.                                              |
| [docs/ide-schemas.md](docs/ide-schemas.md)         | The `schemas/` JSON Schema templates for IDE autocomplete.                   |
| [docs/rules.md](docs/rules.md)                     | Rule operators, evaluation order and analysis errors.                        |
| [docs/validation.md](docs/validation.md)           | Response validation and every error code.                                    |
| [docs/abi.md](docs/abi.md)                         | The wire contract: envelope, memory, panics, escaping, header gaps.          |
| [docs/codecs.md](docs/codecs.md)                   | The codec seam: JSON and MessagePack, and why injection is static.           |
| [docs/gotchas.md](docs/gotchas.md)                 | Behaviour worth knowing.                                                     |
| [docs/vectors.md](docs/vectors.md)                 | What the frozen golden vectors assert.                                       |
| [docs/releases.md](docs/releases.md)               | The release pipeline, header guard and manual crates.io publish.             |
| [docs/conformance.md](docs/conformance.md)         | Cross-language parity: what a wrapper must preserve to stay conformant.      |

## The ABI in one paragraph

Every entry point takes **one NUL-terminated UTF-8 JSON request string** (or
`NULL`) and returns a **NUL-terminated UTF-8 JSON envelope** that the caller
releases with `colander_free_string`. Never pass the result to `free()`, and never
free it twice.

```json
{"ok": true,  "result": { ... }}
{"ok": false, "error": {"kind": "invalid_request|validation|panic", "message": "..."}}
```

`invalid_request` means the envelope itself was unusable (null pointer,
non-UTF-8, not a JSON object); `validation` means the payload was rejected;
`panic` means a bug in colander — caught at the boundary and returned as that
envelope. No `char *`-returning entry point returns `NULL`, and the intent is that
no panic unwinds into the caller. The header carries the shapes;
**[docs/getting-started.md](docs/getting-started.md)** gets you to a first call,
and **[docs/entry-points.md](docs/entry-points.md)** is the full reference —
every entry point, the three documents, every field type, every rule operator,
every error code, and the behaviours that surprise people.

```c
#include "colander.h"

char *response = colander_compile("{\"formSchemaJson\":\"{\\\"fields\\\":[]}\"}");
puts(response);
colander_free_string(response);
```

## Header

`include/colander.h` is generated by cbindgen from the crate and kept in sync:

```bash
cbindgen --config cbindgen.toml --crate colander --output include/colander.h
```

## Layout

Flat, one file per domain, and inside each domain one file per concern, so no
file has to be read whole to be understood:

```
src/
  json.rs        JSON DOM + accessors      json/{parse,write}.rs
  rules.rs       rule analyzer + engine    rules/{value,model,analyze,evaluate,expression,number}.rs
  validate.rs    response validation       validate/{model,fields,repeater,calculated,conversion,datetime,constraints}.rs
  schema.rs      JSON Schema subset        schema/{model,check,keywords}.rs
  compile.rs     triple compilation        compile/{context,schemas,fields,util}.rs
  ffi.rs         the C ABI                 ffi/{envelope,compile,rules,response,schema,session,version,memory}.rs
  semver.rs  hash.rs  index.rs  keys.rs  error.rs  lib.rs
tests/           one file per domain, plus the golden-vector suite
include/         the generated header — it declares 9 of the 11 exports;
                 colander_alloc and colander_free_buffer are exported by the
                 library but are not declared in it
docs/
  README.md               documentation index and reading paths
  getting-started.md      the 10-minute path
  concepts.md             the mental model
  glossary.md             plain-language terms
  abi.md                  the wire contract
  entry-points.md         all eleven exported symbols
  documents.md            form, UI and rules schemas
  rules.md                operators, evaluation order, analysis errors
  validation.md           response validation and error codes
  json-schema.md          supported Draft 2020-12 subset
  json.md                 parser, limits, number output, canonical form
  codecs.md               the codec seam
  gotchas.md              behaviour worth knowing
  dependencies.md         what stays hand-written, and why
  vectors.md              what the frozen vectors assert
  releases.md             the release pipeline
```

Tests never sit inside a source file: the crate keeps its tests under `tests/`,
and no file exceeds 400 lines. The crate is deliberately flat — one file per
domain, one file per concern. The single deliberate trait is the `Codec` seam in
`src/codec.rs`, which the envelope boundary is generic over so the wire format is
injectable; there is no plugin layer, and a second use is what would justify
extracting anything else.

## Testing

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

`tests/` replays 249 frozen golden vectors through the C ABI, making **1648
assertions**. Payloads are compared byte-for-byte and contract error codes must
match; the wording of an error message is colander's own and is not part of the
contract. Five entries are not replayed; `docs/vectors.md` lists the reasons. The
vectors are frozen: they were captured once from an external implementation and
the harness that produced them is not part of this repository. See
`docs/vectors.md`.

The suite covers this crate alone: 249 frozen cases across seven groups, with
1648 assertions. The harness and its exclusions are part of this repository, so
the suite runs from a clean clone with nothing but crates.io.
