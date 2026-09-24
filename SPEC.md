# colander contract specification

Contract version: `contract-1`, versioned independently of the crate version.
Status: draft for maintainer approval.

## What this document is

`docs/` explains how the core behaves, in prose, for whoever is using it. This
document is the **contract of record**: the normative statements the crate is
measured against. Where the two disagree, this document wins and `docs/` is
wrong. Three things are versioned on separate axes — the crate
(`CARGO_PKG_VERSION`), the ABI (`colander_abi_version()`), and this contract —
and they can move independently.

## Status markers

Every requirement carries exactly one: **`live`** — implemented and conformant in
the crate today; **`decided`** — the maintainer has decided this, the crate does
not do it yet; **`proposed`** — written down but not yet decided. A `decided`
requirement is a commitment, not a description: the contract is satisfied when
no `decided` requirement remains unresolved.

## Conformance at a glance

| Group | Subject                         | `live` | `decided` | `proposed` |
| ----- | ------------------------------- | ------ | --------- | ---------- |
| C     | Wire contract and the ABI       | 11     | 0         | 0          |
| E     | The six operations              | 9      | 0         | 0          |
| D     | Documents, fields, id and code  | 4      | 0         | 0          |
| R     | Rules and the dependency check  | 17     | 0         | 0          |
| V     | Response validation             | 10     | 1         | 0          |
| S     | JSON Schema subset              | 12     | 0         | 0          |
| H     | Key-sorted form and hashing     | 5      | 0         | 0          |
| P     | Compilation, components, semver | 11     | 0         | 0          |
| X     | Retirements and reversals       | 2      | 1         | 0          |
| F     | The frozen vectors              | 1      | 0         | 0          |

The counts are derived from the markers below, so move a marker and its count
together. The ten groups hold 84 requirements: 82 `live`, 2 `decided`, 0 `proposed`.

## C — Wire contract and the ABI

- **C-1** `live`. Every `char *`-returning entry point takes one NUL-terminated
  UTF-8 JSON request string, or `NULL`, and returns one NUL-terminated UTF-8 JSON
  envelope.
- **C-2** `live`. The envelope is `{"ok":true,"result":{…}}` or
  `{"ok":false,"error":{"kind":…,"message":…}}`, with `kind` one of
  `invalid_request`, `validation` or `panic`.
- **C-3** `live`. No `char *`-returning entry point returns `NULL`. The allocator
  pair is the exception and returns `NULL` for a zero or unrepresentable length.
- **C-4** `live`. A returned string is released with `colander_free_string`, never
  with `free()`. `colander_alloc`/`colander_free_buffer` exist for callers without
  a shared heap.
- **C-5** `live`. `colander_abi_version()` returns the ABI version as a
  `uint32_t`; a caller refuses to bind on a mismatch.
- **C-6** `live`. A present optional key of the wrong JSON type is rejected.
  "Absent" means "use the default"; "present with the wrong type" is an error. An
  explicit `null` is a wrong type, not an absence: `{"mode": null}` is rejected, so
  a caller that would otherwise serialise an absent value as `null` must omit the
  key instead.
- **C-7** `live`. No panic reaches the caller, including the final envelope
  serialization: the encode runs inside the boundary, and a panic there returns
  a static literal instead of unwinding.
- **C-8** `live`. C-6 reaches a key of an object the request carries, not only a
  top-level key: a `components[]` entry's `uiSchemaJson` or `contentHash` with the
  wrong type is rejected, so `{"contentHash": 7}` fails instead of compiling with
  an empty hash.
- **C-9** `live`. When `schemas` is present it must be an object, for every
  `kind`, `instance` included.
- **C-10** `live`. A request larger than 64 MiB is refused with
  `REQUEST_TOO_LARGE` before it is parsed, so the boundary is bounded for
  untrusted callers.
- **C-11** `live`. Every failure carries a `SCREAMING_SNAKE` code a caller can
  branch on: `JSON_PARSE_ERROR`, `JSON_NOT_OBJECT`, `INVALID_UTF8`,
  `NULL_REQUEST`, `INVALID_SEMVER`, `REQUEST_TOO_LARGE`, the `RULE_*`,
  `FIELD_*`, `UI_*`, `COMPONENT_*`, `REPEATER_*` and validation-code families.
  Message wording is never part of the contract.

