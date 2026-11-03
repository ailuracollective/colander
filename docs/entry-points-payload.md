# Payload and utility entry points

Reference for the symbols that take a payload, a version or no document at all,
continuing [entry-points.md](entry-points.md). The envelope is the one in
[abi.md](abi.md); the keys follow.

## colander_validate_schema

Validates a document against a JSON Schema **you supply**; colander ships none of
its own. `schemas` carries the text of whichever schemas the call needs, required
for the kinds actually validated. `kind` selects what is checked and defaults to
`"form"`.

### Request

| `kind`        | Required document keys       | Required `schemas` entries                                                                   |
| ------------- | ---------------------------- | -------------------------------------------------------------------------------------------- |
| `"form"`      | `formSchemaJson`             | `formSchema`; `uiSchema` iff `uiSchemaJson` given; `rulesSchema` iff `rulesSchemaJson` given |
| `"component"` | `formSchemaJson`             | `formSchema`; `uiSchema` iff `uiSchemaJson` given                                            |
| `"workflow"`  | `workflowSchemaJson`         | `workflowSchema`                                                                             |
| `"instance"`  | `schemaJson`, `instanceJson` | none                                                                                         |

`"instance"` is the domain-free entry point: any instance against any schema,
with `label` (default `"instance"`) only naming the document in error messages. A
`schemas` key present but not an object is rejected in every kind, including
`"instance"`, which takes no schema from it (SPEC C-9).

For `"form"`, a rules schema that passes structural validation is also checked
with [`validate_dependencies`](rules.md).

The supported JSON Schema subset is in [json-schema.md](json-schema.md).

### Response

```json
{"ok":true,"result":{"valid":true}}
```

**`{"valid":false}` is never produced.** A document that fails its schema returns
a failure envelope with `kind:"validation"` and a message listing at most the
first five errors, with `(truncated: 5 of N errors shown)` appended when there are
more (SPEC S-7):

```
JSON_PARSE_ERROR: Invalid form schema: required: required property 'schemaVersion' is missing; type: expected object but found string
```

`published` is not accepted for `kind:"workflow"`: a present key fails the call
(SPEC X-2). Workflow semantic validation is out of scope; only JSON Schema
validation runs.

### Errors

| Message                                                                                                          | Cause                                               |
| ---------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| `Unknown schema kind 'x' (expected 'form', 'component', 'workflow' or 'instance').`                              | Unrecognized `kind`                                 |
| `'published' is not accepted for kind:"workflow".`                                                               | `published` present with `kind:"workflow"`          |
| `schemas is required: pass the JSON Schema text for each document kind, e.g. {"formSchema":"…","uiSchema":"…"}.` | `schemas` missing or not an object                  |
| `schemas.formSchema is required to validate this request.`                                                       | A needed entry is absent or not a string            |
| `JSON_PARSE_ERROR: Invalid form schema definition: …`                                                            | A schema in `schemas` is not valid JSON             |
| `JSON_PARSE_ERROR: Invalid form schema: …`                                                                       | The document is not valid JSON, or fails the schema |

An empty string counts as present, so `{"formSchema":""}` satisfies the
requirement and then fails as a non-object schema.

## colander_content_hash

Hashes a form/ui/rules triple **as given**, with no compilation: a byte-exact
fingerprint of the stored bytes, not the content, and not used for pinning. The
authoritative hash of a compiled form is the `contentHash` that
`colander_compile` reports. Compare two stored documents byte-for-byte, or
reproduce that hash from the compiled documents (SPEC H-5).

### Request and response

| Request key       | Type   | Required |
| ----------------- | ------ | -------- |
| `formSchemaJson`  | string | yes      |
| `uiSchemaJson`    | string | no       |
| `rulesSchemaJson` | string | no       |

`result` is exactly `{"contentHash":"<64 lowercase hex chars>"}`.

This one parses with the general JSON parser, so a document may be any JSON
value, not only an object.

### What is hashed

The exact string:

```
{"form":<form>,"ui":<ui|null>,"rules":<rules|null>}
```

with these rules:

- The three keys are always present and always in the order `form`, `ui`,
  `rules`: **document key order, not sorted**.
- A missing document is written as the bare token `null`.
- Each document is serialized in its own document order, with number literals
  preserved exactly: `1.50` stays `1.50`, `1e3` stays `1e3`.
