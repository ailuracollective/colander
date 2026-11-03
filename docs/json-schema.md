# JSON Schema subset

The Draft 2020-12 subset that `colander_validate_schema` implements. colander
ships no schema of its own: the schemas describing your documents travel with
each request, and this page says which keywords may constrain anything.

## Keywords

Every schema is classified structurally before evaluation, recursing into
`properties`, `items`, `additionalProperties`, the combinators (`allOf`,
`anyOf`, `oneOf`, `not`, `if`/`then`/`else`) and the `$defs` entries a `$ref`
reaches. A rejected keyword is caught wherever it appears, however deep.

Each keyword falls into one of three sets:

- **Implemented** — the table below; these assertions run.
- **Annotation-only** — `title`, `description`, `default`, `examples`,
  `deprecated`, `$comment`, `$id`, `$schema`, `$defs`, `$anchor`, `readOnly` and
  `writeOnly`. They never assert: accepted and ignored whatever their value.
- **Unsupported assertions** — every other keyword, and an **error**. The rule is
  open, not a fixed denylist: a keyword is unsupported when it is neither
  implemented nor annotation-only. Passing one as if it asserted would be a
  silent false positive, so it fails the call.

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
still evaluated, unlike older drafts.

### Recursive references are rejected (S-10)

A `$ref` reaching a schema already being resolved — directly (`{"$ref":"#"}`),
indirectly (`a → b → a`), or through `anyOf`, `properties`, `not` or
`if`/`then` — is one deterministic error:

```
Invalid instance: $ref: recursive reference '#/$defs/node' is not supported
```

Classification runs first: a schema that fails it is **never evaluated**. A cycle
can neither recurse without bound (a stack overflow aborts the process;
`catch_unwind` cannot intercept it) nor depend on the driving instance. A `$defs`
entry reached from two independent positions is a shared reference, not a cycle,
and validates normally.

A recursive _instance_ is different: `node`/`next` documents use an
instance-bounded rule, not a recursive schema.

### Evaluation is bounded (S-11)

Non-recursive schemas can amplify work through shared references and combinators,
so evaluation runs on a deterministic budget:

```
steps = 10_000 + 20 * instance_nodes
```

`instance_nodes` counts the instance and every nested array element or object
member, scalars included; each `check_into` invocation, including a combinator
probe, charges one unit. Exhaustion stops evaluation with:

```
Invalid instance: schema: SCHEMA_EVALUATION_LIMIT: schema evaluation exceeded the step budget
```

The bound is integer work, not wall-clock time, so native and WASM agree. A
26-level shared-`$ref` `anyOf` chain of 1,694 bytes took 53.8 s before the bound
and about 2 ms after it; it also bounds the legal `if`/`then` DAG and
shared-reference `allOf` chains.

Error collection has a second, independent cap of 1,000 entries — not the
five-error rendering cap, since an `allOf` branch can otherwise retain millions
of failures before rendering. Past 1,000, later failures are not retained, and
rendered output says `truncated: 5 of 1000 errors shown`.

### Nesting is bounded separately (S-12)

A step budget cannot bound recursion depth: depth is at most the step count, so a
budget for a large instance also admits a `$ref` chain thousands of levels deep,
and a stack overflow aborts the process. Both phases are bounded at 512 levels:

```
Invalid instance: schema: SCHEMA_DEPTH_LIMIT: schema evaluation exceeded the nesting depth
```

Classification is bounded too: a chain of distinct references is not a cycle, so
S-10 accepts it, but the classifier recurses once per level. On the default 8 MiB
stack, 5,000 levels survive and 10,000 abort; 512 sits an order of magnitude
below and far above the parser's 64-level JSON limit. A 30,000-level chain of
1 MB now returns this error in about 110 ms, and a 200-level chain still
evaluates.

Known unsupported assertions include `$dynamicRef`, `$recursiveRef`,
`$vocabulary`, `patternProperties`, `propertyNames`, `dependentRequired`,
`dependentSchemas`, `unevaluatedProperties`, `unevaluatedItems`, `prefixItems`,
`additionalItems`, `contains`, `minContains`, `maxContains`, `contentEncoding`,
`contentMediaType` and `contentSchema`; the list is not exhaustive.

## What `format` asserts

`format` asserts a closed set of names and only against string instances; an
unrecognised name is a schema error (S-5). At most the first five assertion
failures are reported, and a truncated message states how many were shown of the
total (S-7). A `pattern` follows ECMA-262 as far as `fancy-regex` supports (S-6);
an uncompilable pattern is a schema error.

- `email` — pragmatic, not RFC 5322: exactly one `@`, a non-empty local part
  without whitespace, a domain of non-empty labels ending in at least two ASCII
  letters.
- `date` — exact RFC 3339 `full-date`: `YYYY-MM-DD` with a valid month and day,
  leap years included.
- `date-time` — exact RFC 3339 `date-time` minus leap seconds: `full-date`, `T`,
  `full-time`. A space separator is rejected.
- `time` — exact RFC 3339 `full-time` minus leap seconds: `HH:MM:SS`, an
  optional fraction, then `Z` or `±HH:MM`.
- `uuid` — structural: 8-4-4-4-12 hex digits, case-insensitive; version and
  variant nibbles unchecked, braces rejected.
- `ipv4` — dotted decimal, no leading zeros except a group that is exactly `0`;
  each group 0–255.
- `ipv6` — RFC 4291 forms except zone IDs: eight groups, or `::` used exactly
  once, with an embedded IPv4 address allowed in the final 32 bits.
- `hostname` — RFC 1034 labels: 1–63 ASCII letters, digits or hyphens, no leading
  or trailing hyphen, at most 253 characters; a single label and a numeric final
  label are accepted.
- `uri` — syntactic shape only: a letter-led scheme, `:`, a non-empty remainder;
  authority and path are not validated.

Every keyword is evaluated independently, so one instance can collect several
errors.

## See also

- The parser and canonical form behind these schemas: [json.md](json.md)
- The entry point that runs this subset: [entry-points-payload.md](entry-points-payload.md#colander_validate_schema)
- The documents you validate: [documents.md](documents.md)