## E — The six operations

Request and response keys are in `docs/entry-points.md`; this section fixes
behaviour, not shape.

- **E-1** `live`. `colander_compile` expands `component-ref` fields, canonicalizes
  the three documents, reports `dependencyMetadataJson` and hashes the compiled
  triple. `dependencyMetadataJson` is not part of the hash.
- **E-2** `live`. `colander_evaluate_rules` returns `visibility`, `enabled` and
  `required` keyed by field **id**, plus `calculatedValues` and
  `validationErrors`. Every field appears in the three maps.
- **E-3** `live`. `colander_validate_response` returns `normalizedAnswersJson`,
  `errors` and `isValid`, in `Draft` or `Complete` mode. A rejected answer is not
  an exception.
- **E-4** `live`. `colander_validate_schema` validates against a schema the caller
  supplies, for kinds `form`, `component`, `workflow` and `instance`. A schema
  failure is a failure envelope; `{"valid":false}` is never returned.
- **E-5** `live`. `colander_content_hash` hashes the triple as given, with no
  compilation.
- **E-6** `live`. `colander_next_version` returns a version; `colander_version_info`
  reports name, crate version and ABI version.
- **E-7** `live`. A call is independent: a request carries every document it needs,
  and the library remembers nothing between calls.
- **E-8** `live`. `colander_validate_response` accepts an array answer, so a
  `choice` with `allowMultiple` is validatable: arrays become lists in the rule
  value domain, and per-field conversion normalizes them. A `choice` without
  `allowMultiple` rejects an array answer as a field error.
- **E-9** `live`. One unusable answer produces one error, not a failed call:
  the flattening step records a missing rule value and the per-field loop
  reports the type error, so errors collected earlier survive.

## D — Documents, fields, `id` and `code`

- **D-1** `live`. A form is three documents — form, UI and rules — sharing one
  version lineage; a version mismatch between form and rules is an error.
- **D-2** `live`. Exactly twelve field type names exist, matched case-sensitively,
  with no aliases.
- **D-3** `live`. Rules and the UI reference fields by `id`; answers and calculated
  values are keyed by `code`. The two are not interchangeable.
- **D-4** `live`. Repeater children are flat scalar fields: anything with
  nested `items` under a repeater is rejected under the code
  `REPEATER_NESTED_FIELD`.

## R — Rules and the dependency check

