# JSON Schema subset

This page describes the Draft 2020-12 subset that `colander_validate_schema`
implements. colander ships no schemas of its own: the JSON Schemas that describe
your documents travel with each request, and this page says which keywords they
may use to actually constrain anything.

## JSON Schema keywords

`colander_validate_schema` implements this subset of Draft 2020-12. Nothing else is
implemented, and **unknown keywords are silently ignored** — there is no
unknown-keyword detection at all.

| Keyword                                 | Applies to | Notes                                                                                    |
| --------------------------------------- | ---------- | ---------------------------------------------------------------------------------------- |
| `type`                                  | any        | A string or an array of strings; `integer` accepts `1.0` and `1e3`                       |
| `enum`                                  | any        | Compared with numeric equality, so `1` matches `1.0`                                     |
| `const`                                 | any        | Same comparison                                                                          |
| `allOf`                                 | any        | All must pass                                                                            |
| `anyOf`                                 | any        | At least one must pass                                                                   |
| `oneOf`                                 | any        | Exactly one must pass                                                                    |
| `not`                                   | any        | Must fail the given schema                                                               |
| `if` / `then` / `else`                  | any        | `then`/`else` without `if` are ignored                                                   |
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
| `exclusiveMinimum` / `exclusiveMaximum` | number     | The Draft-4 boolean forms are ignored                                                    |
| `multipleOf`                            | number     | Absolute tolerance 1e-6 on the quotient                                                  |
| `format`                                | string     | **Annotation only — never asserted**                                                     |
| `$ref`                                  | —          | Local `#` pointers only, with `~0`/`~1` unescaping; no `$id`, no remote refs, no anchors |
| `false` (boolean schema)                | —          | Nothing is valid against it; `true` accepts everything                                   |

Not implemented, therefore ignored: `$schema`, `$id`, `$anchor`, `$defs` (other
than as a `$ref` target), `title`, `description`, `default`, `examples`,
`deprecated`, `patternProperties`, `propertyNames`, `dependentRequired`,
`dependentSchemas`, `unevaluatedProperties`, `unevaluatedItems`, `prefixItems`,
`additionalItems`, `contains`, `contentEncoding`, `contentMediaType`.

A keyword carrying the wrong JSON type is ignored too: `{"minLength": "3"}` does
not constrain anything. `$ref` siblings are still evaluated, which differs from
older drafts.

Every keyword is evaluated independently, so one instance can collect several
errors — but at most the first five are reported.

## Next

- The parser and canonical form behind these schemas: [json.md](json.md)
- The entry point that runs this subset: [entry-points.md](entry-points.md#colander_validate_schema)
- The documents you validate: [documents.md](documents.md)
