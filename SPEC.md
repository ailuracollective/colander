# colander contract specification

Contract version: `contract-1`, versioned independently of the crate version.
Status: draft for maintainer approval.

## What this document is

`docs/` explains in prose how the core behaves; this document is the **contract
of record**: the normative statements the crate is measured against. Where the
two disagree, this document wins and `docs/` is wrong.

The crate (`CARGO_PKG_VERSION`), the ABI (`colander_abi_version()`) and this
contract are versioned on separate axes and move independently.

## Status markers

Every requirement carries exactly one marker:

- **`live`** — implemented and conformant today.
- **`decided`** — the maintainer has decided this; the crate does not do it yet.
- **`proposed`** — written down but not yet decided.

A `decided` requirement is a commitment, not a description; the contract is
satisfied when none remains unresolved.

## Conformance at a glance

| Group | Subject                         | `live` | `decided` | `proposed` |
| ----- | ------------------------------- | ------ | --------- | ---------- |
| C     | Wire contract and the ABI       | 11     | 0         | 0          |
| E     | The seven operations            | 10     | 0         | 0          |
| D     | Documents, fields, id and code  | 4      | 0         | 0          |
| R     | Rules and the dependency check  | 17     | 0         | 0          |
| V     | Response validation             | 11     | 1         | 0          |
| S     | JSON Schema subset              | 13     | 0         | 0          |
| H     | Key-sorted form and hashing     | 5      | 0         | 0          |
| P     | Compilation, components, semver | 11     | 0         | 0          |
| X     | Retirements and reversals       | 2      | 1         | 0          |
| F     | The frozen vectors              | 1      | 0         | 0          |

Counts derive from the markers below: move a marker and its count together. The
ten groups hold 87 requirements — 85 `live`, 2 `decided`, 0 `proposed`.

## C — Wire contract and the ABI

- **C-1** `live`. Every `char *`-returning entry point takes one NUL-terminated
  UTF-8 JSON request string or `NULL` and returns one NUL-terminated UTF-8 JSON
  envelope.
- **C-2** `live`. The envelope is `{"ok":true,"result":{…}}` or
  `{"ok":false,"error":{"kind":…,"message":…}}`, `kind` one of
  `invalid_request`, `validation` or `panic`.
- **C-3** `live`. No `char *`-returning entry point returns `NULL`, except the
  allocator pair for a zero or unrepresentable length.
- **C-4** `live`. A returned string is released with `colander_free_string`, never
  `free()`; `colander_alloc`/`colander_free_buffer` serve callers without a shared
  heap.
- **C-5** `live`. `colander_abi_version()` returns the ABI version as a
  `uint32_t`; a caller refuses to bind on a mismatch.
- **C-6** `live`. A present optional key of the wrong JSON type is rejected;
  absent means the default, and explicit `null` is a wrong type, not an absence:
  `{"mode": null}` is rejected.
- **C-7** `live`. No panic reaches the caller, including the final envelope
  serialization: the encode runs inside the boundary, where a panic returns a
  static literal instead of unwinding.
- **C-8** `live`. C-6 reaches a key of an object the request carries, not only a
  top-level key: a `components[]` entry's `uiSchemaJson` or `contentHash` with the
  wrong type is rejected; `{"contentHash": 7}` fails instead of compiling with an
  empty hash.
- **C-9** `live`. `schemas`, when present, must be an object for every `kind`,
  `instance` included.
- **C-10** `live`. A request over 64 MiB is refused with `REQUEST_TOO_LARGE`
  before parsing.
- **C-11** `live`. Every failure carries a branchable `SCREAMING_SNAKE` code:
  `JSON_PARSE_ERROR`, `JSON_NOT_OBJECT`, `INVALID_UTF8`, `NULL_REQUEST`,
  `INVALID_SEMVER`, `REQUEST_TOO_LARGE`, the `RULE_*`, `FIELD_*`, `UI_*`,
  `COMPONENT_*`, `REPEATER_*` and validation-code families. Message wording is
  never part of the contract.

## E — The seven operations