- **R-1** `live`. The operator set is fixed: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`,
  `and`, `or`, `not`, `empty`, `coalesce`, `add`, `sub`, `mul`, `div`.
- **R-2** `live`. Calculations run in topological order, then predicates, then
  validations.
- **R-3** `live`. A validation entry reports its error when `assert` is present,
  non-null and falsy. `when`, when present, guards the entry.
- **R-4** `live`. A `validations` entry with no `assert` is rejected by the
  analyzer, whether or not it carries `when`, under the code
  `RULE_MISSING_ASSERT`: an entry with nothing to assert can never report
  anything.
- **R-5** `live`. A duplicate field `code` is rejected by every entry point
  that reads a form, with or without a rules document, under the code
  `RULE_DUPLICATE_FIELD_CODE`. The check runs on the effective documents:
  for `colander_compile` that is the compiled triple, because rules
  legitimately reference fields that exist only after component expansion.
- **R-6** `live`. The dependency check runs in every entry point, rather than only
  in `colander_validate_schema` with `kind:"form"`.
- **R-7** `live`. Repeater row scope. A `calculate` on a repeater child with rows
  evaluates once per row, in a scope holding that row over the outer values; the
  row's own child codes resolve to that row, and fields outside resolve normally.
  Each result is written back into its row, so a later calculation or aggregate
  observes computed values. `calculatedValues` for the child is keyed by code and
  holds an array, one entry per row, and the same array replaces the flattened
  value in the working set. The aggregate surface is `count(code)` and
  `sum(code, childCode)`: `count` returns how many rows a repeater has, falling
  back to the caller's count when it is a number and to 0 otherwise; `sum` adds
  a child across the rows as numbers, treating a missing or non-numeric child as
  0, and is `null` with no rows.
- **R-7a** `live`. Row data reaches the evaluator alongside the flat values, not
  inside them. `colander_validate_response` builds rows from the submitted
  answers; `colander_evaluate_rules` reads a `List` under a repeater code as that
  repeater's rows (a number stays a row count, with no per-row calculation).
  Either way the observable R-7 is identical.
- **R-8** `live`. Explicitly out of scope for row scope v1, and none of it
  exists: per-row `visibility`/`enabled`/`required`, per-row validations with
  row-scoped paths, index addressing such as `items[0].price`, and the aggregates
  `min`, `max`, `every` and `some`.
- **R-9** `live`. Row-scope references fail closed. A repeater-child code read
  directly — from a predicate, a cross-field validation, or a `calculate`
  that is not on a child of the same repeater — is rejected under the code
  `RULE_INVALID_ROW_REFERENCE`, because the flat values keep only the last
  row per child code and any other row would be arbitrary. The aggregate
  positions (`count`/`sum` arguments) and a `calculate` on a sibling child
  of the same repeater are the legal positions.
- **R-10** `live`. A `fields` rule entry carrying a key other than
  `visibleWhen`, `enabledWhen`, `requiredWhen` or `calculate` is rejected
  under the code `RULE_UNKNOWN_RULE_KEY`: an unknown key can never apply
  anything, so accepting it would fail open to the defaults.
- **R-11** `live`. When any expression in a call fails, the whole call fails:
  there is no partial evaluation result.
- **R-12** `live`. Expression shapes are checked before any evaluation and
  fail closed with a `RULE_*` code: fixed-arity operators (`eq`, `neq`,
  `gt`, `gte`, `lt`, `lte`, `add`, `sub`, `mul`, `div`, `count`, `sum`,
  `not`, `empty`) accept exactly their arity, `and`/`or`/`coalesce` fold
  over their list, arguments are never `null`, a `ref` is a non-empty
  string, a `lit` is never combined with `op`/`args`, an unknown operator
  is `RULE_UNSUPPORTED_EXPRESSION_OPERATOR`, and a wrong count is
  `RULE_INVALID_EXPRESSION_ARITY`. A `validations` entry accepts only
  `code`, `when`, `assert` and `message` (`RULE_UNKNOWN_VALIDATION_KEY`).
- **R-13** `live`. A repeater-child calculation evaluates in O(outer
  values + rows), not O(outer values × rows): the per-row scope overlays a
  template cloned once per calculation, so an N-row calculation does not
  clone the working set N times.
- **R-14** `live`. The engine's analysis is linear in the work it is given,
  and every entry point pays for it once per call: dependency ordering walks
  a reverse adjacency (O(fields + edges)) instead of rescanning every
  dependency list per node; reference collection dedupes through a set; and
  a call that both validates and evaluates analyzes the pair a single time.
  Chained per-row calculations stay linear: the per-row template never
  carries a previously calculated per-row array, and a row referencing a
  sibling calculated child reads that child's value _for that row_.
- **R-15** `live`. `sum` accumulates with compensation (Kahan). The result is
  still row-order sensitive — IEEE-754 addition is not associative — but
  summing thousands of small decimals no longer drifts in the last digits.
- **R-16** `live`. The comparison operators (`eq`, `neq`, `gt`, `gte`, `lt`,
  `lte`) decide integers exactly: two integers compare as integers, and an
  integer compares against an integral double through `i128`, so neither
  `9007199254740993` and `9007199254740992` nor `i64::MAX` and its nearest
  double collapse into an equality. This is the rule `CALCULATED_VALUE_MISMATCH`
  uses (V-8, V-11), so the two surfaces cannot disagree about one pair. A bool
  or numeric-string operand is still coerced to a number, and two doubles still
  compare within the shared absolute tolerance; only the integer paths are
  exact.

## V — Response validation

- **V-1** `live`. `Draft` accepts an incomplete response; `Complete` requires every
  required field. Within one field only the first applicable error is reported, and
  a field that errors is never normalized.
- **V-2** `live`. The check order within a field is hidden/disabled, then
  read-only, then required, then type conversion, then constraints.
- **V-3** `decided`. A read-only field that carries a submitted value reports
  `READONLY_FIELD_MODIFIED`. `DISABLED_FIELD_VALUE` is reserved for a field that is
  not active for some other reason. Today a read-only field is `enabled: false` by
  default, so the inactive check wins and the imprecise code is reported; the
  precise code appears only when a rule re-enables the field. The behaviour must be
  symmetric between scalar fields and repeater children.
- **V-4** `live`. The error codes in `docs/validation.md` are the contract; the
  message wording is not.
- **V-5** `live`. A calculated value that is not finite when it is stored is
  skipped in `Draft` and reports `CALCULATED_VALUE_INVALID` in `Complete`. The
  case is reachable: a `number` field rounds by scaling to its decimal places,
  and a finite value near the top of the `f64` range overflows that scale.
- **V-6** `live`. An answer keyed by an unknown field still reports
  `UNKNOWN_FIELD` even when another answer in the same call is unusable.
- **V-7** `live`. A `path` is a JSON pointer into the form schema, with two
  exceptions: `/answers/<key>` for an unknown key, and `/rules/validations` for a
  failed cross-field validation.
- **V-8** `live`. Flag and value semantics shared with the rule engine: a
  `requiredWhen` predicate overwrites the schema default in both directions,
  so `requiredWhen: false` unsets a schema `required: true`; the
  `CALCULATED_VALUE_MISMATCH` comparison is numeric across integer and
  double spellings within an absolute tolerance of `1e-6`, and it is the
  same equality the rule operators use; and normalization is sparse — an
  empty scalar answer is omitted from `normalizedAnswersJson`, while
  repeaters always serialize, even empty.
- **V-9** `live`. A response carries at most 100 errors; when more were
  collected the last entry is `VALIDATION_ERRORS_TRUNCATED` and its message
  states the total, so an invalid submission cannot amplify the response
  without bound.
- **V-10** `live`. The error list is bounded in bytes as well as in count:
  caller-controlled strings echoed inside a message or path are elided after
  256 characters (marking what was elided), and the errors kept together
  total at most 64 KiB. A multi-megabyte field code cannot turn a small
  failure into a large response.
- **V-11** `live`. Integer comparison is exact against an integral double: an
  exact answer beyond the exactly-representable `f64` range (2^53) is a
  `CALCULATED_VALUE_MISMATCH`, and a calculation that lands outside that range
  on an `integer` field is `CALCULATED_VALUE_INVALID` and is not stored. The
  core never rounds a client's exact integer away in silence.

## S — JSON Schema subset

- **S-1** `live`. The supported Draft 2020-12 keywords are exactly those listed in
  `docs/json-schema.md`, with local `#` references only.
