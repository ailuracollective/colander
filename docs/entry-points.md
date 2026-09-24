# Entry points

This page is the reference for the entire exported surface of colander: the
eleven symbols the shared library exposes, the request keys each one accepts,
the response it returns and the errors it can produce. Read it when you need the
exact request or response shape for one call.

Every `char *`-returning function returns the envelope described in
[abi.md](abi.md) — `{"ok":true,"result":{…}}` on success or
`{"ok":false,"error":{"kind":"…","message":"…"}}` on failure. Check `ok` before
reading `result`; no `char *`-returning entry point returns NULL.

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

Two behaviours worth memorising:

- `colander_version_info` and `colander_abi_version` take **no request argument**; the
  other six do. The last two rows of the table are not entry points in that
  sense: `colander_alloc` and `colander_free_buffer` move memory rather than JSON.
- An optional key that is **present with the wrong JSON type is rejected**, the
  same as a required key. "Absent" means "use the default"; "present with the
  wrong type" is an error, so `{"mode": 3}` fails instead of behaving like
  `{"mode": "Draft"}`. An explicit `null` counts as the wrong type, not as an
  absence, so a caller that would serialise an absent value as `null` must omit
  the key: `{"mode": null}` fails. The failure is the request failing its own type
  check, so it comes back as `kind:"validation"`. The same check reaches the keys
  of an object the request carries: a `components[]` entry's `uiSchemaJson` or
  `contentHash` of the wrong type is rejected (SPEC C-8), so `{"contentHash": 7}`
  fails instead of compiling with an empty hash.

## colander_compile

Expands `component-ref` fields into groups, canonicalizes the three documents
and hashes the result. This is the entry point to run before storing or
publishing a form.

### Request

| Key               | Type   | Required | Notes                                                      |
| ----------------- | ------ | -------- | ---------------------------------------------------------- |
| `formSchemaJson`  | string | yes      | JSON text of the form schema                               |
| `uiSchemaJson`    | string | no       | JSON text of the UI schema                                 |
| `rulesSchemaJson` | string | no       | JSON text of the rules schema                              |
| `components`      | array  | no       | Resolved component versions; must be an array when present |

Each entry of `components` is an object:

| Key              | Type   | Required |
| ---------------- | ------ | -------- |
| `code`           | string | yes      |
| `version`        | string | yes      |
| `formSchemaJson` | string | yes      |
| `uiSchemaJson`   | string | no       |
| `contentHash`    | string | no       |

`uiSchemaJson` and `contentHash` must be strings when present: `{"contentHash": 7}`
is a validation failure, not an empty pin.

There is no repository and no callback: the caller decides what "published"
means by choosing which component versions to hand in.

### Response

`result` has exactly these keys, in this order:

| Key                      | Type           | Notes                                           |
| ------------------------ | -------------- | ----------------------------------------------- |
| `formSchemaJson`         | string         | Canonical form schema, `component-ref` expanded |
| `uiSchemaJson`           | string \| null | `null` when no UI schema was supplied           |
| `rulesSchemaJson`        | string \| null | `null` when no rules schema was supplied        |
| `dependencyMetadataJson` | string         | Component list plus rule evaluation order       |
| `contentHash`            | string         | Lowercase hex SHA-256 of the compiled triple    |

All three documents are serialized canonically: **keys sorted by UTF-8 byte
order**, no whitespace, number literals preserved.

### What compilation produces

- The form schema becomes `{"schemaVersion":…, "$schema":…, "fields":[…]}`.
  `schemaVersion` is copied through and becomes `null` if absent. `$schema` is
  included only when present and non-null. Every other top-level key is dropped.
- Each field is cloned as-is except `items`, which is compiled recursively.
- An `id` is **not** generated, and `code`/`type` are not touched. `compile`
  itself does not require `id` or `code`, and never checks `type` against the
  list of known types — an unknown type survives compilation and only fails
  later, at response validation.
- The UI schema keeps `schemaVersion`, `formSchemaVersion`, `fields`, `$schema`
  and `layout`. Its `fields` object is always emitted (empty if absent); its
  keys are sorted, then the `fields` of every resolved component are merged in,
  ordered by component code, skipping component entries whose value is `null`.
- The rules schema keeps `schemaVersion`, `formSchemaVersion`, `fields`
  (sorted), `$schema` and `validations`.
