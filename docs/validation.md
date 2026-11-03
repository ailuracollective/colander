# Response validation

How to accept a submission with `colander_validate_response`, and what every
code in `errors[]` means. Request keys, error order and `errors[].path` live
with the entry point in
[entry-points.md](entry-points.md#colander_validate_response).

## Draft versus Complete

|                                             | Draft               | Complete                                |
| ------------------------------------------- | ------------------- | --------------------------------------- |
| Type, format, constraint and choice errors  | reported            | reported                                |
| `REQUIRED_FIELD_MISSING`                    | not reported        | reported                                |
| `REPEATER_MIN_ITEMS` / `REPEATER_MAX_ITEMS` | not reported        | reported                                |
| `CALCULATED_VALUE_MISMATCH`                 | not reported        | reported                                |
| Calculated field required and empty         | not reported        | reported                                |
| Failed cross-field validations              | **not** in `errors` | appended with path `/rules/validations` |

Use `Draft` for autosave, `Complete` for submit.

## Validating one page

Send `"mode":"Complete"` with `"pageId":"<id>"`, the id of a top-level
`type:"page"` layout node, to check only the fields that page's `children` name
— a wizard's pre-advance check. Outside the page, required, type, constraint,
repeater-count and calculated-value checks do not run; cross-field `validations`
are deferred, since they may read fields not yet reached;
`normalizedAnswersJson` holds only the page's accepted fields; unknown answer
keys are still reported.

The final submission must be `Complete` **without** `pageId`: a page check is a
navigation aid, not acceptance. `pageId` with `Draft` is
`UI_PAGE_REQUIRES_COMPLETE`; an id the layout lacks is `UI_UNKNOWN_PAGE`;
without `uiSchemaJson` there is no page to find.

A page is a layout node, never a field: no `code`, never an answer key. Its
`children` hold the field nodes it renders, groups and repeaters included. Pages
are flat in `layout` order. With `component-ref` fields, pass the compiled
triple: an unexpanded reference node has no page children.

## Error codes

| `code`                        | Reported when                                                                                                                                                                                                                                                 |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `UNKNOWN_FIELD`               | An answer key matches no field code, or a repeater row carries a key that is not one of its children.                                                                                                                                                         |
| `INVALID_TYPE`                | A value has the wrong JSON type for its field, a repeater is not an array, or a row is not an object.                                                                                                                                                         |
| `UNSUPPORTED_FIELD_TYPE`      | The field's `type` is not one of the validated types.                                                                                                                                                                                                         |
| `INVALID_SCHEMA`              | A `choice` field has no `options` or an empty `options` array.                                                                                                                                                                                                |
| `CONSTRAINT_VIOLATION`        | A length, pattern, bound, `multipleOf` or choice-membership constraint failed.                                                                                                                                                                                |
| `DISABLED_FIELD_VALUE`        | A value was submitted for a currently disabled field.                                                                                                                                                                                                         |
| `HIDDEN_FIELD_VALUE`          | A value was submitted for a currently hidden field.                                                                                                                                                                                                           |
| `READONLY_FIELD_MODIFIED`     | A value was submitted for an enabled read-only field.                                                                                                                                                                                                         |
| `REQUIRED_FIELD_MISSING`      | In Complete mode, a required field is empty.                                                                                                                                                                                                                  |
| `CALCULATED_VALUE_MISMATCH`   | In Complete mode, a submitted calculated value differs from the computed one; integers and doubles compare numerically with tolerance 1e-6, the same equality `eq` uses. Applies to repeater children per row.                                                |
| `VALIDATION_ERRORS_TRUNCATED` | More than 100 errors were collected; the last entry carries the total.                                                                                                                                                                                        |
| `REPEATER_MIN_ITEMS`          | In Complete mode, fewer rows than `minItems`.                                                                                                                                                                                                                 |
| `REPEATER_MAX_ITEMS`          | In Complete mode, more rows than `maxItems`.                                                                                                                                                                                                                  |
| `CALCULATED_VALUE_INVALID`    | In Complete mode, a calculated value is not finite when stored, a finite value overflows while rounding to the field's decimal places, or the computed value violates its own field's type or constraints (`maximum`, `minimum`, `multipleOf`, `pattern`, …). |

Emptiness for `required`: absent keys, `null`, `""`, `[]` and `{}` are empty;
`0` and `false` are **not**. `requiredWhen` overwrites the schema default both
ways, so `requiredWhen: false` unsets `required: true`, symmetric with
`visibleWhen` and `enabledWhen`.

A calculated field is server-authored: read-only and hidden gates do not apply.
A client value is checked against the calculation, not rejected as read-only,
and normalized even when hidden, so it reports `CALCULATED_VALUE_MISMATCH`,
never `READONLY_FIELD_MODIFIED` or `HIDDEN_FIELD_VALUE`.

Normalization is sparse: an empty scalar is omitted from `normalizedAnswersJson`
rather than stored as `null` or `""`, so absent and empty stay indistinguishable
downstream. Repeaters always serialize, even with zero rows (`"items": []`).

Constraint messages name only the first violation on a field:

| Constraint                | Message                                                                   |
| ------------------------- | ------------------------------------------------------------------------- |
| `minLength` / `maxLength` | `Field 'x' must be at least N characters.` / `at most`                    |
| `pattern`                 | `Field 'x' does not match the required pattern.`                          |
| `minimum` / `maximum`     | `Field 'x' must be greater than or equal to N.` / `less than or equal to` |
| `multipleOf`              | `Field 'x' must be a multiple of N.`                                      |

`pattern` is unanchored, follows ECMA-262 as far as `fancy-regex` supports
(look-around and backreferences included), and an uncompilable pattern fails the
call instead of non-matching. `multipleOf` is checked with absolute tolerance
1e-6 on the quotient; `multipleOf: 0` is ignored.

## See also

- Behaviour worth knowing: [gotchas.md](gotchas.md)
- The documents being validated against: [documents.md](documents.md)
- The rule operators behind cross-field checks: [rules.md](rules.md)