- **S-2** `live`. `format` is annotation-only: a schema that constrains only
  `format` accepts every string. The `pattern` dialect and its error rule are
  fixed by S-6.
- **S-3** `live`. Keywords are classified in three sets: implemented;
  annotation-only (`title`, `description`, `default`, `examples`, `deprecated`,
  `$comment`, `$id`, `$schema`, `$defs`, `$anchor`, `readOnly`, `writeOnly`),
  which stay ignored; and **unsupported assertions**, which are an error. An
  assertion keyword the core does not implement must not pass as if it had.
- **S-4** `live`. A keyword whose value has the wrong JSON type is an error.
  `{"minLength": "3"}` must not constrain nothing in silence.
- **S-5** `live`. `format` is asserted for a closed set — `email`, `date`,
  `date-time`, `time`, `uuid`, `ipv4`, `ipv6`, `hostname`, `uri` — and only for
  string instances. An unrecognised format name is an error (see the structural
  classifier). Asserting `format` is a deliberate deviation from the
  annotation-only default, stated here rather than implied. Each check documents
  whether it is exact or a pragmatic approximation.
- **S-6** `live`. `pattern` follows ECMA-262 semantics as far as the
  `fancy-regex` engine supports; a pattern that cannot be compiled is an error,
  never a silent non-match.
- **S-7** `live`. When a failure is truncated, the response says so: at most the
  first five assertion failures are rendered, and the message states how many
  were shown out of the total.