- A rules schema that is present is dependency-checked before
  `dependencyMetadataJson` is built. The check runs against the compiled form
  and rules, so a component's fields are visible to references; see
  [rules.md](rules.md).

`dependencyMetadataJson` is the object:

```json
{"components":[{"code":"…","version":"…","contentHash":"…"}],"rules":{"calculatedFieldIds":["…"],"evaluationOrder":["…"]}}
```

`components` is always present (empty when there are no references), sorted by
`code`, then by version, with `contentHash` set to the empty string when the
caller did not supply one. An empty string is not a pin, so it is carried
unverified; a non-empty pin is checked first (see **Component references** below).
`rules` appears only when a rules schema was
compiled; `calculatedFieldIds` is in document order and `evaluationOrder` is the
topological order calculations run in. The metadata is **not** part of the
content hash.

### Component references

A field of `type: "component-ref"` is replaced by a group:

```json
{"id":"…","code":"…","type":"group","items":[ …the component's compiled fields… ],"required":…,"readOnly":…,"description":…}
```

`required`, `readOnly` and `description` are copied only when present and
non-null; **every other property of the reference field is dropped**.
`componentCode` is required and must be non-empty. `componentVersion` must be
present, non-blank and valid semver. A reference cycle is an error.

The UI layout node for that field is rewritten to a group node whose `children`
come from the component's own UI `layout`; if the component has no layout, colander
builds the default `[{"type":"field","fieldId":"<id>"}, …]` from the component's
fields.

Components are resolved by exact `(code, version)` match against the `components`
batch you pass, and resolution is memoized, so referencing the same version twice
yields one metadata entry.

Each resolved component is pinned by its `contentHash` (SPEC P-3). When the field
is a non-empty string, colander compiles the component on its own — its nested
`component-ref` fields expanded from the same batch and no rules document — and
compares the digest. A mismatch fails the call with `COMPONENT_HASH_MISMATCH`.
An absent key, or an empty string, is **not** a pin: the component compiles and is
carried into `dependencyMetadataJson` with the empty string, unverified. The pin
covers exact bytes, so `1.50` and `1.5` hash differently.

### Errors

| Message                                                                                                                  | Cause                                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `'formSchemaJson' is required and must be a string.`                                                                     | Missing or non-string required key                                                                                               |
| `Invalid form schema: expected a JSON object.`                                                                           | Valid JSON that is not an object                                                                                                 |
| `Expected array at /fields.`                                                                                             | The form has no `fields` array                                                                                                   |
| `Expected field object at /fields/0.`                                                                                    | A field is not an object                                                                                                         |
| `Expected non-empty string at /fields/0/type.`                                                                           | A field has no `type`                                                                                                            |
| `components[] entries must be objects.`                                                                                  | A `components` element is not an object                                                                                          |
| `components[].code is required.`                                                                                         | Missing `code` (same for `version`, `formSchemaJson`)                                                                            |
| `COMPONENT_VERSION_REQUIRED: component-ref at /fields/0 must include componentVersion before publication.`               | Blank or absent `componentVersion`                                                                                               |
| `COMPONENT_VERSION_NOT_FOUND: component 'x' version '1.0.0' referenced at /fields/0 was not found or is not published.`  | No matching entry in `components`                                                                                                |
| `CIRCULAR_COMPONENT_REFERENCE: component 'x' references itself through a -> b -> x.`                                     | A component reference cycle                                                                                                      |
| `COMPONENT_HASH_MISMATCH: component 'x' version '1.0.0' declares contentHash '…' but its compiled triple hashes to '…'.` | A non-empty `contentHash` pin does not match the component's own compiled triple                                                 |
| `Invalid semantic version: 1.0`                                                                                          | Bad `componentVersion`                                                                                                           |
| `Expected field id at /fields/0/id.`                                                                                     | A field lacks a string `id`, raised while analysing the rules for `dependencyMetadataJson` — only when a rules schema is present |
| `The node must be of type 'JsonObject'.`                                                                                 | A `fields` element is not an object                                                                                              |

## colander_evaluate_rules

Evaluates the rules against a set of values. Use it to drive a live form: which
fields are visible, enabled or required, what the calculated fields evaluate to,
and which cross-field validations currently fail.