Request and response keys are in `docs/entry-points.md`; this section fixes
behaviour, not shape.

- **E-1** `live`. `colander_compile` expands `component-ref` fields, canonicalizes
  the three documents, reports `dependencyMetadataJson` and hashes the compiled
  triple (H-3).
- **E-2** `live`. `colander_evaluate_rules` returns `visibility`, `enabled` and
  `required` keyed by field **id**, plus `calculatedValues` and
  `validationErrors`; every field appears in the three maps.
- **E-3** `live`. `colander_validate_response` returns `normalizedAnswersJson`,
  `errors` and `isValid`, in `Draft` or `Complete` mode; a rejected answer is not
  an exception.
- **E-4** `live`. `colander_validate_schema` validates against a caller-supplied
  schema for kinds `form`, `component`, `workflow` and `instance`; a schema
  failure is a failure envelope, never `{"valid":false}`.
- **E-5** `live`. `colander_content_hash` hashes the triple as given, with no
  compilation.
- **E-6** `live`. `colander_next_version` returns a version;
  `colander_version_info` reports name, crate version and ABI version.
- **E-7** `live`. A call is independent: a request carries every document it
  needs, and the library remembers nothing between calls.
- **E-8** `live`. `colander_validate_response` accepts an array answer, so a
  `choice` with `allowMultiple` is validatable; arrays become lists in the rule
  value domain, normalized per field. Without `allowMultiple`, a `choice`
  rejects an array answer as a field error.
- **E-9** `live`. One unusable answer produces one error, not a failed call:
  flattening records a missing rule value, the per-field loop reports the type
  error, so earlier errors survive.
- **E-10** `live`. `colander_describe_form` returns the compiled triple's field
  index: per field its `id`, `code`, `path` (a JSON pointer into the form schema),
  container `parentPath`, `type`, schema `required` and `readOnly`, and the
  triple's `contentHash`. `code` is document-declared and present on every valid
  field; answer-key status follows from `type`. Entries are flat, in field order,
  `parentPath` `null` at top level.
  A `component-ref` appears as its expansion: the schema baseline, not a rule
  evaluation (`colander_evaluate_rules`). Presentation keys
  (`title`, `description`) are not described and stay the caller's, inert by
  `documents.md`.

## D — Documents, fields, `id` and `code`

- **D-1** `live`. A form is three documents — form, UI and rules — sharing one
  version lineage; a form/rules version mismatch is an error.
- **D-2** `live`. Exactly twelve field type names exist, case-sensitive, with no
  aliases.
- **D-3** `live`. Rules and the UI reference fields by `id`; answers and
  calculated values are keyed by `code`; the two are not interchangeable.
- **D-4** `live`. Repeater children are flat scalar fields: nested `items` under a
  repeater is rejected with `REPEATER_NESTED_FIELD`.

## R — Rules and the dependency check

- **R-1** `live`. The operator set is fixed: `eq`, `neq`, `gt`, `gte`, `lt`,
  `lte`, `and`, `or`, `not`, `empty`, `coalesce`, `add`, `sub`, `mul`, `div`.
- **R-2** `live`. Calculations run in topological order, then predicates, then
  validations.
- **R-3** `live`. A validation entry reports its error when `assert` is present,
  non-null and falsy; `when`, when present, guards it.
- **R-4** `live`. A `validations` entry with no `assert` is rejected by the
  analyzer, whether or not it carries `when`, under `RULE_MISSING_ASSERT`.
- **R-5** `live`. A duplicate field `code` is rejected by every entry point that
  reads a form, with or without a rules document, under
  `RULE_DUPLICATE_FIELD_CODE`; the check runs on the effective documents — for
  `colander_compile`, the compiled triple.
- **R-6** `live`. The dependency check runs in every entry point, not only
  `colander_validate_schema` with `kind:"form"`.
