# Behaviour worth knowing

These are real behaviours, most of them inherited rather than chosen. Each one
has surprised someone.

1. **A multi-select answer fails the whole call.** Before per-field validation,
   every answer is converted to the rule value domain, which rejects arrays and
   objects: a `choice` with `allowMultiple` (or any array answer) makes
   `colander_validate_response` fail with `Nested JSON arrays are not supported as
   answer values.` An object answer — a group or `component-ref` code — fails the
   same way with `Nested JSON objects are not supported as answer values.` Both
   aborts happen before per-field validation, so `INVALID_TYPE` is never
   observable for an array or object answer, and the `allowMultiple` conversion
   path is unreachable through this entry point.
2. **`readOnly` shows up as `enabled: false`.** A read-only field with a
   submitted value reports `DISABLED_FIELD_VALUE`, not
   `READONLY_FIELD_MODIFIED`. The latter only appears when a rule enables a
   read-only field.
3. **`colander_validate_schema` never returns `{"valid":false}`.** A schema failure
   is a failure envelope. Do not look for a `valid` field on the error path.
4. **Duplicate field codes are rejected by the dependency check, which is
   skipped when no rules document is supplied.** Two fields sharing a `code` fail
   every entry point that reads a form and rules pair —
   `colander_evaluate_rules`, `colander_validate_response`, `colander_compile` and
   `colander_validate_schema` with `kind:"form"` — with `An item with the same key
   has already been added. Key: x`. Because `colander_validate_response` and
   `colander_compile` treat absent or blank rules as no rules, the same duplicate
   passes them when `rulesSchemaJson` is absent. Two fields sharing an `id` still
   collapse to a single entry in the rule maps.
5. **An absent `assert` does not fail a validation.** A `validations` entry with
   no `assert` always reports its error; use `when` to guard it.
6. **Repeater row values flatten into the top-level rule values.** A repeater's
   own code is visible to expressions as its row **count**, and each row's
   children are readable by their plain codes — only the last row survives.
7. **Optional keys of the wrong type are ignored.** Required keys are the only
   ones that reject a type.
8. **`published` is accepted and ignored** by `kind:"workflow"`.
9. **`CALCULATED_VALUE_INVALID` is unreachable**, because a non-finite
   calculation becomes `null` first.
10. **Version comparison is not semver-strict.** `1..0.0`, `01.0.0` and
    `"  1.2.3  "` are valid; pre-release and build suffixes are not.
11. **`compile` drops unknown top-level keys** from the form, UI and rules
    documents, and unknown keys from layout nodes, while preserving unknown keys
    inside fields.
12. **A component's `contentHash` is never verified.** colander carries it into
    `dependencyMetadataJson` and trusts it.
13. **The dependency check is skipped when no rules document is supplied.**
    `colander_validate_response` and `colander_compile` treat absent or blank
    rules as no rules, so a malformed rules document cannot reach them; only a
    document that is actually supplied is checked. `colander_evaluate_rules`
    always requires one.
14. **`compile` does not sort or inspect `fields`**; it only expands references
    and recurses into `items`. Canonical serialization sorts object keys, but
    array order is yours to define.

## Next

- What stays hand-written, and why: [dependencies.md](dependencies.md)
- Response validation codes: [validation.md](validation.md)
- The wire contract and its surprises: [abi.md](abi.md)