### Request

| Key               | Type   | Required | Notes                                           |
| ----------------- | ------ | -------- | ----------------------------------------------- |
| `formSchemaJson`  | string | yes      | JSON text; must parse to an object              |
| `rulesSchemaJson` | string | yes      | JSON text; must parse to an object              |
| `uiSchemaJson`    | string | no       | Only `fields.<id>.hidden` is read               |
| `values`          | object | no       | Current answers; must be an object when present |

The **keys of `values` are field codes**, not field ids.

### Response

| Key                | Type                       | Keyed by       | Notes                                                      |
| ------------------ | -------------------------- | -------------- | ---------------------------------------------------------- |
| `visibility`       | object of bool             | **field id**   | `false` when the UI schema marks the field `hidden`        |
| `enabled`          | object of bool             | **field id**   | `false` for a `readOnly` field, unless a rule overrides it |
| `required`         | object of bool             | **field id**   | From the form schema, unless a rule overrides it           |
| `calculatedValues` | object                     | **field code** | Only fields with a `calculate` expression                  |
| `validationErrors` | array of `{code, message}` | —              | Cross-field validations that failed                        |

The id/code split is the single easiest thing to get wrong here: the three
boolean maps are keyed by `id`, `calculatedValues` by `code`.

Every field in the form appears in the three boolean maps, whether or not it has
any rule. Only fields with rules can change.

A validation entry contributes an error when its `assert` expression evaluates
falsy. Its `when` guards it: if `when` is present and evaluates falsy, the entry
is skipped. An entry with no `assert` never reaches evaluation: the analyzer
rejects it, whether or not it carries `when`. An entry with no `code` is
reported as `VALIDATION_<position>` and one with no `message` as
`"Validation failed."`.

A supplied `rulesSchemaJson` is dependency-checked before evaluation: the
`RULE_*` analysis errors in [rules.md](rules.md) abort the call.

The operators an expression may use, the comparison and truthiness rules, and
the analysis errors are in [rules.md](rules.md).

## colander_validate_response

Validates submitted answers and returns them normalized. This is the entry point
for accepting a submission.

### Request

| Key               | Type   | Required | Notes                                            |
| ----------------- | ------ | -------- | ------------------------------------------------ |
| `formSchemaJson`  | string | yes      | JSON text; must parse to an object               |
| `answersJson`     | string | yes      | JSON text; must parse to an **object**           |
| `uiSchemaJson`    | string | no       | Only `fields.<id>.hidden` is read                |
| `rulesSchemaJson` | string | no       | Absent or whitespace-only is treated as no rules |
| `mode`            | string | no       | `"Draft"` (default) or `"Complete"`              |

When `rulesSchemaJson` is absent or blank, colander synthesizes an empty rules
document pinned to the form's `schemaVersion` (or `1.0.0` if the form has none),
so no version-mismatch error can occur. A rules document that is present is
dependency-checked exactly like `colander_validate_schema` with `kind:"form"`;
the synthesized document is not.

### Response

| Key                     | Type                             | Notes                                   |
| ----------------------- | -------------------------------- | --------------------------------------- |
| `normalizedAnswersJson` | string                           | Compact JSON object of converted values |
| `errors`                | array of `{code, path, message}` | Empty when the response is valid        |
| `isValid`               | bool                             | `true` exactly when `errors` is empty   |

`normalizedAnswersJson` contains only fields that were accepted and converted:
keyed by field code, in the order the fields appear in the form. A field that
produced an error is omitted. Repeaters appear as arrays of row objects.

`errors[].path` is a JSON pointer to the field's location **in the form schema**
— `/fields/0`, `/fields/0/items/1`, `/fields/2/1/<childId>` for a repeater row —
with two exceptions: an unknown answer key reports `/answers/<key>`, and a failed
cross-field validation reports `/rules/validations`.

What each mode reports, and the full list of `errors[].code` values, are in
[validation.md](validation.md).

### The order errors are produced in

1. Unknown top-level answer keys.
2. Scalar fields, in form order.
3. Repeaters, in form order: row count, then each row's children.
4. Calculated fields.
5. Cross-field validations (Complete only).

Within one field only the first applicable error is reported, and a field that
errors is never normalized. The checks run in this order: hidden/disabled →
read-only → required → type conversion → constraints.

