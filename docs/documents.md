# The documents

This page describes the three JSON documents a form is made of — the form
schema, the UI schema and the rules schema — what each key means, and how the
three fit together. Read it when you are authoring or inspecting a form, UI or
rules document; the mental model behind them is in [concepts.md](concepts.md)
and the call that compiles them is in [entry-points.md](entry-points.md).

## Form schema

```json
{
  "schemaVersion": "1.0.0",
  "$schema": "…",
  "fields": [
    {"id": "f1", "code": "first_name", "type": "text", "required": true, "maxLength": 40},
    {"id": "f2", "code": "tags", "type": "choice", "options": [{"value": "a"}]}
  ]
}
```

| Top-level key   | Type             | Required          | Read by                                                         |
| --------------- | ---------------- | ----------------- | --------------------------------------------------------------- |
| `fields`        | array of objects | yes for `compile` | The field tree                                                  |
| `schemaVersion` | string           | no                | Rules version matching; the UI and rules documents reference it |
| `$schema`       | string           | no                | Copied to compiled output only                                  |

Every field object:

| Key                       | Type    | Required     | Meaning                                                                                                                                          |
| ------------------------- | ------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `type`                    | string  | yes          | See [field types](#field-types). Must be non-empty                                                                                               |
| `id`                      | string  | for indexing | Stable identifier; keys the rule maps. May be empty for `compile` alone                                                                          |
| `code`                    | string  | for indexing | Answer key; keys `values` and `calculatedValues`. Duplicates are always rejected (`RULE_DUPLICATE_FIELD_CODE`), with or without a rules document |
| `required`                | bool    | no           | Default `false`. Seeds `required` in the evaluation                                                                                              |
| `readOnly`                | bool    | no           | Default `false`. Seeds `enabled:false` in the evaluation                                                                                         |
| `items`                   | array   | no           | Children of a `group`, `repeater` or `component-ref`                                                                                             |
| `options`                 | array   | for `choice` | Each `{"value": "…"}`; other keys, including `label`, are ignored                                                                                |
| `allowMultiple`           | bool    | no           | `true` makes a `choice` accept an array                                                                                                          |
| `minLength` / `maxLength` | integer | no           | Counted in **UTF-16 code units**                                                                                                                 |
| `pattern`                 | string  | no           | Unanchored ECMA-262 pattern; an empty string is ignored, an uncompilable one is an error                                                         |
| `minimum` / `maximum`     | number  | no           | Inclusive bounds                                                                                                                                 |
| `multipleOf`              | number  | no           | Step; also derives decimal places for calculations                                                                                               |
| `decimalPlaces`           | integer | no           | Calculation rounding, clamped to 0–10, default 2                                                                                                 |
| `minItems` / `maxItems`   | integer | no           | Repeater row count, enforced in Complete mode only                                                                                               |
| `description`, `title`    | string  | no           | Copied through by `compile`; otherwise inert                                                                                                     |

Any other key is preserved verbatim by `compile` and otherwise ignored.
`title` in particular has **no meaning on a form field** — it is a layout-node
key.

The `id`, `code` and `type` keys are read with two different strictness rules:
`compile` requires only a non-empty `type`, while building the indexes that
rules and validation use requires `id` and `code` to be strings — but accepts
them empty. A `null` is treated as absent, and a wrong type is an error:
`An element of type 'True' cannot be converted to a 'String'.`

## Field types

| `type`          | Answer must be                                                                                                                            | Converted to                              |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `text`          | string                                                                                                                                    | string                                    |
| `textarea`      | string                                                                                                                                    | string                                    |
| `number`        | JSON number                                                                                                                               | double                                    |
| `integer`       | JSON number with no fraction or exponent, fitting `i64` (stricter than the JSON Schema subset's `integer`, which accepts `1.0` and `1e3`) | integer                                   |
| `boolean`       | `true` / `false`                                                                                                                          | boolean                                   |
| `date`          | string, ISO-like date                                                                                                                     | `YYYY-MM-DD`                              |
| `datetime`      | string, ISO-like date-time                                                                                                                | `YYYY-MM-DDTHH:MM:SS.fffffff±HH:MM`       |
| `time`          | string, `HH:MM` or `HH:MM:SS`                                                                                                             | `HH:MM:SS`                                |
| `choice`        | string (or array when `allowMultiple`)                                                                                                    | string (or list)                          |
| `group`         | —                                                                                                                                         | never validated directly; its `items` are |
| `repeater`      | array of row objects                                                                                                                      | array of row objects                      |
| `component-ref` | —                                                                                                                                         | expanded to a `group` by `compile`        |

Anything else yields `UNSUPPORTED_FIELD_TYPE`. The names are matched exactly and
case-sensitively: **`colander` accepts no aliases**. If you want `email`,
`bool`, `dropdown` or `section` to work, the sibling crate `slate-ai` is what
maps them onto these twelve.

### Dates

Split on `-` or `/` into exactly three numbers. The first component is the year
when it is greater than 31; otherwise the last is. So `2024-03-05`, `2024/3/5`,
`03/05/2024` and `3-5-24` all parse. A two-digit year uses the pivot 00–29 →
2000–2029 and 30–99 → 1930–1999. A year above 9999, a month outside 1–12, or a
day outside the month is rejected. Output is always `YYYY-MM-DD`. This is
deliberately more lenient than the `format: date` assertion in
[json-schema.md](json-schema.md), which requires strict RFC 3339: answers are
normalized, schemas are strict.

### Date-times

A date part, then `T` or a space, then a clock. A trailing `Z` or `z` means UTC;
otherwise a trailing offset is read as `±HH:MM`, `±HHMM` or `±HH`, with hours ≤
14 and minutes ≤ 59. **An input without a trailing offset is assumed to be
UTC**, never the machine's local offset. The clock is `HH:MM` or `HH:MM:SS` with
an optional fraction, on a 24-hour clock. Output always carries seven fractional
digits and an explicit offset.

### Times

`HH:MM` or `HH:MM:SS`; a fractional part is accepted and then dropped.

## UI schema

| Top-level key                        | Type   | Meaning                                      |
| ------------------------------------ | ------ | -------------------------------------------- |
| `fields`                             | object | Keyed by field **id**; only `hidden` is read |
| `layout`                             | array  | Presentation tree, rewritten by `compile`    |
| `schemaVersion`, `formSchemaVersion` | string | Copied through by `compile`                  |
| `$schema`                            | string | Copied through when present                  |

A `fields.<id>` entry with `"hidden": true` seeds `visibility[id] = false`.
Entries are otherwise opaque to the core; `label`, `widget` and `width` are
declared vocabulary but are never read.

The recognised widget names are `text-input`, `textarea`, `number-input`,
`integer-input`, `toggle`, `date-picker`, `datetime-picker`, `time-picker`,
`select`, `group` and `repeater` — but the core never validates or interprets
them, so any string passes through.

Layout nodes are `{"type": …, "id": …, "title": …, "description": …,
"fieldId": …, "children": […], "itemTemplate": […], "addButtonLabel": …,
"removeButtonLabel": …}`. `type` is required and non-empty; there is no
enumeration of allowed node types. **Unknown layout keys are dropped.** A node
of `type:"field"` whose `fieldId` names an expanded component reference becomes
a `type:"group"` node with the component's layout as `children`.

## Rules schema

```json
{
  "schemaVersion": "1.0.0",
  "formSchemaVersion": "1.0.0",
  "fields": {
    "f1": {
      "visibleWhen": {"op": "eq", "args": [{"ref": "kind"}, {"lit": "person"}]},
      "enabledWhen": …,
      "requiredWhen": …,
      "calculate": {"op": "add", "args": [{"ref": "a"}, {"ref": "b"}]}
    }
  },
  "validations": [
    {"code": "AGE_RANGE", "when": …, "assert": …, "message": "Age must be 18 or over."}
  ]
}
```

| Top-level key       | Type   | Meaning                                                     |
| ------------------- | ------ | ----------------------------------------------------------- |
| `fields`            | object | Keyed by field **id**                                       |
| `validations`       | array  | Cross-field checks                                          |
| `schemaVersion`     | string | The rules document's own version                            |
| `formSchemaVersion` | string | Must equal the form's `schemaVersion` when both are present |
| `$schema`           | string | Copied through by `compile`                                 |

Each `fields.<id>` entry may carry `visibleWhen`, `enabledWhen`, `requiredWhen`
and `calculate`. Each is a null-or-expression; `null` and an absent key are the
same thing. A rule node that is not an object is silently skipped, but an id
that is not a form field id is a hard error (a `RULE_UNKNOWN_FIELD` analysis
error; see [rules.md](rules.md)).

A `validations` entry is `{"code", "when", "assert", "message"}`. `code` must be
a string and unique across the array, and `assert` is required: an entry without
one is rejected by the analyzer.

## Expressions

An expression is one of:

```json
{"ref": "field_code"}       // the current value of a field, null if unknown
{"lit": <any JSON>}         // a literal
{"op": "eq", "args": [ … ]} // an operation
```

A node with a non-empty `ref` is a reference and nothing else is read from it;
otherwise a `lit` wins over `op`. References resolve against **field codes**.
The operators and their semantics are in [rules.md](rules.md).

## Groups, repeaters and their codes

A `group` is `{"type":"group","items":[…]}`. Its own code is **not** an answer
key: submitting under it reports `UNKNOWN_FIELD`, while its children are
accepted as ordinary top-level answers. No submitted object aborts the call: an
object on a scalar field reports `INVALID_TYPE` from per-field validation.

A `repeater` is `{"type":"repeater","items":[…]}` plus optional `minItems` and
`maxItems`. Its answer is an array of row objects; each row's keys must be its
children's codes. Children must be flat scalar fields: anything with nested
`items` under a repeater is rejected with `REPEATER_NESTED_FIELD`.

## Rule evaluation order

1. Baseline: every field gets `visibility = !hidden` from the UI schema,
   `enabled = !readOnly` from the form schema, and `required` from the form
   schema.
2. Calculations run in topological dependency order. Each result is written to
   `calculatedValues[code]` **and** into the working value set, so later
   calculations and all predicates observe it. A `calculate` on a repeater child
   with rows runs once per row instead, and its array replaces the flattened
   value everywhere downstream (SPEC R-7).
3. `visibleWhen`, `enabledWhen` and `requiredWhen` override the baseline,
   reading the values from step 2 — in both directions, so `requiredWhen:
   false` unsets a schema `required: true`.
4. Cross-field validations run last.

Calculation results are normalized for their field: on an `integer` field the
value is rounded; on a `number` field it is snapped to `multipleOf` and rounded
to the resolved decimal places (explicit `decimalPlaces` clamped to 0–10, else
derived from `multipleOf`, else 2). A non-finite result becomes `null`, and a
non-numeric result passes through unchanged.

## Component references

A form field of `type: "component-ref"` is not a user-visible field: it names a
published component by `componentCode` and `componentVersion`, and
`colander_compile` replaces it with a `group` built from that component's form
schema, wrapping its UI `layout` as the group's `children`. Resolution is by
exact `(code, version)` against the `components` batch in the compile request.

The compile request's `components` shape, the expansion rules, the rewritten
layout node and the reference errors live with `colander_compile` in
[entry-points.md](entry-points.md#component-references).

## Next

- Rule operators and evaluation details: [rules.md](rules.md)
- Every request and response shape: [entry-points.md](entry-points.md)
- The mental model: [concepts.md](concepts.md)