- **R-7** `live`. Repeater row scope. A `calculate` on a repeater child with rows
  runs once per row, that row layered over the outer values: the row's child codes
  resolve to that row, outer fields normally. Results are written back into their
  row for later calculations. `calculatedValues` for the child is keyed by code,
  one entry per row, replacing the flattened value. Aggregates are `count(code)`
  and `sum(code, childCode)`: `count` returns a repeater's row count, or the
  caller's count when numeric, else 0; `sum` adds a child across rows as numbers,
  missing or non-numeric as 0, and is `null` with no rows.
- **R-7a** `live`. Row data reaches the evaluator alongside the flat values, not
  inside them. `colander_validate_response` builds rows from submitted answers;
  `colander_evaluate_rules` reads a `List` under a repeater code as that
  repeater's rows (a number stays a row count, no per-row calculation). Observable
  R-7 is identical either way.
- **R-8** `live`. Out of scope for row scope v1, and absent: per-row
  `visibility`/`enabled`/`required`, per-row validations with row-scoped paths,
  index addressing such as `items[0].price`, and the aggregates `min`, `max`,
  `every` and `some`.
- **R-9** `live`. Row-scope references fail closed: a repeater-child code read
  directly — from a predicate, a cross-field validation, or a `calculate` not on a
  child of the same repeater — is rejected with `RULE_INVALID_ROW_REFERENCE`.
  Legal positions: aggregate arguments (`count`/`sum`) and a `calculate` on a
  sibling child of the same repeater.
- **R-10** `live`. A `fields` rule entry carrying a key other than `visibleWhen`,
  `enabledWhen`, `requiredWhen` or `calculate` is rejected with
  `RULE_UNKNOWN_RULE_KEY`.
- **R-11** `live`. When any expression in a call fails, the whole call fails.
- **R-12** `live`. Expression shapes are checked before evaluation and fail closed
  with a `RULE_*` code: fixed-arity operators (`eq`, `neq`, `gt`, `gte`, `lt`,
  `lte`, `add`, `sub`, `mul`, `div`, `count`, `sum`, `not`, `empty`) accept
  exactly their arity; `and`/`or`/`coalesce` fold over their list; arguments are
  never `null`; a `ref` is non-empty; a `lit` never combines with `op`/`args`; an
  unknown operator is `RULE_UNSUPPORTED_EXPRESSION_OPERATOR`; a wrong count is
  `RULE_INVALID_EXPRESSION_ARITY`. A `validations` entry accepts only `code`,
  `when`, `assert` and `message` (`RULE_UNKNOWN_VALIDATION_KEY`).
- **R-13** `live`. A repeater-child calculation evaluates in O(outer values +
  rows), not O(outer values × rows): the per-row scope overlays a template cloned
  once per calculation.
- **R-14** `live`. Analysis is linear in the work given, and every entry point pays
  once per call: dependency ordering walks a reverse adjacency (O(fields + edges))
  and reference collection dedupes through a set; a call that both validates and
  evaluates analyzes the pair once. Chained per-row calculations stay linear: the
  per-row template carries no previously calculated per-row array, and a row
  referencing a sibling calculated child reads that child's value _for that row_.
- **R-15** `live`. `sum` accumulates with compensation (Kahan); the result is still
  row-order sensitive — IEEE-754 addition is not associative — but thousands of
  small decimals no longer drift in the last digits.
- **R-16** `live`. Integer comparisons (`eq`, `neq`, `gt`, `gte`, `lt`, `lte`) are
  exact: two integers compare as integers, an integer against an integral double
  through `i128`, so neither `9007199254740993` and `9007199254740992` nor
  `i64::MAX` and its nearest double collapse into equality.
  `CALCULATED_VALUE_MISMATCH` uses this rule (V-8, V-11). A bool or numeric-string
  operand is still coerced, two doubles compare within the shared absolute tolerance; only integer paths
  are exact.

## V — Response validation

- **V-1** `live`. `Draft` accepts an incomplete response, `Complete` requires
  every required field; within a field only the first applicable error is
  reported, and an erroring field is never normalized.
- **V-2** `live`. Check order per field: hidden/disabled, read-only, required, type
  conversion, constraints.
