# colander documentation

colander is a portable form runtime exposed as a C ABI shared library. It
compiles a form, evaluates its rules, validates a response against them and hashes
the result. It holds no state, performs no I/O and ships no schema of its own.

The pages are split by task. Read only what you need before your first call.

## Documents

| Document                                           | What it covers                                                                      | Read it when                                                        |
| -------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| [getting-started.md](getting-started.md)           | Build, make a first call, read the envelope, free the string.                       | You want a first call working.                                      |
| [concepts.md](concepts.md)                         | Request/response, the three documents, `id` versus `code`, caller-side types.       | You are designing a form and want the model before the syntax.      |
| [glossary.md](glossary.md)                         | Definitions of the terms used across these pages.                                   | You met a term you do not recognise.                                |
| [abi.md](abi.md)                                   | Envelope, ownership, the allocator pair, panics, escaping, the header's gaps.       | You are writing a binding or debugging the boundary.                |
| [entry-points.md](entry-points.md)                 | Reference for compile, describe, rule evaluation and response validation.           | You need the request or response keys for a document operation.     |
| [entry-points-payload.md](entry-points-payload.md) | Schema validation, hashing, versioning, string release and the allocator pair.      | You need a payload, version or memory symbol.                       |
| [documents.md](documents.md)                       | The form, UI and rules schemas, field types, pages, expressions and groups.         | You are authoring a form, UI or rules document.                     |
| [ide-schemas.md](ide-schemas.md)                   | The `schemas/` JSON Schema templates for IDE autocomplete.                          | You are authoring JSON and want completion and validation.          |
| [rules.md](rules.md)                               | Operators, comparison and truthiness, row scope, analysis errors.                   | You are writing `visibleWhen`, `calculate` or `assert` expressions. |
| [validation.md](validation.md)                     | Draft versus Complete, page-scoped checks, normalization and every error code.      | You are accepting a submission or handling an `errors[]` result.    |
| [json-schema.md](json-schema.md)                   | The Draft 2020-12 subset used by `colander_validate_schema`.                        | You are passing a JSON Schema to the core.                          |
| [json.md](json.md)                                 | Parser limits, number output and the key-sorted form.                               | You care about key order, number spelling or hashing.               |
| [codecs.md](codecs.md)                             | The codec seam: JSON and MessagePack, and why injection is static.                  | You are changing what travels between a document and its bytes.     |
| [gotchas.md](gotchas.md)                           | Behaviour worth knowing, most of it inherited rather than chosen.                   | Something surprised you.                                            |
| [dependencies.md](dependencies.md)                 | What stays hand-written, and why.                                                   | You wonder why the crate has so few dependencies.                   |
| [vectors.md](vectors.md)                           | What the frozen golden vectors assert.                                              | You are running or extending the test suite.                        |
| [releases.md](releases.md)                         | The release-please flow, the header guard, crates.io publishing and recovery.       | You are cutting a release or publishing to crates.io.               |
| [conformance.md](conformance.md)                   | Cross-language parity: the value domain, errors and hashes a wrapper must preserve. | You are writing or checking a WASM, TypeScript or .NET wrapper.     |

## Reading paths

**A first call in ten minutes.** Read [getting-started.md](getting-started.md),
then [abi.md](abi.md) for the boundary rules.

**Designing a form.** Read [concepts.md](concepts.md), then
[documents.md](documents.md), then [rules.md](rules.md), then
[validation.md](validation.md).

**Debugging an error.** Read [validation.md](validation.md) for the error codes,
[gotchas.md](gotchas.md) for known surprises, and [abi.md](abi.md) for envelope
and memory problems.

## Repository map

- [README.md](../README.md) — what the crate is, how to build it and how to test it.
- [SPEC.md](../SPEC.md) — the contract of record: normative requirements and their status.
- [include/colander.h](../include/colander.h) — the generated C header; it declares ten of the twelve exports (see [abi.md](abi.md#the-generated-header-does-not-declare-every-export)).
- [src/](../src/) — the Rust sources, one file per concern.
- [tests/](../tests/) — unit tests and the golden-vector suite.
- [schemas/](../schemas/) — base JSON Schema templates ([ide-schemas.md](ide-schemas.md)).
