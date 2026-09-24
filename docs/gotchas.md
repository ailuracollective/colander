# Behaviour worth knowing

These are real behaviours, most of them inherited rather than chosen. Each one
has surprised someone.

1. **A multi-select answer validates, and one bad answer does not abort the
   call.** A `choice` with `allowMultiple` accepts an array and normalizes it to
   the selected values. Any answer the value domain cannot hold — an object, or
   a nested object inside an array — contributes a missing rule value instead of
   failing the call, and per-field validation reports it as `INVALID_TYPE`. An
   unknown key keeps the `UNKNOWN_FIELD` it collected. (SPEC E-8, E-9, V-6.)
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
5. **An absent `assert` fails the analysis.** A `validations` entry with no
   `assert` is rejected by the analyzer, whether or not it carries `when`,
   because an entry with nothing to assert can never report anything.
6. **Repeater row values flatten into the top-level rule values.** A repeater's
   own code is visible to expressions as its row **count**, and each row's
   children are readable by their plain codes — only the last row survives.
7. **An optional key of the wrong type is rejected.** A present key whose JSON
   type does not match the documented one fails the call, the same as a bad
   required key; it is not silently ignored. "Absent" still means "use the
   default", so an omitted `mode` is `"Draft"` while `{"mode": 3}` is an error.
8. **Retired (SPEC X-2):** `published` is no longer accepted for
   `kind:"workflow"`; a present key is a validation failure.
9. **A finite number can become non-finite while rounding.** A `number` field
   rounds to its decimal places by scaling, and a value near the top of the
   `f64` range overflows that scale. A calculated value that is not finite when
   it is stored is skipped in `Draft` and reports `CALCULATED_VALUE_INVALID` in
   `Complete`; it is not turned into `null`.
10. **Versions are strict triplets.** `1..0.0`, `01.0.0`, `"  1.2.3  "` and
    `-0.0.0` are rejected; pre-release and build suffixes stay rejected.
11. **`compile` drops unknown top-level keys** from the form, UI and rules
    documents, and unknown keys from layout nodes, while preserving unknown keys
    inside fields.
12. **A component's `contentHash` is a pin, and a pin is verified.** A non-empty
    `contentHash` on a `components[]` entry is recomputed from that component's own
    compiled triple and compared; a mismatch fails the call with
    `COMPONENT_HASH_MISMATCH`. An absent key or an empty string is **not** a pin:
    colander carries it into `dependencyMetadataJson` as the empty string and does
    not compare it.
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
