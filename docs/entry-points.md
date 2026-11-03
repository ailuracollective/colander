# Entry points

Reference for the document operations: `colander_compile`,
`colander_describe_form`, `colander_evaluate_rules` and
`colander_validate_response`. The payload, version and memory symbols are in
[entry-points-payload.md](entry-points-payload.md).

Every `char *`-returning function returns the [abi.md](abi.md) envelope:
`{"ok":true,"result":{…}}` on success,
`{"ok":false,"error":{"kind":"…","message":"…"}}` on failure. Check `ok` before
reading `result`; none returns NULL.

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

Two conventions apply:

- `colander_version_info` and `colander_abi_version` take no request argument;
  the allocator pair moves memory, not JSON.
- A key **present with the wrong JSON type is rejected**, required or not:
  absent means "use the default", explicit `null` counts as the wrong type. The
  check reaches nested request objects, so a `components[]` entry's
  `uiSchemaJson` or `contentHash` of the wrong type fails the call (SPEC C-8);
  `{"contentHash": 7}` does not compile with an empty hash.

## colander_compile

Expands `component-ref` fields into groups, canonicalizes the three documents and
hashes the result. Run it before storing or publishing.

### Request

| Key               | Type   | Required | Notes                                                      |
| ----------------- | ------ | -------- | ---------------------------------------------------------- |
| `formSchemaJson`  | string | yes      | JSON text of the form schema                               |
| `uiSchemaJson`    | string | no       | JSON text of the UI schema                                 |
| `rulesSchemaJson` | string | no       | JSON text of the rules schema                              |
| `components`      | array  | no       | Resolved component versions; must be an array when present |

Each `components` entry is an object:

| Key              | Type   | Required |
| ---------------- | ------ | -------- |
| `code`           | string | yes      |
| `version`        | string | yes      |
| `formSchemaJson` | string | yes      |
| `uiSchemaJson`   | string | no       |
| `contentHash`    | string | no       |

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

All three documents are serialized canonically: object keys sorted by UTF-8 byte
order, no whitespace, number literals preserved.

### What compilation produces

- The form schema becomes `{"schemaVersion":…,"$schema":…,"fields":[…]}`:
  `schemaVersion` is copied through, `null` if absent; `$schema` only when
  present and non-null; every other top-level key is dropped.
- Each field is cloned as-is except `items`, compiled recursively. No `id` is
  **generated**; `code` and `type` are untouched. `compile` requires neither `id`
  nor `code` and never checks `type` against the known list: an unknown type
  survives compilation and fails at response validation.
- The UI schema keeps `schemaVersion`, `formSchemaVersion`, `fields`, `$schema`
  and `layout`. Its `fields` object is always emitted (empty if absent); keys are
  sorted, then every resolved component's `fields` are merged in, ordered by
  component code, skipping `null` values.
- The rules schema keeps `schemaVersion`, `formSchemaVersion`, `fields` (sorted),
  `$schema` and `validations`.
- A supplied rules schema is dependency-checked before `dependencyMetadataJson`
  is built, against the compiled form and rules, so a component's fields are
  visible to references; see [rules.md](rules.md).

`dependencyMetadataJson` is the object:

```json
{"components":[{"code":"…","version":"…","contentHash":"…"}],"rules":{"calculatedFieldIds":["…"],"evaluationOrder":["…"]}}
```

`components` is always present (empty with no references), sorted by `code`, then
version, with `contentHash` set to the empty string when the caller supplied none;
an empty string is carried unverified, a non-empty pin is checked first. `rules`
appears only when a rules schema was compiled; `calculatedFieldIds` is in document
order, `evaluationOrder` the topological order calculations run in. The metadata
is **not** part of the content hash.

### Component references

A field of `type: "component-ref"` is replaced by a group:

```json
{"id":"…","code":"…","type":"group","items":[ …the component's compiled fields… ],"required":…,"readOnly":…,"description":…}
```

