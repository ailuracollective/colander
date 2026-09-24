# JSON Schema subset

This page describes the Draft 2020-12 subset that `colander_validate_schema`
implements. colander ships no schemas of its own: the JSON Schemas that describe
your documents travel with each request, and this page says which keywords they
may use to actually constrain anything.

## JSON Schema keywords

`colander_validate_schema` implements this subset of Draft 2020-12. Every schema
is classified structurally before the instance is evaluated against it, and the
classification recurses: it applies inside `properties`, `items`,
`additionalProperties`, the combinators
(`allOf`, `anyOf`, `oneOf`, `not`, `if`/`then`/`else`) and the `$defs` entries a
`$ref` reaches. A rejected keyword is caught wherever it appears, even three
levels deep.

Each keyword falls into one of three sets:

- **Implemented** — the table below. These are the assertions the subset runs.
- **Annotation-only** — `title`, `description`, `default`, `examples`,
  `deprecated`, `$comment`, `$id`, `$schema`, `$defs`, `$anchor`, `readOnly` and
  `writeOnly`. They never assert, so they are accepted and ignored whatever their
  value.
- **Unsupported assertions** — every other keyword, and an **error**. The rule is
  open rather than a fixed denylist: a keyword is unsupported when it is neither
  implemented nor annotation-only. Passing one as if it had asserted would be a
  silent false positive, so it fails the call instead.

| Keyword                                 | Applies to | Notes                                                                                    |
| --------------------------------------- | ---------- | ---------------------------------------------------------------------------------------- |
| `type`                                  | any        | A string or an array of strings; `integer` accepts `1.0` and `1e3`                       |
| `enum`                                  | any        | Compared with numeric equality, so `1` matches `1.0`                                     |
| `const`                                 | any        | Same comparison                                                                          |
| `allOf`                                 | any        | All must pass                                                                            |
| `anyOf`                                 | any        | At least one must pass                                                                   |
| `oneOf`                                 | any        | Exactly one must pass                                                                    |
| `not`                                   | any        | Must fail the given schema                                                               |
| `if` / `then` / `else`                  | any        | `then`/`else` without `if` assert nothing                                                |
| `properties`                            | object     | Only keys present in the instance are checked                                            |
| `required`                              | object     | —                                                                                        |
| `additionalProperties`                  | object     | `false` or a schema; if `properties` is absent, `false` rejects everything               |
| `minProperties` / `maxProperties`       | object     | —                                                                                        |
| `items`                                 | array      | One schema for every element; no tuple form                                              |
| `minItems` / `maxItems`                 | array      | —                                                                                        |
| `uniqueItems`                           | array      | At most one error is reported                                                            |
| `minLength` / `maxLength`               | string     | Length in **UTF-16 code units**                                                          |
| `pattern`                               | string     | Unanchored; an uncompilable pattern always fails                                         |
| `minimum` / `maximum`                   | number     | Inclusive                                                                                |
| `exclusiveMinimum` / `exclusiveMaximum` | number     | The Draft-4 boolean form is a wrong type and is rejected                                 |
| `multipleOf`                            | number     | Absolute tolerance 1e-6 on the quotient                                                  |
| `format`                                | string     | **Annotation only — never asserted**                                                     |
| `$ref`                                  | —          | Local `#` pointers only, with `~0`/`~1` unescaping; no `$id`, no remote refs, no anchors |
| `false` (boolean schema)                | —          | Nothing is valid against it; `true` accepts everything                                   |

An implemented keyword whose value has the wrong JSON type is an **error**:
`{"minLength": "3"}` fails instead of constraining nothing. `$ref` siblings are
still evaluated, which differs from older drafts.

The known unsupported assertions include `$dynamicRef`, `$recursiveRef`,
`$vocabulary`, `patternProperties`, `propertyNames`, `dependentRequired`,
`dependentSchemas`, `unevaluatedProperties`, `unevaluatedItems`, `prefixItems`,
`additionalItems`, `contains`, `minContains`, `maxContains`, `contentEncoding`,
`contentMediaType` and `contentSchema`; the set is not exhaustive.

Still open in this subset: `format` is annotation-only (S-5), `pattern` uses the
Rust `regex` dialect and a pattern that cannot be compiled is treated as matching
nothing rather than as a schema error (S-6), and at most the first five errors are
reported without saying so (S-7).

Every keyword is evaluated independently, so one instance can collect several
errors.

## Next

- The parser and canonical form behind these schemas: [json.md](json.md)
- The entry point that runs this subset: [entry-points.md](entry-points.md#colander_validate_schema)
- The documents you validate: [documents.md](documents.md)
