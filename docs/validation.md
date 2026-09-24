# Response validation

This page tells you how to accept a submission with `colander_validate_response`
and what every code in its `errors[]` array means. The request and response
keys, the order errors are produced in, and the meaning of `errors[].path` live
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

## Response validation error codes

| `code`                      | Reported when                                                                                                                                                                                               |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `UNKNOWN_FIELD`             | An answer key matches no field code, or a repeater row carries a key that is not one of its children                                                                                                        |
| `INVALID_TYPE`              | A value has the wrong JSON type for its field, a repeater is not an array, or a repeater row is not an object                                                                                               |
| `UNSUPPORTED_FIELD_TYPE`    | The field's `type` is not one of the validated types                                                                                                                                                        |
| `INVALID_SCHEMA`            | A `choice` field has no `options`, or an empty `options` array                                                                                                                                              |
| `CONSTRAINT_VIOLATION`      | A length, pattern, bound, `multipleOf` or choice-membership constraint failed                                                                                                                               |
| `DISABLED_FIELD_VALUE`      | A value was submitted for a field that is currently disabled                                                                                                                                                |
| `HIDDEN_FIELD_VALUE`        | A value was submitted for a field that is currently hidden                                                                                                                                                  |
| `READONLY_FIELD_MODIFIED`   | A value was submitted for a read-only field that is nonetheless enabled                                                                                                                                     |
| `REQUIRED_FIELD_MISSING`    | In Complete mode, a required field is empty                                                                                                                                                                 |
| `CALCULATED_VALUE_MISMATCH` | In Complete mode, a submitted calculated value differs from the computed one (integers and doubles compare numerically, tolerance 1e-6). Applies to repeater children per row, symmetric with scalar fields |
| `REPEATER_MIN_ITEMS`        | In Complete mode, fewer rows than `minItems`                                                                                                                                                                |
| `REPEATER_MAX_ITEMS`        | In Complete mode, more rows than `maxItems`                                                                                                                                                                 |
| `CALCULATED_VALUE_INVALID`  | In Complete mode, a calculated value is not finite when it is stored; a finite value can overflow while rounding to the field's decimal places                                                              |

Emptiness for `required` purposes: an absent key, `null`, `""`, `[]` and `{}` are
empty. `0` and `false` are **not** empty. A `requiredWhen` predicate overwrites
the schema default in both directions: `requiredWhen: false` unsets a schema
`required: true`, symmetric with `visibleWhen`/`enabledWhen`.

Normalization is sparse: an empty scalar answer is omitted from
`normalizedAnswersJson` rather than stored as `null` or `""` — absent and
empty stay indistinguishable downstream. Repeaters always serialize, even
with zero rows (`"items": []`).

Constraint messages name only the first violation on a field:

| Constraint                | Message                                                                   |
| ------------------------- | ------------------------------------------------------------------------- |
| `minLength` / `maxLength` | `Field 'x' must be at least N characters.` / `at most`                    |
| `pattern`                 | `Field 'x' does not match the required pattern.`                          |
| `minimum` / `maximum`     | `Field 'x' must be greater than or equal to N.` / `less than or equal to` |
| `multipleOf`              | `Field 'x' must be a multiple of N.`                                      |

`pattern` is unanchored and follows ECMA-262 as far as the `fancy-regex` engine
supports, look-around and backreferences included; a pattern that cannot be
compiled fails the call instead of being treated as a non-match. `multipleOf` is
checked with an absolute tolerance of 1e-6 on the quotient, and a `multipleOf` of
`0` is ignored.

## Next

- Behaviour worth knowing: [gotchas.md](gotchas.md)
- The entry point that produces these codes: [entry-points.md](entry-points.md#colander_validate_response)
- The documents being validated against: [documents.md](documents.md)