`required`, `readOnly` and `description` are copied only when present and
non-null. **Every other property of the reference field is dropped.**
`componentCode` is required and non-empty; `componentVersion` must be present,
non-blank and valid semver. A cycle is an error keyed on `(code, version)`: the
same code at another version is another triple, not a cycle. Nesting is bounded
at 64 levels (`COMPONENT_DEPTH_EXCEEDED`); total expansion is bounded by the P-8
budget regardless of depth.

The field's UI layout node becomes a group node whose `children` come from the
component's own UI `layout`; without one, colander builds the default
`[{"type":"field","fieldId":"<id>"}, …]` from the component's fields.

Components resolve by exact `(code, version)` match against the `components`
batch and memoize: one metadata entry and one compiled field array per version.
The first reference expands the component; later ones clone the compiled fields
(SPEC P-8), and a pin is verified once per call, not per site (SPEC P-9).
Expansion is bounded at 1,000,000 field nodes and 256 MiB of materialised
output, so an exponential graph fails with `COMPONENT_BUDGET_EXCEEDED` rather
than exhausting memory.

Each resolved component is pinned by its `contentHash` (SPEC P-3). A non-empty
string makes colander compile the component on its own — nested `component-ref`
fields expanded from the same batch, no rules document — and compare the digest;
a mismatch fails with `COMPONENT_HASH_MISMATCH`. An absent key or an empty string
is **not** a pin: the component compiles and is carried into
`dependencyMetadataJson` unverified. The pin covers exact bytes, so `1.50` and
`1.5` hash differently.

### Errors

| Message                                                                                                                  | Cause                                                                            |
| ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `'formSchemaJson' is required and must be a string.`                                                                     | Missing or non-string required key                                               |
| `JSON_NOT_OBJECT: Invalid form schema: expected a JSON object.`                                                          | Valid JSON that is not an object                                                 |
| `COMPILE_EXPECTED_ARRAY: expected an array at /fields.`                                                                  | The form has no `fields` array                                                   |
| `FIELD_NOT_OBJECT: /fields/0 must be an object.`                                                                         | A field is not an object                                                         |
| `COMPILE_EXPECTED_STRING: expected a non-empty string at /fields/0/type.`                                                | A field has no `type`                                                            |
| `COMPONENT_INVALID_ENTRY: components[] entries must be objects.`                                                         | A `components` element is not an object                                          |
| `COMPONENT_MISSING_KEY: components[].code is required.`                                                                  | Missing `code` (same for `version`, `formSchemaJson`)                            |
| `COMPONENT_UNKNOWN_KEY: components[] entry carries unknown key 'k'.`                                                     | An unknown key in a `components` entry                                           |
| `COMPONENT_BUDGET_EXCEEDED: expansion materialised more than 1000000 fields.`                                            | The expansion budget is exhausted (wide or fan-out reference graphs)             |
| `COMPONENT_VERSION_REQUIRED: component-ref at /fields/0 must include componentVersion before publication.`               | Blank or absent `componentVersion`                                               |
| `COMPONENT_VERSION_NOT_FOUND: component 'x' version '1.0.0' referenced at /fields/0 was not found or is not published.`  | No matching entry in `components`                                                |
| `CIRCULAR_COMPONENT_REFERENCE: component 'x' references itself through a -> b -> x.`                                     | A component reference cycle                                                      |
| `COMPONENT_HASH_MISMATCH: component 'x' version '1.0.0' declares contentHash '…' but its compiled triple hashes to '…'.` | A non-empty `contentHash` pin does not match the component's own compiled triple |
| `INVALID_SEMVER: invalid semantic version: 1.0`                                                                          | Bad `componentVersion`                                                           |
| `UI_UNKNOWN_KEY: UI schema carries unknown top-level key 'k'.`                                                           | An unknown top-level UI key (also for layout nodes)                              |
| `FIELD_MISSING_KEY: field id at /fields/0/id is required.`                                                               | A field lacks a string `id` or `code`                                            |
| `FIELD_INVALID_TYPE: /fields/0/id is Number, expected String.`                                                           | A field key has the wrong JSON type                                              |
| `REQUEST_TOO_LARGE: request is N bytes, over the 67108864-byte limit.`                                                   | A request over the FFI byte cap (C-10)                                           |