- **V-3** `decided`. A read-only field carrying a submitted value reports
  `READONLY_FIELD_MODIFIED`; `DISABLED_FIELD_VALUE` is reserved for a field
  inactive for another reason. Today a read-only field is `enabled: false` by
  default, so the inactive check wins; the precise code appears only when a rule
  re-enables it. The behaviour must be symmetric between scalars and repeater
  children.
- **V-4** `live`. The codes in `docs/validation.md` are the contract; message
  wording is not.
- **V-5** `live`. A calculated value that is not finite when stored is skipped in
  `Draft` and reports `CALCULATED_VALUE_INVALID` in `Complete`; the case is
  reachable: a `number` field scales to its decimal places, and a finite value
  near the top of the `f64` range overflows that scale.
- **V-6** `live`. An answer for an unknown field still reports `UNKNOWN_FIELD`,
  even when another answer in the call is unusable.
- **V-7** `live`. A `path` is a JSON pointer into the form schema, except
  `/answers/<key>` for an unknown key and `/rules/validations` for a failed
  cross-field validation.
- **V-8** `live`. Flag and value semantics shared with the rule engine: a
  `requiredWhen` predicate overwrites the schema default in both directions, so
  `requiredWhen: false` unsets a schema `required: true`;
  `CALCULATED_VALUE_MISMATCH` compares numerically across integer and double
  spellings within an absolute tolerance of `1e-6` (R-16); normalization is sparse
  — an empty scalar answer is omitted from `normalizedAnswersJson`, while
  repeaters always serialize, even empty.
- **V-9** `live`. A response carries at most 100 errors; when more were collected,
  the last entry is `VALIDATION_ERRORS_TRUNCATED` and its message states the total.
- **V-10** `live`. The error list is bounded in bytes as well as count:
  caller-controlled strings echoed in a message or path are elided after 256
  characters (marking what was elided), and kept errors total at most 64 KiB.
- **V-11** `live`. Integer comparison is exact (R-16): an exact answer beyond the
  exactly-representable `f64` range (2^53) is a `CALCULATED_VALUE_MISMATCH`, and a
  calculation outside that range on an `integer` field is
  `CALCULATED_VALUE_INVALID` and is not stored. The core never rounds a client's
  exact integer away in silence.
- **V-12** `live`. `colander_validate_response` accepts an optional `pageId` with
  `mode:"Complete"`, resolved against the top level of the UI `layout`: only a
  `type:"page"` node whose `id` matches is a page. It then checks only the fields
  its `children` name; required, type, constraint, repeater-count and
  calculated-value checks outside the page are not run, cross-field `validations`
  are deferred, and `normalizedAnswersJson` holds only the page's accepted fields.
  Unknown answer keys are still reported. An unknown page is `UI_UNKNOWN_PAGE`;
  `pageId` with `Draft` is `UI_PAGE_REQUIRES_COMPLETE`. A call without `pageId` is
  unchanged.

## S — JSON Schema subset

- **S-1** `live`. Supported Draft 2020-12 keywords are exactly those in
  `docs/json-schema.md`, with local `#` references only.
- **S-2** `live`. `format` is not annotation-only here: a schema carrying it
  asserts it for string instances (S-5). The `pattern` dialect and its error rule
  are fixed by S-6.
- **S-3** `live`. Keywords are classified in three sets: implemented;
  annotation-only (`title`, `description`, `default`, `examples`, `deprecated`,
  `$comment`, `$id`, `$schema`, `$defs`, `$anchor`, `readOnly`, `writeOnly`),
  ignored; and **unsupported assertions**, an error.
- **S-4** `live`. A keyword whose value has the wrong JSON type is an error.
  `{"minLength": "3"}` must not constrain nothing in silence.
- **S-5** `live`. `format` is asserted for a closed set — `email`, `date`,
  `date-time`, `time`, `uuid`, `ipv4`, `ipv6`, `hostname`, `uri` — and only for
  strings; an unrecognised name is an error (see the structural classifier). Each
  check documents whether it is exact or a pragmatic approximation.