## colander_validate_schema

Validates a document against a JSON Schema **you supply**. colander ships no
schemas of its own: `schemas` carries the text of whichever schemas the call
needs, and each one is required for the kinds actually validated.

`kind` selects what is checked and defaults to `"form"`.

### Request

| `kind`        | Required document keys       | Required `schemas` entries                                                                   |
| ------------- | ---------------------------- | -------------------------------------------------------------------------------------------- |
| `"form"`      | `formSchemaJson`             | `formSchema`; `uiSchema` iff `uiSchemaJson` given; `rulesSchema` iff `rulesSchemaJson` given |
| `"component"` | `formSchemaJson`             | `formSchema`; `uiSchema` iff `uiSchemaJson` given                                            |
| `"workflow"`  | `workflowSchemaJson`         | `workflowSchema`                                                                             |
| `"instance"`  | `schemaJson`, `instanceJson` | none                                                                                         |

`"instance"` is the domain-free entry point: it validates any instance against
any schema, and `label` (default `"instance"`) only names the document in error
messages.

For `"form"`, a rules schema that passes structural validation is also checked
with [`validate_dependencies`](rules.md).

The subset of JSON Schema the core understands is documented in
[json-schema.md](json-schema.md).

### Response

```json
{"ok":true,"result":{"valid":true}}
```

**`{"valid":false}` is never produced.** A document that fails its schema comes
back as a failure envelope with `kind:"validation"` and a message listing at
most the first five errors. When there are more, the message says so with
`(truncated: 5 of N errors shown)` (SPEC S-7):

```
Invalid form schema: required: required property 'schemaVersion' is missing; type: expected object but found string
```

`published` is not accepted for `kind:"workflow"`: a present key fails the call
(SPEC X-2). Workflow semantic validation, including any notion of a published
version, is out of scope; only JSON Schema validation runs.

### Errors

| Message                                                                                                          | Cause                                               |
| ---------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| `Unknown schema kind 'x' (expected 'form', 'component', 'workflow' or 'instance').`                              | Unrecognized `kind`                                 |
| `'published' is not accepted for kind:"workflow".`                                                               | `published` present with `kind:"workflow"`          |
| `schemas is required: pass the JSON Schema text for each document kind, e.g. {"formSchema":"…","uiSchema":"…"}.` | `schemas` missing or not an object                  |
| `schemas.formSchema is required to validate this request.`                                                       | A needed entry is absent or not a string            |
| `Invalid form schema definition: …`                                                                              | A schema in `schemas` is not valid JSON             |
| `Invalid form schema: …`                                                                                         | The document is not valid JSON, or fails the schema |

An empty string counts as present, so `{"formSchema":""}` satisfies the
requirement and then fails as a non-object schema.

## colander_content_hash

Hashes a form/ui/rules triple **as given**, with no compilation. Use it to
compare two stored documents byte-for-byte, or to reproduce the hash
`colander_compile` reports.

### Request and response

| Request key       | Type   | Required |
| ----------------- | ------ | -------- |
| `formSchemaJson`  | string | yes      |
| `uiSchemaJson`    | string | no       |
| `rulesSchemaJson` | string | no       |

`result` is exactly `{"contentHash":"<64 lowercase hex chars>"}`.

Unlike the other entry points, this one parses the documents with the general
JSON parser, so a document may be any JSON value, not only an object.

### What is hashed

The exact string:

```
{"form":<form>,"ui":<ui|null>,"rules":<rules|null>}
```

with these rules:

- The three keys are always present and always in the order `form`, `ui`,
  `rules` — **document key order, not sorted**.
- A missing document is written as the bare token `null`.
- Each document is serialized in its own document order, with number literals
  preserved exactly: `1.50` stays `1.50` and `1e3` stays `1e3`.
- Strings are escaped by the writer's rules — ASCII letters, digits and
  punctuation stay literal, `"` becomes `\u0022`, and everything below U+0020 or
  at/above U+007F becomes `\uXXXX` uppercase, with surrogate pairs above U+FFFF.
- The digest is SHA-256 over the UTF-8 bytes of that string.

So `{"b":1,"a":1.50}` with a UI of `{"z":"é"}` and no rules hashes the payload:

