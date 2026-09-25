# Concepts

This page explains the model behind colander — what a form engine does, how a
call is shaped, and how the three documents fit together — before any language
or ABI detail.

## What problem a form engine solves

A form has three moving parts: the fields it is built from, the way those fields
are presented, and the rules that decide what is visible, enabled, required or
computed. colander owns the pure, deterministic half of that work: schema
compilation, the rule engine, response validation and a JSON-Schema subset
validator. It compiles a form, evaluates its rules, validates a response against
them, and hashes the result.

## Stateless, no I/O, no schemas of its own

colander holds no state, performs no I/O and ships no schemas of its own.
Several mechanical consequences follow, and they are worth internalising before
you write a binding:

- Every call is independent. A request must carry the documents it needs; the
  library does not remember a previous call.
- There is no repository and no callback. When you compile a form, the caller
  decides what "published" means by choosing which component versions to hand
  in.
- When you validate a document, the JSON Schemas that describe it travel with
  the request. `colander_validate_schema` validates against a schema you supply,
  not against one bundled with the core.
- There is no async runtime, and the crate has no path or development dependency.

## The request/response model

Every `char *`-returning entry point takes **one NUL-terminated UTF-8 JSON
request string** (or `NULL`) and returns **one NUL-terminated UTF-8 JSON
envelope**. The envelope is either `{"ok":true,"result":{…}}` or
`{"ok":false,"error":{"kind":"…","message":"…"}}`. Check `ok` before reading
`result`; no function returns `NULL`, so `NULL` is never a valid error test. The
detailed contract lives in [abi.md](abi.md), and the request and response keys
for each function live in [entry-points.md](entry-points.md).

## The three documents

A form is described by three JSON documents. They share one version lineage and
are usually compiled together.

The **form schema** defines the field tree. It carries `schemaVersion`,
optionally `$schema`, and a `fields` array. Every field has a `type`, an `id`
and a `code`.

The **UI schema** describes presentation. Its `fields` object is keyed by field
**id**, and of the whole document the core reads only `hidden`. It also carries a
`layout` array that `colander_compile` rewrites when it expands component
references.

The **rules schema** attaches behaviour to fields. Its `fields` object is also
keyed by field **id**; each entry may carry `visibleWhen`, `enabledWhen`,
`requiredWhen` and `calculate`. A `validations` array holds cross-field checks.
When both a form and a rules document carry a version and the two differ, that
is an error.

`colander_compile` is what ties them together: it expands `component-ref` fields,
canonicalizes all three documents, produces `dependencyMetadataJson` (the
component list plus rule evaluation order) and hashes the compiled triple. See
[documents.md](documents.md) for the schemas and [rules.md](rules.md) for the
rule engine.

## `id` versus `code`

This is the single easiest thing to get wrong.

A field **`id`** is the internal name of a field. The UI schema's `fields` object
is keyed by it, the rules schema's `fields` object is keyed by it, and the three
boolean maps returned by `colander_evaluate_rules` — `visibility`, `enabled` and
`required` — are keyed by it.

A field **`code`** is the answer key. The `values` object you pass to
`colander_evaluate_rules` is keyed by field code, `calculatedValues` is keyed by
field code, and expressions resolve references against field codes.

In plain words: `id` is how the form talks about a field internally; `code` is
how an answer is labelled. Rules and UI reference fields by `id`; answers and
calculated values are keyed by `code`. The two are read with different
strictness rules, too — see [documents.md](documents.md).

## Caller-side field types

A related mechanical fact: colander accepts exactly thirteen field type names,
matched case-sensitively, and **no aliases**. Callers that use names such as
`email`, `bool`, `dropdown` or `section` must convert them to one of the supported
types before submitting a document.

## Next

- The ten-minute path: [getting-started.md](getting-started.md)
- The wire contract: [abi.md](abi.md)
- The document schemas: [documents.md](documents.md)
- Plain-language terms: [glossary.md](glossary.md)