- Strings are escaped by the writer's rules: printable ASCII stays literal
  except `"`, `\`, `&`, `'`, `+`, `<`, `>` and `` ` ``; `"` becomes `\u0022`,
  `\` becomes `\\`; control characters use `\b`, `\t`, `\n`, `\f`, `\r` or
  `\uXXXX`; non-ASCII becomes `\uXXXX` uppercase, with surrogate pairs above
  U+FFFF.
- The digest is SHA-256 over the UTF-8 bytes of that string.

So `{"b":1,"a":1.50}` with a UI of `{"z":"é"}` and no rules hashes:

```
{"form":{"b":1,"a":1.50},"ui":{"z":"\u00E9"},"rules":null}
```

Documents differing only in whitespace hash the same; differing in key order, they
do not. Compile first for order-insensitive hashing. Canonical form and
number-output rules: [json.md](json.md).

## colander_next_version

| Request key | Type             | Required |
| ----------- | ---------------- | -------- |
| `published` | array of strings | no       |
| `bump`      | string           | no       |

`result` is exactly `{"next":"1.2.4"}`.

With nothing published, or `published` absent, the answer is `"1.0.0"`. Otherwise
colander parses every entry, takes the highest and applies the bump:
`["1.2.3","1.10.0","1.9.9"]` yields `"1.10.1"` for `patch`, `"1.11.0"` for
`minor` and `"2.0.0"` for `major`. `bump` defaults to `"patch"`; any other name
is rejected. Ties keep the last maximal entry.

A present `published` must be an array of strings; any other JSON type is
rejected. Every entry is parsed, so one invalid entry fails the whole call with
`INVALID_SEMVER: invalid semantic version: <entry>`.

### Accepted version syntax

Exactly three dot-separated numeric identifiers under SemVer 2.0.0's rules: ASCII
digits only, no sign, no whitespace, no blank segments, and no leading zero unless
the segment is exactly `0`. Pre-release and build metadata are not permitted.

| Input                                 | Result    |
| ------------------------------------- | --------- |
| `1.2.3`                               | {1, 2, 3} |
| `0.0.0`                               | {0, 0, 0} |
| `1 . 2 . 3`                           | rejected  |
| `01.0.0`                              | rejected  |
| `-0.0.0`, `+1.0.0`                    | rejected  |
| `1..0.0`                              | rejected  |
| `1.0`                                 | rejected  |
| `1.0.0.0`                             | rejected  |
| `1.-1.0`                              | rejected  |
| `1.0.0-beta`, `1.0.0+build`, `v1.0.0` | rejected  |

## Version and utility functions

Five exports carry no JSON request: three report on the library, one releases a
returned string, the allocator pair moves memory. Ownership and escaping are in
[abi.md](abi.md).

### colander_version_info

`colander_version_info()` takes **no argument** and cannot fail:

```json
{"ok":true,"result":{"name":"colander","version":"0.4.1","abi":1}}
```

### colander_abi_version

`colander_abi_version()` returns the ABI version as a plain `uint32_t` (currently
`1`), with no JSON and no allocation. Check it at startup; refuse to run on a
mismatch.

```c
if (colander_abi_version() != 1) { /* refuse to bind */ }
```

### colander_free_string

`colander_free_string(pointer)` releases a string this library returned, and is a
no-op on NULL.

### The allocator pair

`colander_alloc(length)` and `colander_free_buffer(pointer, length)` exist because
ownership of the output is not enough: a caller that cannot allocate inside the
library's heap also has to produce the input, as any embedder without a shared
allocator does.

```c
char *buffer = colander_alloc(request_length + 1);
memcpy(buffer, request, request_length);
buffer[request_length] = '\0';
char *response = colander_compile(buffer);
colander_free_buffer(buffer, request_length + 1);
/* … use response, then */
colander_free_string(response);
```

The pair is the whole contract: the size cannot be recovered from the pointer, so
the length you asked for is the length you pass back. `colander_alloc` returns
NULL for a zero length or an unrepresentable layout, so a non-NULL result is safe
to write to; freeing NULL or a zero length is a no-op. A native caller can pass a
buffer it allocated itself.

`include/colander.h` declares ten of the twelve exports; `colander_alloc` and
`colander_free_buffer` are deliberately absent, so a C caller using either
declares it first. Reasoning:
[abi.md](abi.md#the-generated-header-does-not-declare-every-export).

## See also

- The three document schemas: [documents.md](documents.md)
- The wire contract: [abi.md](abi.md)
- Rule operators and analysis errors: [rules.md](rules.md)
