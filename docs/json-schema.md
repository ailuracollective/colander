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

| Keyword                                 | Applies to | Notes                                                                                                                                                                                        |
| --------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `type`                                  | any        | A string or an array of strings from the closed set `object`, `array`, `string`, `boolean`, `null`, `number`, `integer`; any other name is a schema error. `integer` accepts `1.0` and `1e3` |
| `enum`                                  | any        | Compared with numeric equality, so `1` matches `1.0`                                                                                                                                         |
| `const`                                 | any        | Same comparison                                                                                                                                                                              |
| `allOf`                                 | any        | All must pass                                                                                                                                                                                |
| `anyOf`                                 | any        | At least one must pass                                                                                                                                                                       |
| `oneOf`                                 | any        | Exactly one must pass                                                                                                                                                                        |
| `not`                                   | any        | Must fail the given schema                                                                                                                                                                   |
| `if` / `then` / `else`                  | any        | `then`/`else` without `if` assert nothing                                                                                                                                                    |
| `properties`                            | object     | Only keys present in the instance are checked                                                                                                                                                |
| `required`                              | object     | —                                                                                                                                                                                            |
| `additionalProperties`                  | object     | `false` or a schema; if `properties` is absent, `false` rejects everything                                                                                                                   |
| `minProperties` / `maxProperties`       | object     | —                                                                                                                                                                                            |
| `items`                                 | array      | One schema for every element; no tuple form                                                                                                                                                  |
| `minItems` / `maxItems`                 | array      | —                                                                                                                                                                                            |
| `uniqueItems`                           | array      | At most one error is reported; numeric equality is by mathematical value (`0` and `-0.0` are duplicates, `9007199254740993` and `9007199254740992.0` are not)                                |
| `minLength` / `maxLength`               | string     | Length in **UTF-16 code units**                                                                                                                                                              |
| `pattern`                               | string     | Unanchored, ECMA-262-style; a pattern that cannot be compiled is an error                                                                                                                    |
| `minimum` / `maximum`                   | number     | Inclusive                                                                                                                                                                                    |
| `exclusiveMinimum` / `exclusiveMaximum` | number     | The Draft-4 boolean form is a wrong type and is rejected                                                                                                                                     |
| `multipleOf`                            | number     | Absolute tolerance 1e-6 on the quotient                                                                                                                                                      |
| `format`                                | string     | Asserts a closed set; an unrecognised name is an error. See below.                                                                                                                           |
| `$ref`                                  | —          | Local `#` pointers only, with `~0`/`~1` unescaping; no `$id`, no remote refs, no anchors                                                                                                     |
| `false` (boolean schema)                | —          | Nothing is valid against it; `true` accepts everything                                                                                                                                       |

An implemented keyword whose value has the wrong JSON type is an **error**:
`{"minLength": "3"}` fails instead of constraining nothing. `$ref` siblings are
still evaluated, which differs from older drafts.

### Recursive references are rejected (S-10)

The core does not evaluate recursive schemas, so it rejects them instead of
running them. A `$ref` that reaches a schema already being resolved — directly
(`{"$ref":"#"}`), indirectly (`a → b → a`), or through `anyOf`, `properties`,
`not` or `if`/`then` — produces one deterministic error:

```
Invalid instance: $ref: recursive reference '#/$defs/node' is not supported
```

Classification runs first and a schema that fails it is **never evaluated**.
That ordering is the safety property: a cycle can neither recurse without bound
(a stack overflow aborts the process and `catch_unwind` cannot intercept it) nor
depend on the instance that would drive the recursion. Reaching one `$defs`
entry from two independent positions is a shared reference, not a cycle, and
validates normally. A recursive _instance_ is a different matter: a
`node`/`next` document is validated by an instance-bounded rule, not by a
recursive schema.

### Evaluation work and error collection are bounded (S-11)

A non-recursive schema can amplify work through shared references and
combinators. Evaluation therefore has a deterministic budget:

```
steps = 10_000 + 20 * instance_nodes
```

`instance_nodes` counts the instance itself and every nested array element or
object member, including their scalar values. One unit is charged at every
`check_into` invocation, including a combinator probe. When the budget is
exhausted, evaluation stops and the caller sees the stable failure:

