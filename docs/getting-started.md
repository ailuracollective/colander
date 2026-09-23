# Getting started

This page takes you from a checkout to a working call in about ten minutes, in
any language that can call a C ABI.

colander is a form engine with a C ABI. It compiles a form, evaluates its
rules, validates a response against them, and hashes the result. It holds no
state, performs no I/O and ships no schemas: every call is one JSON request
string in and one JSON envelope out.

## Quick path

1. **Link the library.** Build with `cargo build --release`; the artifact is
   `target/release/libcolander.so` (`.dylib` / `.dll` elsewhere). Declare the
   functions from `include/colander.h`.
2. **Call one entry point** with a NUL-terminated UTF-8 JSON request string.
3. **Read the envelope.** `{"ok":true,"result":{…}}` on success,
   `{"ok":false,"error":{…}}` on failure. Check `ok` before touching `result`.
4. **Free the string** with `colander_free_string()`. Never `free()` it.

```c
#include "colander.h"

char *text = colander_compile(
    "{\"formSchemaJson\":\"{\\\"schemaVersion\\\":\\\"1.0.0\\\",\\\"fields\\\":[]}\"}");
if (text) {
    /* parse the envelope, then: */
    colander_free_string(text);
}
```

The same call in Rust, using the helper that returns text instead of a pointer:

```rust
let reply = colander::ffi::call_with_text(request, colander::ffi::compile::colander_compile);
```

## The envelope

Every `char *`-returning function produces exactly one of these two shapes:

```json
{"ok":true,"result":{…}}
```

```json
{"ok":false,"error":{"kind":"…","message":"…"}}
```

`error.kind` has exactly three values:

| `kind`            | Produced when                                                                           | Meaning              |
| ----------------- | --------------------------------------------------------------------------------------- | -------------------- |
| `invalid_request` | The request was NULL, not valid UTF-8, not valid JSON, or valid JSON but not an object. | You called it wrong. |
| `validation`      | The core rejected the request's content.                                                | The payload is bad.  |
| `panic`           | A Rust panic at the boundary, caught and returned as a failure envelope.                | A bug in colander.   |

The single most important rule: **check `ok` before touching `result`, and
release every returned string with `colander_free_string()` — never `free()`.** No
function returns NULL, so NULL is never a valid "is this an error?" test. The
full contract, including memory ownership and panic behaviour, is in
[abi.md](abi.md).

## Entry points at a glance

| Symbol                       | Purpose                                                                             | Required request keys               |
| ---------------------------- | ----------------------------------------------------------------------------------- | ----------------------------------- |
| `colander_compile`           | Expand `component-ref` fields and produce canonical documents plus a content hash   | `formSchemaJson`                    |
| `colander_evaluate_rules`    | Evaluate visibility, enablement, required, calculations and cross-field validations | `formSchemaJson`, `rulesSchemaJson` |
| `colander_validate_response` | Validate and normalize submitted answers                                            | `formSchemaJson`, `answersJson`     |
| `colander_validate_schema`   | Validate a document against a JSON Schema you supply                                | `kind`-dependent                    |
| `colander_content_hash`      | Hash a form/ui/rules triple                                                         | `formSchemaJson`                    |
| `colander_next_version`      | Pick the next patch version                                                         | —                                   |
| `colander_version_info`      | Report name, crate version and ABI version                                          | none (takes no argument)            |
| `colander_abi_version`       | Return the ABI version as `uint32_t`                                                | none (takes no argument)            |
| `colander_free_string`       | Release a string this library returned                                              | —                                   |
| `colander_alloc`             | Allocate a request buffer inside the library's heap                                 | —                                   |
| `colander_free_buffer`       | Release a buffer from `colander_alloc`                                              | —                                   |

Two asymmetries worth memorising:

- `colander_version_info` and `colander_abi_version` take **no request argument**; the
  other six do. The last two rows of the table are not entry points in that
  sense: `colander_alloc` and `colander_free_buffer` move memory rather than JSON.
- Optional keys of the **wrong JSON type are silently ignored**, as if absent.
  Only required keys reject a bad type. So `{"mode": 3}` behaves like
  `{"mode": "Draft"}`.

The request and response keys for each function are documented in full in
[entry-points.md](entry-points.md).

## Check the ABI version at startup

`colander_abi_version()` returns the ABI version as a plain `uint32_t` (currently
`1`), with no JSON and no allocation. Check it at startup and refuse to run on a
mismatch:

```c
if (colander_abi_version() != 1) { /* refuse to bind */ }
```

## Where to go next

- The ABI contract in full: [abi.md](abi.md)
- Every entry point, request and response: [entry-points.md](entry-points.md)
- The mental model: [concepts.md](concepts.md)
- The three document schemas: [documents.md](documents.md)
- Writing rules: [rules.md](rules.md)
- Accepting a submission: [validation.md](validation.md)
- Behaviour that surprises people: [gotchas.md](gotchas.md)
- Terms used across these pages: [glossary.md](glossary.md)
