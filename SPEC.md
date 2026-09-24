# colander contract specification

Contract version: `contract-1`, versioned independently of the crate version.
Status: draft for maintainer approval.

## What this document is

`docs/` explains how the core behaves, in prose, for whoever is using it. This
document is the **contract of record**: the normative statements the crate is
measured against. Where the two disagree, this document wins and `docs/` is
wrong.

Three things are versioned on separate axes: the crate (`CARGO_PKG_VERSION`), the
ABI (`colander_abi_version()`), and this contract. They can move independently.

## Status markers

Every requirement carries exactly one:

- **`live`** — implemented and conformant in the crate today.
- **`decided`** — the maintainer has decided this; the crate does not do it yet.
- **`proposed`** — written down but not yet decided.

A `decided` requirement is a commitment, not a description. The contract is
satisfied when no `decided` requirement remains unresolved.

## Conformance at a glance

| Group | Subject                         | `live` | `decided` | `proposed` |
| ----- | ------------------------------- | ------ | --------- | ---------- |
| C     | Wire contract and the ABI       | 7      | 2         | 0          |
| E     | The six operations              | 7      | 2         | 0          |
| D     | Documents, fields, id and code  | 3      | 0         | 0          |
| R     | Rules and the dependency check  | 5      | 3         | 0          |
| V     | Response validation             | 5      | 2         | 0          |
| S     | JSON Schema subset              | 4      | 3         | 0          |
| H     | Canonical form and hashing      | 3      | 2         | 0          |
| P     | Compilation, components, semver | 5      | 2         | 0          |
| X     | Retirements and reversals       | 2      | 1         | 0          |
| F     | The frozen vectors              | 1      | 0         | 0          |

The counts are derived from the markers below, so move a marker and its count
together. The ten groups hold 59 requirements: 42 `live`, 17 `decided` and 0
`proposed`.

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
- **C-7** `decided`. No panic reaches the caller, including the final envelope
  serialization. Today the panic boundary covers request parsing and the core
  body, but the last `encode` call runs outside it.
- **C-8** `live`. C-6 reaches a key of an object the request carries, not only a
  top-level key: a `components[]` entry's `uiSchemaJson` or `contentHash` with the
  wrong type is rejected, so `{"contentHash": 7}` fails instead of compiling with
  an empty hash.
- **C-9** `decided`. When `schemas` is present it must be an object, for every
  `kind`, `instance` included. Today a wrong-typed `schemas` is ignored for
  `kind:"instance"`.

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
- **E-8** `decided`. `colander_validate_response` accepts an array answer, so a
  `choice` with `allowMultiple` is validatable. Today the first array or object
  answer aborts the whole call before per-field validation, which discards the
  errors already collected and makes `INVALID_TYPE` unobservable for those
  answers.
- **E-9** `decided`. One unusable answer produces one error, not a failed call.
  Today `flatten_for_rules` is fatal.

## D — Documents, fields, `id` and `code`

- **D-1** `live`. A form is three documents — form, UI and rules — sharing one
  version lineage; a version mismatch between form and rules is an error.
- **D-2** `live`. Exactly twelve field type names exist, matched case-sensitively,
  with no aliases.
- **D-3** `live`. Rules and the UI reference fields by `id`; answers and calculated
  values are keyed by `code`. The two are not interchangeable.

## R — Rules and the dependency check

- **R-1** `live`. The operator set is fixed: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`,
  `and`, `or`, `not`, `empty`, `coalesce`, `add`, `sub`, `mul`, `div`.
- **R-2** `live`. Calculations run in topological order, then predicates, then
  validations.
- **R-3** `live`. A validation entry reports its error when `assert` is present,
  non-null and falsy. `when`, when present, guards the entry. An entry with no
  `assert` is inert.
- **R-4** `decided`. A `validations` entry with no `assert` is rejected by the
  analyzer, whether or not it carries `when`: an entry with nothing to assert can
  never report anything, so `when` alone does not make it valid. Today it is
  silently inert, so a mistyped key produces a validation that never runs.
- **R-5** `live`. A duplicate field `code` is rejected by every entry point that
  reads a form and rules pair. The check runs on the effective documents: for
  `colander_compile` that is the compiled triple, because rules legitimately
  reference fields that exist only after component expansion.
- **R-6** `live`. The dependency check runs in every entry point, rather than only
  in `colander_validate_schema` with `kind:"form"`.
- **R-7** `decided`. Repeater row scope. A `calculate` on a repeater child
  evaluates once per row, in row scope: the row's own child codes resolve to that
  row, and fields outside the repeater resolve normally. The repeater's own code
  resolves to its row count. `calculatedValues` for a repeater child is keyed by
  code and holds an array, one entry per row. The aggregate surface is
  `count(code)` and `sum(code, childCode)`.
- **R-8** `decided`. Explicitly out of scope for row scope v1: per-row
  `visibility`/`enabled`/`required`, per-row validations with row-scoped paths,
  index addressing such as `items[0].price`, and the aggregates `min`, `max`,
  `every` and `some`.

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
- **V-6** `decided`. An answer keyed by an unknown field still reports
  `UNKNOWN_FIELD` even when another answer in the same call is unusable. Today
  those already-collected errors are discarded.
- **V-7** `live`. A `path` is a JSON pointer into the form schema, with two
  exceptions: `/answers/<key>` for an unknown key, and `/rules/validations` for a
  failed cross-field validation.

## S — JSON Schema subset

- **S-1** `live`. The supported Draft 2020-12 keywords are exactly those listed in
  `docs/json-schema.md`, with local `#` references only.