## colander_describe_form

Reports the field index of a compiled triple: every field's identifiers and flags,
for rendering a form and for mapping an `errors[].path` back to a field. It
compiles first, so a `component-ref` is reported by its expansion and the
returned `contentHash` describes exactly what was described.

Use it instead of reparsing the compiled `formSchemaJson`: the pointers and flags
come from the index the core built when validating the document, so they cannot
drift from those `colander_validate_response` reports.

### Request

The same keys as `colander_compile`, with the same requirements.

### Response

```json
{"fields":[{"id":"…","code":"…","path":"/fields/0","parentPath":null,
            "type":"text","required":false,"readOnly":false}],
 "contentHash":"<hex>"}
```

| Key           | Notes                                                                  |
| ------------- | ---------------------------------------------------------------------- |
| `fields`      | Every field, in document order, including groups and repeater children |
| `contentHash` | The hash of the compiled triple that was described                     |

Each entry carries:

| Key          | Notes                                                                     |
| ------------ | ------------------------------------------------------------------------- |
| `id`         | How the rules and the UI reference the field                              |
| `code`       | The code the document declares; present for every valid field             |
| `path`       | JSON pointer to the field, the same string an `errors[].path` points at   |
| `parentPath` | The container's pointer, `null` for a top-level field                     |
| `type`       | One of the twelve field type names                                        |
| `required`   | The schema baseline, not a rule evaluation                                |
| `readOnly`   | The schema baseline that `colander_evaluate_rules` derives `enabled` from |

A group's `code` is reported like any other; whether it is an answer key follows
from its `type`: a group is a container, not a value. Presentation is not
described: a field's `title` and `description` are inert by
[documents.md](documents.md), and a layout node's title travels in the compiled
`uiSchemaJson`.

`visibility`, `enabled`, `required` after rules and calculated values come from
`colander_evaluate_rules`; the schema baseline is not a rule evaluation.

### Errors

Every error `colander_compile` can produce, unchanged. A document the core
refuses yields a failure envelope, never a partial index.

## colander_evaluate_rules

Evaluates rules against a set of values: which fields are visible, enabled or
required, what calculated fields evaluate to, and which cross-field validations
currently fail.

### Request

| Key               | Type   | Required | Notes                                           |
| ----------------- | ------ | -------- | ----------------------------------------------- |
| `formSchemaJson`  | string | yes      | JSON text; must parse to an object              |
| `rulesSchemaJson` | string | yes      | JSON text; must parse to an object              |
| `uiSchemaJson`    | string | no       | Only `fields.<id>.hidden` is read               |
| `values`          | object | no       | Current answers; must be an object when present |

The **keys of `values` are field codes**, not field ids. An entry for a repeater
code may be a number (a row count) or an array of row objects, supplying that
repeater's rows for per-row calculation (SPEC R-7a).

### Response

| Key                | Type                       | Keyed by       | Notes                                                                                              |
| ------------------ | -------------------------- | -------------- | -------------------------------------------------------------------------------------------------- |
| `visibility`       | object of bool             | **field id**   | `false` when the UI schema marks the field `hidden`                                                |
| `enabled`          | object of bool             | **field id**   | `false` for a `readOnly` field, unless a rule overrides it                                         |
| `required`         | object of bool             | **field id**   | From the form schema, unless a rule overrides it                                                   |
| `calculatedValues` | object                     | **field code** | Only fields with a `calculate` expression; a repeater child's value is an array, one entry per row |
| `validationErrors` | array of `{code, message}` | —              | Cross-field validations that failed                                                                |

