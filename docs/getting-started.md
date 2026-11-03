# Getting started

From a checkout to a working call in about ten minutes, in any language that can
call a C ABI.

colander is a form engine behind a C ABI. It compiles a form, evaluates its
rules, validates a response against them and hashes the result. It holds no
state, performs no I/O and ships no schema: every call is one JSON request string
in and one JSON envelope out.

## Quick path

1. **Build and link.** Run `cargo build --release`; the artifact is
   `target/release/libcolander.so` (`.dylib` or `.dll` elsewhere). Declare the
   exported functions, either from `include/colander.h` or by hand.
2. **Call an entry point** with a NUL-terminated UTF-8 JSON request string.
3. **Read the envelope.** Check `ok` before touching `result`.
4. **Free the string** with `colander_free_string()`. Never call `free()`.

```c
#include "colander.h"

char *text = colander_compile(
    "{\"formSchemaJson\":\"{\\\"schemaVersion\\\":\\\"1.0.0\\\",\\\"fields\\\":[]}\"}");
if (text) {
    /* parse the envelope, then: */
    colander_free_string(text);
}
```

The same call from Rust, using the helper that returns text instead of a pointer:

```rust
let reply = colander::ffi::call_with_text(request, colander::ffi::compile::colander_compile);
```

## The envelope

Every `char *`-returning function produces one of two shapes:

```json
{"ok":true,"result":{…}}
```

```json
{"ok":false,"error":{"kind":"…","message":"…"}}
```

`error.kind` has exactly three values:

| `kind`            | Produced when                                                            | Meaning              |
| ----------------- | ------------------------------------------------------------------------ | -------------------- |
| `invalid_request` | The request was NULL, not valid UTF-8, not valid JSON, or not an object. | You called it wrong. |
| `validation`      | The core rejected the request's content.                                 | The payload is bad.  |
| `panic`           | A Rust panic at the boundary, caught and returned as a failure envelope. | A bug in colander.   |

**Check `ok` before touching `result`, and release every returned string with
`colander_free_string()`.** No `char *`-returning entry point returns NULL, so
NULL is never an "is this an error?" test. The full contract is in
[abi.md](abi.md).

Check the ABI version once at startup: `colander_abi_version()` returns a plain
`uint32_t` (currently `1`), and a mismatch means refusing to bind.

```c
if (colander_abi_version() != 1) { /* refuse to bind */ }
```

## Entry points at a glance

| Symbol                       | Purpose                                                                 | Required request keys               |
| ---------------------------- | ----------------------------------------------------------------------- | ----------------------------------- |
| `colander_compile`           | Expand `component-ref` fields; produce canonical documents and a hash   | `formSchemaJson`                    |
| `colander_describe_form`     | Report the field index of a compiled triple                             | `formSchemaJson`                    |
| `colander_evaluate_rules`    | Evaluate visibility, enablement, required, calculations and validations | `formSchemaJson`, `rulesSchemaJson` |
| `colander_validate_response` | Validate and normalize submitted answers                                | `formSchemaJson`, `answersJson`     |
| `colander_validate_schema`   | Validate a document against a schema you supply                         | `kind`-dependent                    |
| `colander_content_hash`      | Hash a form/ui/rules triple as given                                    | `formSchemaJson`                    |
| `colander_next_version`      | Pick the next version from the published set and a bump                 | —                                   |
| `colander_version_info`      | Report name, crate version and ABI version                              | none                                |
| `colander_abi_version`       | Return the ABI version as `uint32_t`                                    | none                                |
| `colander_free_string`       | Release a string this library returned                                  | —                                   |
| `colander_alloc`             | Allocate a request buffer inside the library's heap                     | —                                   |
| `colander_free_buffer`       | Release a buffer from `colander_alloc`                                  | —                                   |

Two rules cover the table:

- `colander_version_info` and `colander_abi_version` take no request argument.
  The seven document functions take a JSON request; `colander_free_string`
  releases a returned string, and the allocator pair moves memory.
- A key that is **present with the wrong JSON type is rejected**, required or
  not. Absent means "use the default"; an explicit `null` counts as the wrong
  type. `{"mode": 3}` and `{"mode": null}` both fail with
  `kind:"validation"`, while an omitted `mode` uses the default.

The full request and response keys are in [entry-points.md](entry-points.md) and
[entry-points-payload.md](entry-points-payload.md).

## See also

- The boundary contract: [abi.md](abi.md)
- The mental model: [concepts.md](concepts.md)
- The three document schemas: [documents.md](documents.md)
- Accepting a submission: [validation.md](validation.md)