- **S-2** `live`. `format` is annotation-only, and `pattern` uses the Rust
  `regex` dialect, so an uncompilable pattern matches nothing.
- **S-3** `live`. Keywords are classified in three sets: implemented;
  annotation-only (`title`, `description`, `default`, `examples`, `deprecated`,
  `$comment`, `$id`, `$schema`, `$defs`, `$anchor`, `readOnly`, `writeOnly`),
  which stay ignored; and **unsupported assertions**, which are an error. An
  assertion keyword the core does not implement must not pass as if it had.
- **S-4** `live`. A keyword whose value has the wrong JSON type is an error.
  `{"minLength": "3"}` must not constrain nothing in silence.
- **S-5** `decided`. `format` is asserted for a closed set — `email`, `date`,
  `date-time`, `time`, `uuid`, `ipv4`, `ipv6`, `hostname`, `uri` — and an
  unrecognised format name is an error. Asserting `format` is a deliberate
  deviation from the annotation-only default, stated here rather than implied.
- **S-6** `decided`. `pattern` follows ECMA-262 semantics as far as the engine
  supports; a pattern that cannot be compiled is an error, never a silent
  non-match.
- **S-7** `decided`. When a failure is truncated, the response says so. Today at
  most the first five errors are reported and the truncation is silent.

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
- **H-4** `decided`. The key-sorted form defined in H-1 is called that in this
  contract, not "canonical": it is not RFC 8785, which orders by UTF-16 code
  units, and interoperation with that standard is not offered. The wire names stay
  unchanged for now; renaming an export is a separate, larger contract change.
- **H-5** `decided`. The authoritative hash for identifying a compiled form is the
  compiled triple's `contentHash` (H-3), which is order-insensitive.
  `colander_content_hash` (H-2) is a byte-exact fingerprint of the documents as
  given: it moves when key order or number spelling moves, and it is not used for
  pinning.

## P — Compilation, components and semver

- **P-1** `live`. A `component-ref` field is replaced by a group; the component is
  resolved by exact `(code, version)` match against the batch the caller supplies;
  resolution is memoized; a reference cycle is an error. Every other property of
  the reference field is dropped. There is no repository and no callback.
- **P-2** `live`. `dependencyMetadataJson` lists the resolved components sorted by
  `code` then version, plus the calculated field ids and the evaluation order. A
  component's missing `contentHash` becomes the empty string.
- **P-3** `live`. A component's `contentHash` is verified: when it is a non-empty
  string it is recomputed from that component's own compiled triple — nested
  `component-ref` fields expanded and no rules document — and compared, and a
  mismatch is a hard error `COMPONENT_HASH_MISMATCH`. An absent key or an empty
  string is not a pin and is carried into `dependencyMetadataJson` unverified. The
  pin covers exact bytes, so `1.50` and `1.5` hash differently, which is intended.
- **P-4** `live`. `colander_next_version` returns `"1.0.0"` when nothing is
  published, and otherwise increments the patch of the highest published version
  with no carry.
- **P-5** `decided`. The accepted version grammar is exactly three non-negative
  integers under SemVer 2.0.0's numeric-identifier rules: no leading zeros, no
  blank segments, no surrounding whitespace, no `v` prefix. **Pre-release and build
  metadata are not permitted** on a component version, so `1.0.0-beta` is rejected
  by design and `1..0.0` is rejected as malformed.
- **P-6** `decided`. `colander_next_version` takes the intended bump (`patch`,
  `minor` or `major`) and applies SemVer precedence to the highest published
  version. It never infers the bump from a list of version strings.
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
  untouched. Payloads are compared byte-for-byte and an error case only by its
  `SCREAMING_SNAKE` code. No automatic guard covers the vectors, so the affected
  cases are enumerated by hand before each change lands.

## Open items

No requirement is unresolved: `contract-1` carries no `proposed` marker. Two
capabilities are recorded as candidates rather than obligations — structured
schema-validation errors, and a structural classifier that would derive a bump —
and both live in `odd/tasks/public-readiness-roadmap.md`.