```
{"form":{"b":1,"a":1.50},"ui":{"z":"\u00E9"},"rules":null}
```

Because key order and number spelling are part of the payload, two documents
that differ only in whitespace hash the same, but two that differ in key order
do not. Compile first if you want order-insensitive hashing.

The canonical form and the number-output rules are described in
[json.md](json.md).

## colander_next_version

| Request key | Type             | Required |
| ----------- | ---------------- | -------- |
| `published` | array of strings | no       |

`result` is exactly `{"next":"1.2.4"}`.

With nothing published — or with `published` absent — the answer is `"1.0.0"`.
Otherwise colander parses every entry, takes the highest, and increments **only
its patch**: `["1.2.3","1.10.0","1.9.9"]` yields `"1.10.1"`. Major and minor
never change, and there is no carry. Ties keep the last maximal entry rather
than the first.

A present `published` must be an array of strings; any other JSON type is
rejected. Every entry is parsed, so one invalid entry fails the whole call with
`Invalid semantic version: <entry>`.

### Accepted version syntax

Split on `.`, trim each segment, drop empty ones, then require **exactly three**
non-negative integers:

| Input                                 | Result                                  |
| ------------------------------------- | --------------------------------------- |
| `1.2.3`                               | {1, 2, 3}                               |
| `1 . 2 . 3`                           | {1, 2, 3}                               |
| `01.0.0`                              | {1, 0, 0}                               |
| `-0.0.0`                              | {0, 0, 0}                               |
| `1..0.0`                              | accepted — the blank segment is dropped |
| `1.0`                                 | rejected                                |
| `1.0.0.0`                             | rejected                                |
| `1.-1.0`                              | rejected                                |
| `1.0.0-beta`, `1.0.0+build`, `v1.0.0` | rejected                                |

## Version and utility functions

The remaining six exported symbols do not carry a JSON request in the usual
sense. Three report on the library itself, one releases a returned string, and
the allocator pair moves memory. The ownership and escaping rules they rely on
are covered in depth in [abi.md](abi.md).

### colander_version_info

`colander_version_info()` takes **no argument** and cannot fail:

```json
{"ok":true,"result":{"name":"colander","version":"0.1.0","abi":1}}
```

### colander_abi_version

`colander_abi_version()` returns the ABI version as a plain `uint32_t` (currently
`1`), with no JSON and no allocation. Check it at startup and refuse to run on a
mismatch.

```c
if (colander_abi_version() != 1) { /* refuse to bind */ }
```

### colander_free_string

`colander_free_string(pointer)` releases a string this library returned, and is a
no-op on NULL.

### The allocator pair

`colander_alloc(length)` and `colander_free_buffer(pointer, length)` exist because
ownership of the _output_ is not enough: a caller that cannot allocate inside the
library's heap also has to _produce_ the input. Any embedder without a shared
allocator is that shape.

```c
char *buffer = colander_alloc(request_length + 1);
memcpy(buffer, request, request_length);
buffer[request_length] = '\0';
char *response = colander_compile(buffer);
colander_free_buffer(buffer, request_length + 1);
/* … use response, then */
colander_free_string(response);
```

The pair is the whole contract: there is no way to recover the size from the
pointer, so the length you asked for is the length you pass back. `colander_alloc`
returns NULL for a zero length or an unrepresentable layout, so a non-NULL result
is always safe to write to; freeing NULL or a zero length is a no-op.

A native caller can ignore both and pass a buffer it allocated itself.

### Exports the generated header does not declare

`include/colander.h` is generated by cbindgen from `cbindgen.toml`, whose
`[export] include` list names only nine functions. The library exports eleven:
`colander_alloc` and `colander_free_buffer` are built and exported by the shared
library but are **not declared in the generated header**. A C caller that uses
either must declare it before use, matching the ABI:

```c
uint8_t *colander_alloc(size_t length);
void colander_free_buffer(uint8_t *pointer, size_t length);
```

The other nine are declared in `include/colander.h`. The full reasoning is in
[abi.md](abi.md#the-generated-header-does-not-declare-every-export).

## Next

- The three document schemas: [documents.md](documents.md)
- The wire contract behind every response: [abi.md](abi.md)
- Rule operators and analysis errors: [rules.md](rules.md)
- Response validation and error codes: [validation.md](validation.md)
