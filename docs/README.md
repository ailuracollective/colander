# colander documentation

This directory is the reference for colander, a domain-neutral form core
exposed as a C-ABI shared library. colander compiles a form, evaluates its
rules, validates a response against them and hashes the result; it holds no
state, performs no I/O and ships no schemas of its own. The pages below are split
by task, so you can read only what you need before your first call.

## Documents

| Document                                 | What it covers                                                                                                        | Read it when                                                        |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| [getting-started.md](getting-started.md) | The ten-minute path: build, call, read the envelope, free the string.                                                 | You want a first call working.                                      |
| [concepts.md](concepts.md)               | The mental model: request/response, the three documents, `id` versus `code`, where the AI layer sits.                 | You are designing a form and want the model before the syntax.      |
| [glossary.md](glossary.md)               | Plain-language definitions of the terms used across these pages.                                                      | You met a term you do not recognise.                                |
| [abi.md](abi.md)                         | The wire contract: envelope, memory ownership, the allocator pair, panics, escaping, and the generated header's gaps. | You are writing a language binding or debugging the boundary.       |
| [entry-points.md](entry-points.md)       | Full reference for all twelve exported symbols.                                                                       | You need the request and response keys for one call.                |
| [documents.md](documents.md)             | The form, UI and rules schemas, field types and expressions.                                                          | You are authoring a form, UI or rules document.                     |
| [rules.md](rules.md)                     | Rule operators, their semantics and the analysis errors.                                                              | You are writing `visibleWhen`, `calculate` or `assert` expressions. |
| [validation.md](validation.md)           | Response validation, normalization and every error code.                                                              | You are accepting a submission or handling an `errors[]` result.    |
| [json-schema.md](json-schema.md)         | The supported Draft 2020-12 subset used by `colander_validate_schema`.                                                | You are passing a JSON Schema to the core.                          |
| [json.md](json.md)                       | The parser, its limits, number output and canonical form.                                                             | You care about key order, number spelling or hashing.               |
| [gotchas.md](gotchas.md)                 | Behaviour worth knowing, most of it inherited rather than chosen.                                                     | Something surprised you.                                            |
| [dependencies.md](dependencies.md)       | What stays hand-written, and why.                                                                                     | You wonder why the crate has so few dependencies.                   |
| [vectors.md](vectors.md)                 | What the frozen golden vectors assert.                                                                                | You are running or extending the test suite.                        |
| [releases.md](releases.md)               | The release pipeline: cocogitto bumps, the header guard, what CI releases, and the manual crates.io publish.          | You are cutting a release or publishing to crates.io.               |

## Reading paths

**I want to call the library in 10 minutes.** Read
[getting-started.md](getting-started.md), then [abi.md](abi.md) for the boundary
rules.

**I am designing a form.** Read [concepts.md](concepts.md), then
[documents.md](documents.md), then [rules.md](rules.md), then
[validation.md](validation.md).

**I am debugging an error.** Read [validation.md](validation.md) for the error
codes, [gotchas.md](gotchas.md) for known surprises, and [abi.md](abi.md) for
envelope and memory problems.

## Repository map

- [README.md](../README.md) — what the crate is, how to build it and how to test it.
- [include/colander.h](../include/colander.h) — the generated C header; it declares nine of the twelve exports (see [abi.md](abi.md#the-generated-header-does-not-declare-every-export)).
- [src/](../src/) — the Rust sources, one file per concern.
- [tests/](../tests/) — the unit tests and the golden-vector suite.

## Next

Start with [getting-started.md](getting-started.md).