Every field appears in the three boolean maps, whether or not it has a rule. Only
fields with rules can change.

A validation entry contributes an error when its `assert` expression evaluates
falsy. Its `when` guards it: if `when` is present and falsy, the entry is
skipped. An entry with no `assert` never reaches evaluation; the analyzer rejects
it, whether or not it carries `when`. An entry with no `code` is reported as
`VALIDATION_<position>`, and one with no `message` as `"Validation failed."`.

A supplied `rulesSchemaJson` is dependency-checked before evaluation: the
`RULE_*` analysis errors in [rules.md](rules.md) abort the call.

### Errors

The `RULE_*` analysis errors, plus expression failures raised during evaluation.

## colander_validate_response

Validates submitted answers and returns them normalized.

### Request

| Key               | Type   | Required | Notes                                            |
| ----------------- | ------ | -------- | ------------------------------------------------ |
| `formSchemaJson`  | string | yes      | JSON text; must parse to an object               |
| `answersJson`     | string | yes      | JSON text; must parse to an **object**           |
| `uiSchemaJson`    | string | no       | `hidden`, plus `layout` pages for `pageId`       |
| `rulesSchemaJson` | string | no       | Absent or whitespace-only is treated as no rules |
| `mode`            | string | no       | `"Draft"` (default) or `"Complete"`              |
| `pageId`          | string | no       | Validate one page; requires `"mode":"Complete"`  |

When `rulesSchemaJson` is absent or blank, colander synthesizes an empty rules
document pinned to the form's `schemaVersion` (or `1.0.0` if the form has none),
so no version-mismatch error can occur. A present rules document is
dependency-checked exactly like `colander_validate_schema` with `kind:"form"`;
the synthesized one is not.

With `pageId`, the call checks only the fields named by the top-level layout node
of `type:"page"` whose `id` matches, and returns normalized answers for that page
alone; cross-field validations are deferred to the final call. `pageId` with
`Draft` fails with `UI_PAGE_REQUIRES_COMPLETE`, and an id the `layout` does not
carry fails with `UI_UNKNOWN_PAGE`. Unknown answer keys are still reported. The
unscoped `Complete` call remains the authoritative acceptance step; a page check
is a navigation aid.

### Response

| Key                     | Type                             | Notes                                   |
| ----------------------- | -------------------------------- | --------------------------------------- |
| `normalizedAnswersJson` | string                           | Compact JSON object of converted values |
| `errors`                | array of `{code, path, message}` | Empty when the response is valid        |
| `isValid`               | bool                             | `true` exactly when `errors` is empty   |

`normalizedAnswersJson` contains only fields that were accepted and converted,
keyed by field code, in form order; a field that produced an error is omitted.
Repeaters appear as arrays of row objects. With `pageId`, only the page's fields
are present.

`errors[].path` is a JSON pointer to the field's location **in the form schema**:
`/fields/0`, `/fields/0/items/1`, or `/fields/2/1/<childId>` for a repeater row.
Two exceptions: an unknown answer key reports `/answers/<key>`, a failed
cross-field validation `/rules/validations`. What each mode reports, and every
`errors[].code`, is in [validation.md](validation.md).

### Order errors are produced in

1. Unknown top-level answer keys.
2. Scalar fields, in form order.
3. Repeaters, in form order: row count, then each row's children.
4. Calculated fields.
5. Cross-field validations (Complete only, and skipped when `pageId` is present).

Within one field only the first applicable error is reported, and a field that
errors is never normalized. The checks run in this order: hidden/disabled →
read-only → required → type conversion → constraints.

## See also

- The remaining symbols: [entry-points-payload.md](entry-points-payload.md)
- The wire contract: [abi.md](abi.md)
- Response validation and error codes: [validation.md](validation.md)
