# Concepts

The model behind colander: what a form engine does, how a call is shaped, and how
the three documents fit together, before any ABI detail.

## What a form engine solves

A form has three moving parts: the fields it is built from, the way they are
presented, and the rules that decide what is visible, enabled, required or
computed. colander owns the pure, deterministic half of that work: schema
compilation, the rule engine, response validation and a JSON Schema subset
validator. It compiles a form, evaluates its rules, validates a response against
them and hashes the result.

## Stateless, no I/O, no schemas of its own

colander holds no state, performs no I/O and ships no schema of its own. Four
consequences matter when you write a binding:

- Every call is independent. A request must carry the documents it needs; the
  library remembers nothing between calls.
- There is no repository and no callback. When you compile a form, the caller
  decides what "published" means by choosing which component versions to hand in.
- When you validate a document, the JSON Schemas that describe it travel with the
  request. `colander_validate_schema` validates against a schema you supply.
- There is no async runtime, and the crate has no path or development dependency.

## The request/response model

Every `char *`-returning entry point takes one NUL-terminated UTF-8 JSON request
string (or `NULL`) and returns one NUL-terminated UTF-8 JSON envelope:

```json
{"ok":true,"result":{…}}
{"ok":false,"error":{"kind":"…","message":"…"}}
```

Check `ok` before reading `result`; no function returns `NULL`, so `NULL` is never
a valid error test. The boundary contract is in [abi.md](abi.md), and the request
and response keys are in [entry-points.md](entry-points.md).

## The three documents

A form is described by three JSON documents. They share one version lineage and
are usually compiled together.

The **form schema** defines the field tree. It carries `schemaVersion`,
optionally `$schema`, and a `fields` array. Every field has a `type`, an `id` and
a `code`.

The **UI schema** describes presentation. Its `fields` object is keyed by field
**id**, and the core reads only `hidden` there. It also carries a `layout` array
that `colander_compile` rewrites when it expands component references. The core
reads `layout` only when response validation is scoped with `pageId`: a top-level
`type:"page"` node names the fields one page renders.

The **rules schema** attaches behaviour to fields. Its `fields` object is keyed by
field **id**; each entry may carry `visibleWhen`, `enabledWhen`, `requiredWhen`
and `calculate`. A `validations` array holds cross-field checks. When both a form
and a rules document carry a version and the two differ, that is an error.

`colander_compile` ties the three together: it expands `component-ref` fields,
canonicalizes all three documents, produces `dependencyMetadataJson` (the
component list plus rule evaluation order) and hashes the compiled triple. See
[documents.md](documents.md) for the schemas and [rules.md](rules.md) for the rule
engine.

## `id` versus `code`

A field **`id`** is the internal name of a field. The UI and rules `fields`
objects are keyed by it, and so are the three boolean maps returned by
`colander_evaluate_rules`: `visibility`, `enabled` and `required`.

A field **`code`** is the answer key. The `values` object passed to
`colander_evaluate_rules` is keyed by code, `calculatedValues` is keyed by code,
and expressions resolve references against codes.

In plain words: `id` is how the form talks about a field internally, and `code` is
how an answer is labelled. The two are read with different strictness rules; see
[documents.md](documents.md).

## Caller-side field types

colander accepts exactly twelve field type names, matched case-sensitively, with
**no aliases**. Callers that use names such as `email`, `bool`, `dropdown` or
`section` must convert them to one of the twelve before submitting a document.

## See also

- The ten-minute path: [getting-started.md](getting-started.md)
- The wire contract: [abi.md](abi.md)
- The document schemas: [documents.md](documents.md)