```
Invalid instance: schema: SCHEMA_EVALUATION_LIMIT: schema evaluation exceeded the step budget
```

The bound is integer work, not a wall-clock timeout, so native and WASM agree.
In the audit, a 26-level shared-`$ref` `anyOf` chain occupying 1 694 bytes took
53.8 s before this bound and returns in about 2 ms after it. The same budget
also bounds the legal `if`/`then` DAG shape and shared-reference `allOf` chains.

Error collection has a second, independent cap of 1 000 entries. This is not the
five-error rendering cap: an `allOf` branch can otherwise retain millions of
failures before rendering starts. Once collection reaches 1 000, later failures
are not retained; rendered failures state `truncated: 5 of 1000 errors shown`.
The step bound limits evaluation work, while the collection cap limits retained
error state; neither bound changes a valid workload that stays within them.

### Nesting depth is bounded separately (S-12)

A step budget cannot bound recursion depth: depth is at most the step count, so
a budget sized for a large instance also admits a `$ref` chain thousands of
levels deep, and a stack overflow aborts the process instead of returning an
error. Both phases are therefore bounded at 512 levels of nesting:

```
Invalid instance: schema: SCHEMA_DEPTH_LIMIT: schema evaluation exceeded the nesting depth
```

Classification is bounded too, and that is the phase that matters most here: a
chain of distinct references is not a cycle, so S-10 accepts it, but the
classifier still recurses once per level. Measured on the default 8 MiB stack,
5 000 levels survive and 10 000 abort; 512 sits an order of magnitude below
that and stays well above the parser's own 64-level JSON nesting limit. A
30 000-level chain of 1 MB of schema now returns this error in about 110 ms
instead of aborting, and a 200-level chain still evaluates normally.

The known unsupported assertions include `$dynamicRef`, `$recursiveRef`,
`$vocabulary`, `patternProperties`, `propertyNames`, `dependentRequired`,
`dependentSchemas`, `unevaluatedProperties`, `unevaluatedItems`, `prefixItems`,
`additionalItems`, `contains`, `minContains`, `maxContains`, `contentEncoding`,
`contentMediaType` and `contentSchema`; the set is not exhaustive.

Nothing in this subset remains open. `format` is asserted for the closed set
(S-5); the `pattern` dialect follows ECMA-262 as far as the `fancy-regex` engine
supports, and an uncompilable pattern is a schema error (S-6); at most the first
five assertion failures are reported, and a truncated message states how many
were shown out of the total (S-7).

What each `format` asserts, and what it does not:

- `email` — pragmatic, not RFC 5322: exactly one `@`, a non-empty local part
  without whitespace, and a domain of non-empty labels ending in at least two
  ASCII letters.
- `date` — exact RFC 3339 `full-date`: `YYYY-MM-DD` with a valid month and day,
  including leap years.
- `date-time` — exact RFC 3339 `date-time` except leap seconds: a `full-date`, a
  `T`, and a `full-time`. A space separator is not accepted.
- `time` — exact RFC 3339 `full-time` except leap seconds: `HH:MM:SS`, an
  optional fraction, then `Z` or `±HH:MM`.
- `uuid` — structural: 8-4-4-4-12 hex digits, case-insensitive; the version and
  variant nibbles are not checked, and braces are rejected.
- `ipv4` — dotted decimal with no leading zeros except a group that is exactly
  `0`; each group 0–255.
- `ipv6` — RFC 4291 forms except zone IDs: eight groups, or `::` used exactly
  once, with an embedded IPv4 address allowed in the final 32 bits.
- `hostname` — RFC 1034 labels: 1–63 ASCII letters, digits or hyphens, never
  starting or ending with a hyphen, at most 253 characters; a single label and a
  numeric-looking final label are accepted.
- `uri` — syntactic shape only: a letter-led scheme, `:`, and a non-empty
  remainder; the authority and path are not validated.

Every keyword is evaluated independently, so one instance can collect several
errors.

## Next

- The parser and canonical form behind these schemas: [json.md](json.md)
- The entry point that runs this subset: [entry-points.md](entry-points.md#colander_validate_schema)
- The documents you validate: [documents.md](documents.md)