- **S-8** `live`. The `type` vocabulary is closed: `object`, `array`,
  `string`, `boolean`, `null`, `number` and `integer`. Any other name is a
  schema error, not an assertion that every instance fails.
- **S-9** `live`. `uniqueItems` is linear in the number of items: items are
  bucketed by a value hash that agrees with JSON Schema numeric equality
  (`1` and `1.0` share a bucket, and so do `0` and `-0.0`), and the full deep
  comparison runs inside a bucket, so collisions stay exact. Equality is by
  mathematical value, not by `f64` rounding: two numbers written as different
  literals are equal when they denote the same number, so
  `9007199254740993` and `9007199254740993.0` are duplicates while
  `9007199254740993` and `9007199254740992.0` are not. The rule is recursive, so
  numbers nested in arrays and objects are compared the same way.
- **S-10** `live`. A schema whose local `$ref` graph reaches itself — directly,
  indirectly, or through a combinator, a property or a blocking keyword — is
  rejected with one deterministic `recursive reference '<pointer>' is not
  supported` error. The core does not evaluate recursive schemas: a schema that
  fails classification is never evaluated, so a cycle can neither abort the
  process nor depend on the instance that would drive it. Reaching one `$defs`
  entry from two independent positions is a shared reference, not a cycle, and
  keeps validating.
- **S-11** `live`. Schema evaluation has a deterministic per-call budget of
  `10_000 + 20 * instance_nodes` steps, charging one unit at every `check_into`
  invocation. Exhaustion stops evaluation and reports the stable
  `SCHEMA_EVALUATION_LIMIT` error. Error collection is separately capped at
  1,000 entries, and rendered output states when the list is truncated. Neither
  bound depends on wall-clock time or runtime-specific iteration.
- **S-12** `live`. Schema nesting is bounded independently of the step budget,
  at 512 levels, during both classification and instance evaluation; exceeding
  it reports the stable `SCHEMA_DEPTH_LIMIT` error. A step budget cannot
  substitute for this bound: recursion depth is at most the step count, so a
  budget sized for a large instance also admits a `$ref` chain deep enough to
  exhaust the stack, and a stack overflow aborts the process rather than
  returning an error. A chain of distinct references is not a cycle and stays
  legal up to that depth.

## H — Canonical form and hashing

- **H-1** `live`. Canonical serialization sorts object keys by UTF-8 byte order,
  emits no whitespace, and preserves number literals. It is **not** RFC 8785: that
  standard orders by UTF-16 code units.
- **H-2** `live`. `colander_content_hash` hashes
  `{"form":…,"ui":…,"rules":…}` in document order with raw number spelling. It is
  deliberately **not** the sorted canonical form, so two documents differing only
  in key order or number spelling hash differently. Compile first for a
  hash that ignores document order.
- **H-3** `live`. `colander_compile` returns `contentHash`, a lowercase-hex SHA-256
  of the canonical compiled triple, with `dependencyMetadataJson` excluded.
- **H-4** `live`. The key-sorted form defined in H-1 is called that in this
  contract, not "canonical": it is not RFC 8785, which orders by UTF-16 code
  units, and interoperation with that standard is not offered. The wire names stay
  unchanged for now; renaming an export is a separate, larger contract change.
- **H-5** `live`. The authoritative hash for identifying a compiled form is the
  compiled triple's `contentHash` (H-3), which is order-insensitive.
  `colander_content_hash` (H-2) is a byte-exact fingerprint of the documents as
  given: it moves when key order or number spelling moves, and it is not used for
  pinning.

## P — Compilation, components and semver

- **P-1** `live`. A `component-ref` field is replaced by a group; the component is
  resolved by exact `(code, version)` match against the batch the caller supplies;
  resolution is memoized; a reference cycle — keyed on `(code, version)`, so the
  same code at another version is another triple, not a cycle — is an error; and
  nesting is bounded at 64 levels (`COMPONENT_DEPTH_EXCEEDED`). The reference
  shell keeps `required`, `readOnly` and `description` when present; every other
  property of the reference field is dropped. There is no repository and no callback.
