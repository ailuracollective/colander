# Rules

This page is the reference for the rule expression language: every operator,
what it evaluates to, how operands are compared and converted, and the
structural errors a rules document can carry. Read it when you are authoring a
`visibleWhen`, `enabledWhen`, `requiredWhen`, `calculate` or `assert`
expression.

The order the engine evaluates rules in is in
[documents.md](documents.md#rule-evaluation-order); this page covers the
expressions themselves.

## Rule operators

| Operator                   | Arity | Semantics                                                               |
| -------------------------- | ----- | ----------------------------------------------------------------------- |
| `eq`                       | 2     | Equal                                                                   |
| `neq`                      | 2     | Not equal                                                               |
| `gt`                       | 2     | Greater                                                                 |
| `gte`                      | 2     | Greater or equal                                                        |
| `lt`                       | 2     | Less                                                                    |
| `lte`                      | 2     | Less or equal                                                           |
| `and`                      | 0+    | `true` when every argument is truthy; **no arguments → `true`**         |
| `or`                       | 0+    | `true` when any argument is truthy; **no arguments → `false`**          |
| `not`                      | 1+    | Negates the first argument; **no arguments → error**                    |
| `empty`                    | 1+    | `true` when the first argument is empty                                 |
| `coalesce`                 | 0+    | First non-empty argument, else `null`                                   |
| `add`, `sub`, `mul`, `div` | 2     | Arithmetic; either operand empty → `null`; a non-finite result → `null` |

Operands beyond the second are evaluated and then ignored, so the binary
operators take **at least** two arguments rather than exactly two. Arithmetic
always produces a double: `add` of the integers `1` and `2` yields `3.0`, which
serializes as `3`.

Comparisons use this ordering:

- `null` sorts below every other value; two `null`s are equal.
- Two strings compare lexically.
- `NaN` sorts below every non-NaN number, and two `NaN`s are equal.
- `-0.0` equals `0.0`.
- Anything else is compared as a number, so `"5"` and `5` compare equal. A list,
  row set or raw object on either side is an error.

Truthiness (`and`, `or`, `not`, and every `when`/`visibleWhen`/… predicate):
`null` and `false` are false, a non-empty string is true, a non-zero number is
true, and a list, row set or raw object is always true.

Emptiness (`empty`, `coalesce`): `null`, a non-finite number, `""`, `[]` and no
rows are empty; `false`, `0` and a raw object never are.

Numeric conversion, used by comparisons and arithmetic: `null` → `0`, `true` →
`1`, `false` → `0`, a string is parsed (accepting `Infinity`, `+inf`, `NaN` and
friends, after trimming), and a list, row set or raw object is an error. Note
that division by zero therefore yields `null`, not an error.

An unsupported operator, a missing `op`, or missing `args` aborts the call with
a `validation` error such as `Unsupported expression operator 'pow'.` or
`Expression 'eq' requires at least 2 argument(s).`

When any expression in a call fails, the whole call fails: there is no partial
evaluation result.

## Rule analysis errors

These are emitted whenever a form and rules pair is read. `colander_validate_schema`
with `kind:"form"` and a `rulesSchemaJson` emits them, and so do
`colander_evaluate_rules`, `colander_validate_response` and `colander_compile`
whenever a rules document is present. They are structural problems, not user
errors.

`colander_validate_response` and `colander_compile` treat an absent or
whitespace-only `rulesSchemaJson` as no rules, so the check does not run then.
`colander_evaluate_rules` always requires a rules document.

The check includes the duplicate-`code` rejection that the code-keyed index
performs, in addition to the messages below.

| Message                                                                                                           | Cause                                             |
| ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| `An item with the same key has already been added. Key: x`                                                        | Two fields share a `code`                         |
| `RULE_SCHEMA_VERSION_MISMATCH: rules formSchemaVersion 'x' does not match form schemaVersion 'y'.`                | The two versions are both present and differ      |
| `RULE_UNKNOWN_FIELD: rules reference unknown form field id 'f9' at /fields/f9.`                                   | A `fields` key is not a form field id             |
| `RULE_UNKNOWN_FIELD_REF: expression at /fields/f1/visibleWhen references unknown field code 'x'.`                 | An expression names a field code no field has     |
| `RULE_CALCULATE_NOT_READONLY: calculated field 'f1' at /fields/f1/calculate must be readOnly in the form schema.` | A `calculate` on a field without `readOnly: true` |
| `RULE_SELF_REFERENCE: calculated field 'f1' at /fields/f1/calculate must not reference its own code 'total'.`     | A calculation reading its own field               |
| `RULE_CYCLIC_DEPENDENCY: calculated fields contain a cyclic dependency.`                                          | Calculations form a cycle                         |
| `RULE_DUPLICATE_VALIDATION_CODE: validation code 'X' at /validations/2/code is duplicated.`                       | Two validations share a `code`                    |
| `Expected validation object at /validations/0.`                                                                   | A `validations` element is not an object          |
| `Expected validation code at /validations/0/code.`                                                                | A `validations` element has no `code`             |

## Next

- Response validation and every error code: [validation.md](validation.md)
- The document schemas and rule evaluation order: [documents.md](documents.md)
- Full entry-point reference: [entry-points.md](entry-points.md)