- **S-6** `live`. `pattern` follows ECMA-262 semantics as far as `fancy-regex`
  supports; an uncompilable pattern is an error, never a silent non-match.
- **S-7** `live`. When a failure is truncated, the response says so: at most the
  first five assertion failures are rendered, and the message states how many of the total were shown.
- **S-8** `live`. The `type` vocabulary is closed: `object`, `array`, `string`,
  `boolean`, `null`, `number` and `integer`; any other name is a schema error, not
  an assertion that every instance fails.
- **S-9** `live`. `uniqueItems` is linear in the number of items: items are
  bucketed by a value hash agreeing with JSON Schema numeric equality (`1` and
  `1.0` share a bucket, so do `0` and `-0.0`), with full deep comparison inside a
  bucket. Equality is by mathematical value, not `f64` rounding: two literals
  denoting the same number are equal, so `9007199254740993` and
  `9007199254740993.0` are duplicates while `9007199254740993` and
  `9007199254740992.0` are not. The rule is recursive, so numbers nested in
  arrays and objects compare the same way.
- **S-10** `live`. A local `$ref` graph that reaches itself — directly,
  indirectly, or through a combinator, a property or a blocking keyword — is
  rejected with one deterministic `recursive reference '<pointer>' is not
  supported` error: a schema that fails classification is never evaluated, so a
  cycle can neither abort the process nor depend on the instance that would drive
  it. Reaching one `$defs` entry from two independent positions is a shared
  reference, not a cycle, and keeps validating.
- **S-11** `live`. Schema evaluation has a deterministic per-call budget of
  `10_000 + 20 * instance_nodes` steps, charging one unit per `check_into`
  invocation; exhaustion stops evaluation with `SCHEMA_EVALUATION_LIMIT`. Error
  collection is separately capped at 1,000 entries. Neither bound depends on
  wall-clock time.
- **S-12** `live`. Schema nesting is bounded independently of the step budget, at
  512 levels, during classification and instance evaluation; exceeding it reports
  `SCHEMA_DEPTH_LIMIT`. A step budget cannot substitute: a large budget admits a
  `$ref` chain deep enough to exhaust the stack. A chain of distinct references is
  not a cycle and stays legal up to that depth.
- **S-13** `live`. A `pattern` carries a cost bound: at most 512 quantifier and
  alternation constructs, counted structurally during classification, so an
  over-budget pattern is refused with `PATTERN_TOO_COMPLEX` before any string is
  matched. The count is a structural measure, never a timing measurement, so it
  is identical on every runtime. Measured cost: 1,000 constructs cost about 29 ms,
  4,000 about 374 ms and 32,000 about 17.6 s, while matched text length does not
  change the cost once the count is fixed. Escaped metacharacters and character
  classes are literals and do not count.

## H — Canonical form and hashing

- **H-1** `live`. Canonical serialization sorts object keys by UTF-8 byte order,
  emits no whitespace, and preserves number literals; it is **not** RFC 8785,
  which orders by UTF-16 code units.
- **H-2** `live`. `colander_content_hash` hashes
  `{"form":…,"ui":…,"rules":…}` in document order with raw number spelling —
  deliberately **not** the sorted canonical form, so two documents differing only
  in key order or number spelling hash differently. Compile first for an
  order-insensitive hash.
- **H-3** `live`. `colander_compile` returns `contentHash`, a lowercase-hex
  SHA-256 of the canonical compiled triple, with `dependencyMetadataJson`
  excluded.
- **H-4** `live`. The key-sorted form defined in H-1 is called that here, not
  "canonical"; interoperation with RFC 8785 is not offered. The wire names stay
  unchanged for now: renaming an export is a separate, larger contract change.
- **H-5** `live`. The authoritative hash for a compiled form is the compiled
  triple's `contentHash` (H-3), which is order-insensitive;
  `colander_content_hash` (H-2) is a byte-exact fingerprint, not used for pinning.

## P — Compilation, components and semver