- **P-2** `live`. `dependencyMetadataJson` lists the resolved components sorted by
  `code` then version, plus the calculated field ids and the evaluation order. A
  component's missing `contentHash` becomes the empty string.
- **P-3** `live`. A component's `contentHash` is verified: when it is a non-empty
  string it is recomputed from that component's own compiled triple — nested
  `component-ref` fields expanded and no rules document — and compared, and a
  mismatch is a hard error `COMPONENT_HASH_MISMATCH`. An absent key or an empty
  string is not a pin and is carried into `dependencyMetadataJson` unverified. The
  pin covers exact bytes, so `1.50` and `1.5` hash differently, which is intended.
- **P-9** `live`. A component's `contentHash` is verified once per
  `(code, version)` per call, not once per reference site: the recomputed
  digest is memoized, so a component referenced N times costs one
  verification plus N clones of its compiled fields.
- **P-8** `live`. A component referenced from N sites is expanded once per
  `(code, version)`; each later reference clones the compiled field array
  (so cost is linear in the materialised output, not exponential in the
  reference graph). Expansion is bounded by an explicit budget: at most
  1,000,000 field nodes and 256 MiB of materialised output, failing with
  `COMPONENT_BUDGET_EXCEEDED`. A depth limit alone is not a complexity
  bound.
- **P-10** `live`. The `components` batch is indexed by `(code, version)`
  before expansion, so resolving R references over a batch of C costs
  O(C + R) rather than O(C·R); a batch that lists the same `(code, version)`
  twice is rejected with `COMPONENT_DUPLICATE_VERSION` instead of letting
  document order decide which one wins.
- **P-11** `live`. A resolved component keeps its source form only until its
  first expansion; later references use the memoized compiled fields, so a
  large batch does not retain every source document for the whole call.
- **P-4** `live`. `colander_next_version` returns `"1.0.0"` when nothing is
  published, and otherwise increments the patch of the highest published version
  with no carry.
- **P-5** `live`. The accepted version grammar is exactly three non-negative
  integers under SemVer 2.0.0's numeric-identifier rules: ASCII digits only, no
  sign, no whitespace, no blank segments, and no leading zero unless the segment
  is exactly `0`. **Pre-release and build metadata are not permitted** on a
  component version.
- **P-6** `live`. `colander_next_version` takes the intended bump (`patch`,
  `minor` or `major`), defaulting to `patch`, and applies SemVer precedence to
  the highest published version. It never infers the bump from a list of version
  strings. An unrecognised bump name is an error.
- **P-7** `live`. `colander_compile` keeps a known set of top-level keys and drops
  unknown ones, while preserving unknown keys inside fields. It neither sorts nor
  inspects `fields`, and it does not check `type` against the known list. Array
  order belongs to the caller.

## X — Retirements and reversals

- **X-1** `live`. `CALCULATED_VALUE_INVALID` is **not** retired. Removing it was
  decided and then reversed before landing, because the decision rested on the
  claim that the branch was unreachable and that claim is false (see V-5). Keep
  the branch: it is the safety net for the overflow, and without it an
  unrepresentable value degrades into a silent `null`.
- **X-2** `live`. `published` is not accepted for `kind:"workflow"`: a present key
  fails the call with `kind:"validation"`.
- **X-3** `decided`. `DISABLED_FIELD_VALUE` stops being the reported code for a
  read-only field (see V-3).

## F — The frozen vectors

- **F-1** `live`. `tests/golden/vectors/*.json` are frozen against accidental
  drift. They were recorded from an external implementation and cannot be
  regenerated. A **decided** contract change moves the affected expectation in the
  same commit and names the decision it implements; every other case stays
  untouched. Payloads are compared canonically (key-sorted, so document order
  is not asserted) and an error case only by its
  `SCREAMING_SNAKE` code. No automatic guard covers the vectors, so the affected
  cases are enumerated by hand before each change lands.

## Open items

No requirement is unresolved: `contract-1` carries no `proposed` marker. Two
capabilities are recorded as candidates, not obligations — structured
schema-validation errors, and a structural classifier that would derive a bump —
both in `odd/tasks/public-readiness-roadmap.md`.
