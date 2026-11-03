# Rules

Reference for the rule expression language: every operator, its result, how
operands compare and convert, and the structural errors a rules document can
carry. Read it when authoring a `visibleWhen`, `enabledWhen`, `requiredWhen`,
`calculate` or `assert` expression.

Evaluation order is in [documents.md](documents.md#rule-evaluation-order); this
page covers the expressions.

## Operators

| Operator                   | Arity | Semantics                                                                                      |
| -------------------------- | ----- | ---------------------------------------------------------------------------------------------- |
| `eq`                       | 2     | Equal                                                                                          |
| `neq`                      | 2     | Not equal                                                                                      |
| `gt`                       | 2     | Greater                                                                                        |
| `gte`                      | 2     | Greater or equal                                                                               |
| `lt`                       | 2     | Less                                                                                           |
| `lte`                      | 2     | Less or equal                                                                                  |
| `and`                      | 0+    | `true` when every argument is truthy; **no arguments → `true`**                                |
| `or`                       | 0+    | `true` when any argument is truthy; **no arguments → `false`**                                 |
| `not`                      | 1     | Negates its argument; **no arguments → error**                                                 |
| `empty`                    | 1     | `true` when its argument is empty                                                              |
| `coalesce`                 | 0+    | First non-empty argument, else `null`                                                          |
| `add`, `sub`, `mul`, `div` | 2     | Arithmetic; either operand empty → `null`; a non-finite result → `null`                        |
| `count`                    | 1     | Row count of a repeater: `{"op":"count","args":[{"ref":"rep"}]}`                               |
| `sum`                      | 2     | Sum of a child across a repeater's rows: `{"op":"sum","args":[{"ref":"rep"},{"ref":"child"}]}` |

### Row scope and aggregates

A `calculate` on a repeater child runs once per row, in a scope holding that row
over the outer values; each result is written back into its row, so a later
calculation or aggregate observes it. `calculatedValues` for the child holds one
entry per row, which a plain reference observes.

The flat values carry only the repeater's row count, never the rows. A `ref` to
an empty repeater is `0` (falsy) on every entry point, and an N-row calculation
does not clone the row payload per row (SPEC R-13). A calculation chaining from
a sibling calculated child reads that child's value **for the current row**;
outside row scope a reference observes the whole-column array. Keeping per-row
arrays out of the row template makes chained per-row calculations linear (SPEC
R-14).

`count` returns the row count (0 with none). `sum` adds a child across rows as
numbers, treats a missing or non-numeric child as 0, and is `null` with no rows.
Accumulation is compensated, so small decimals do not drift in their last
digits, but the total stays row-order sensitive: floating-point addition is not
associative (SPEC R-15).

Per-row `visibility`, `enabled`, `required` and validations, index addressing,
and other aggregates are out of scope.

## Operands and arithmetic

Operands are checked before evaluation (SPEC R-12). Fixed-arity operators (`eq`,
`neq`, `gt`, `gte`, `lt`, `lte`, `add`, `sub`, `mul`, `div`, `count`, `sum`)
take exactly their arity; a wrong count is `RULE_INVALID_EXPRESSION_ARITY`, not
a silently ignored tail. `and`, `or` and `coalesce` fold over their whole list;
`not` and `empty` take one argument.

Arithmetic produces a double whenever a double holds the result exactly: `add(1, 2)` yields `3.0`, serialized as `3`; it produces an exact integer only when a
double would round it away. `count` yields an integer; `sum` always a double,
spelled without decimals when whole. An aggregate's **first** argument names a
repeater; `sum` then names the child column across its rows. A child code in
first position is `RULE_AGGREGATE_NOT_REPEATER`, where it used to evaluate to a
silent `0`.

`fields` and `validations` must be an object and an array when present. A
malformed container is `RULE_FIELDS_NOT_OBJECT` or `RULE_VALIDATIONS_NOT_ARRAY`,
not a document that quietly applies no rules.

## Comparison

Comparisons use this ordering:

- `null` sorts below every other value; two `null`s are equal.
- Two strings compare lexically.
- `NaN` sorts below every non-`NaN` number; two `NaN`s are equal.
- `-0.0` equals `0.0`.
- Anything else compares as a number, so `"5"` and `5` are equal; a list, row
  set or raw object on either side is an error.
- Two numbers within the shared absolute tolerance `1e-6` compare **equal** on
  every operator: `eq(1, 1.0000005)` is `true`, and so is `lt(1, 1.0000005)`.
  This is the equality the calculated-value check uses (SPEC V-8).
- **Integers are compared exactly, never through `f64`.** Two integers compare
  as integers, and an integer compares against an integral double through
  `i128`. Above 2^53 a double cannot represent every integer:
  `eq(9007199254740993, 9007199254740992)` is `false`, as is comparing
  `i64::MAX` with the nearest double (`2^63`). Tolerance applies to the double
  paths only.
- **Integer arithmetic is exact too.** `add`, `sub` and `mul` over two integers
  compute in `i128`: `add(9007199254740993, 1)` is `9007199254740994`, and
  `add(9223372036854775806, 1)` is `i64::MAX`, not `9.223372036854776e+18`. The
  result _type_ changes only where the old value was wrong: an integer a double
  holds exactly still comes back as a double, and only a value a double would
  round stays an integer. `div` truncates toward zero; division by zero is
  `null`.

## Truthiness, emptiness and conversion

Truthiness (`and`, `or`, `not`, and every `when`/`visibleWhen`/… predicate):
`null` and `false` are false, a non-empty string is true, a non-zero number is
true, and a list, row set or raw object is always true.

Emptiness (`empty`, `coalesce`): `null`, a non-finite number, `""`, `[]` and no
rows are empty; `false`, `0` and a raw object never are. The asymmetry matters:
an empty list or row set is empty **and** truthy, so `and([])` is `true` while
`empty([])` is also `true`. Test emptiness explicitly rather than relying on
truthiness for collections.

Numeric conversion, used by comparisons and arithmetic: `null` → `0`, `true` →
`1`, `false` → `0`, a string is parsed (accepting `Infinity`, `+inf`, `NaN` and
friends, after trimming), and a list, row set or raw object is an error.
Division by zero therefore yields `null`, not an error.

## Shape errors

An unsupported operator, a missing `op`, or missing `args` aborts the call with
a `validation` error, e.g. `Unsupported expression operator 'pow'.` or
`Expression 'eq' requires at least 2 argument(s).`

Any expression failure fails the whole call: there is no partial evaluation
result.

## Row scope

A repeater child's code has no meaning outside a row. The flat value map keeps
only the last row per child code, so reading a child code from a predicate, a
cross-field validation, or a calculation outside that repeater is rejected with
`RULE_INVALID_ROW_REFERENCE` instead of observing an arbitrary row. The two
legal positions are a `calculate` on a child of the same repeater (sibling codes
resolve to the current row) and the aggregates `count`/`sum`.

## Analysis errors

These are emitted whenever a form and rules pair is read: by
`colander_validate_schema` with `kind:"form"` and a `rulesSchemaJson`, and by
`colander_evaluate_rules`, `colander_validate_response` and `colander_compile`
whenever a rules document is present. They are structural problems, not user
errors.

`colander_validate_response` and `colander_compile` treat an absent or
whitespace-only `rulesSchemaJson` as no rules, so the check does not run;
`colander_evaluate_rules` always requires a rules document.

The check includes the duplicate-`code` rejection the code-keyed index performs,
on the effective documents and with or without a rules document, in addition to
the messages below.

| Message                                                                                                                                     | Cause                                                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `RULE_DUPLICATE_FIELD_CODE: duplicate field code 'x'.`                                                                                      | Two fields share a `code`                                                                                                            |
| `RULE_SCHEMA_VERSION_MISMATCH: rules formSchemaVersion 'x' does not match form schemaVersion 'y'.`                                          | The two versions are both present and differ                                                                                         |
| `RULE_UNKNOWN_FIELD: rules reference unknown form field id 'f9' at /fields/f9.`                                                             | A `fields` key is not a form field id                                                                                                |
| `RULE_UNKNOWN_FIELD_REF: expression at /fields/f1/visibleWhen references unknown field code 'x'.`                                           | An expression names a field code no field has                                                                                        |
| `RULE_CALCULATE_NOT_READONLY: calculated field 'f1' at /fields/f1/calculate must be readOnly in the form schema.`                           | A `calculate` on a field without `readOnly: true`                                                                                    |
| `RULE_SELF_REFERENCE: calculated field 'f1' at /fields/f1/calculate must not reference its own code 'total'.`                               | A calculation reading its own field                                                                                                  |
| `RULE_CYCLIC_DEPENDENCY: calculated fields contain a cyclic dependency.`                                                                    | Calculations form a cycle                                                                                                            |
| `RULE_DUPLICATE_VALIDATION_CODE: validation code 'X' at /validations/2/code is duplicated.`                                                 | Two validations share a `code`                                                                                                       |
| `RULE_UNKNOWN_RULE_KEY: rules for field 'f1' at /fields/f1/visiblewhen use an unknown rule key 'visiblewhen'.`                              | A `fields` entry carries a key other than `visibleWhen`, `enabledWhen`, `requiredWhen` or `calculate`                                |
| `RULE_INVALID_ROW_REFERENCE: expression at /validations/0/assert references repeater-child code 'x' outside the row scope of repeater 'r'.` | A predicate, validation or unrelated calculation reads a repeater child                                                              |
| `RULE_INVALID_EXPRESSION: …`                                                                                                                | A malformed node: not a `ref`/`lit`/`op` object, an empty or non-string `ref`, a `lit` combined with `op`/`args`, or a null argument |
| `RULE_INVALID_EXPRESSION_ARITY: expression 'eq' at /fields/f1/visibleWhen has 3 argument(s), expected exactly 2.`                           | A fixed-arity operator gets the wrong argument count                                                                                 |
| `RULE_UNSUPPORTED_EXPRESSION_OPERATOR: expression at /fields/f1/visibleWhen uses unsupported operator 'pow'.`                               | An operator outside the fixed set                                                                                                    |
| `RULE_INVALID_VALIDATION: /validations/0 is missing a string 'code'.`                                                                       | A validation is not an object or has no `code`                                                                                       |
| `RULE_UNKNOWN_VALIDATION_KEY: validation at /validations/0 carries unknown key 'k'.`                                                        | A `validations` entry has a key other than `code`, `when`, `assert` or `message`                                                     |
| `RULE_MISSING_ASSERT: validation at /validations/0 has no 'assert' to evaluate.`                                                            | A `validations` element has no `assert`                                                                                              |

## See also

- Response validation and every error code: [validation.md](validation.md)
- The document schemas and rule evaluation order: [documents.md](documents.md)
- Full entry-point reference: [entry-points.md](entry-points.md)