- **P-1** `live`. A `component-ref` field becomes a group: the component resolves
  by exact `(code, version)` match in the caller's batch, resolution is memoized,
  a cycle — keyed on `(code, version)`, so the same code at another version is
  another triple — is an error, and nesting is bounded at 64 levels
  (`COMPONENT_DEPTH_EXCEEDED`). The shell keeps `required`, `readOnly` and
  `description`; every other property is dropped. There is no repository and no
  callback.
- **P-2** `live`. `dependencyMetadataJson` lists the resolved components sorted by
  `code` then version, plus calculated field ids and evaluation order; a missing
  component `contentHash` becomes the empty string.
- **P-3** `live`. A component's `contentHash` is verified: a non-empty string is
  recomputed from that component's own compiled triple — nested `component-ref`
  fields expanded, no rules document — and compared; a mismatch is
  `COMPONENT_HASH_MISMATCH`. An absent key or empty string is not a pin and enters
  `dependencyMetadataJson` unverified. The pin covers exact bytes: `1.50` and
  `1.5` hash differently.
- **P-4** `live`. `colander_next_version` returns `"1.0.0"` when nothing is
  published, otherwise the patch of the highest published version, with no carry.
- **P-5** `live`. The accepted version grammar is exactly three non-negative
  integers under SemVer 2.0.0's numeric-identifier rules: ASCII digits only, no
  sign, no whitespace, no blank segments, and no leading zero unless the segment
  is exactly `0`. **Pre-release and build metadata are not permitted** on a
  component version.
- **P-6** `live`. `colander_next_version` takes the intended bump (`patch`,
  `minor` or `major`), defaulting to `patch`, and applies SemVer precedence to the
  highest published version; it never infers the bump from a list of version
  strings. An unrecognised bump name is an error.
- **P-7** `live`. `colander_compile` keeps known top-level keys and drops unknown
  ones, preserving unknown keys inside fields; it neither sorts nor inspects
  `fields`, and does not check `type` against the known list. Array order belongs
  to the caller.
- **P-8** `live`. A component referenced from N sites is expanded once per
  `(code, version)`; each later reference clones the compiled field array, so cost
  is linear in the materialised output.
  Expansion is bounded by an explicit budget: at most 1,000,000 field nodes and
  256 MiB of materialised output, failing with `COMPONENT_BUDGET_EXCEEDED`.
- **P-9** `live`. A component's `contentHash` is verified once per
  `(code, version)` per call, not per reference site.
- **P-10** `live`. The `components` batch is indexed by `(code, version)` before
  expansion, so resolving R references over a batch of C costs O(C + R), not
  O(C·R); a batch listing the same `(code, version)` twice is rejected with
  `COMPONENT_DUPLICATE_VERSION`.
- **P-11** `live`. A resolved component keeps its source form only until its first
  expansion; later references use the memoized compiled fields, so a large batch
  does not retain every source document for the whole call.

## X — Retirements and reversals

- **X-1** `live`. `CALCULATED_VALUE_INVALID` is **not** retired. Removed then reversed before landing: the branch is reachable (V-5). Keep the
  branch: without it an unrepresentable value degrades into a silent `null`.
- **X-2** `live`. `published` is not accepted for `kind:"workflow"`: a present key
  fails the call with `kind:"validation"`.
- **X-3** `decided`. `DISABLED_FIELD_VALUE` stops being the reported code for a
  read-only field (see V-3).

## F — The frozen vectors

- **F-1** `live`. `tests/golden/vectors/*.json` are frozen against accidental
  drift: recorded from an external implementation, they cannot be regenerated. A
  **decided** contract change moves the affected expectation in the same commit
  and names the decision it implements; every other case stays untouched. Payloads
  compare canonically (key-sorted, so order is not asserted), an error case only
  by its `SCREAMING_SNAKE` code. No automatic guard covers them: affected cases
  are enumerated by hand.

## Open items

No requirement is unresolved: `contract-1` carries no `proposed` marker. Two
capabilities are candidates, not obligations — structured schema-validation
errors and a structural bump classifier — both in
`odd/tasks/public-readiness-roadmap.md`.
