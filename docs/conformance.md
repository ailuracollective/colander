# Conformance

This page tells a wrapper author (WASM, TypeScript, .NET, or anything new)
what must hold for the wrapper to be conformant with the Rust core. The
wrappers live outside this repository; this page is the contract they are
measured against, and the frozen vectors are the corpus that measures them.

## What must be preserved

A conformant wrapper is a faithful transport of the core, not a second
implementation of its logic: it must not reimplement rule evaluation,
validation, hashing, or versioning.

- **Value domain.** Integers and doubles compare numerically: a computed
  `3.0` matches a submitted integer literal `3` (tolerance `1e-6`
  absolute, see [rules.md](rules.md)). A wrapper that decodes all JSON
  numbers to one floating type still conforms as long as integral values
  compare equal to whole doubles and non-integral values keep their
  spelling through `format_double` semantics ([json.md](json.md)).
- **Nullability.** An absent key and an explicit `null` are both "empty"
  for `required` purposes, and both are dropped from the normalized
  output ([validation.md](validation.md)). A wrapper must omit absent
  values rather than serialise them as `null` on the request side (SPEC
  C-6 in [../SPEC.md](../SPEC.md)).
- **Errors.** Branch only on `error.kind` (`invalid_request`,
  `validation`, `panic`) and on the `SCREAMING_SNAKE` code prefix of the
  message. Message wording is not contractual ([validation.md](validation.md)).
- **Hashes.** `colander_content_hash` is byte-exact (key order and number
  spelling matter); the `contentHash` from `colander_compile` is
  order-insensitive. The two must never be substituted for each other
  ([json.md](json.md)).
- **UTF-8 and escaping.** Requests and envelopes are NUL-terminated UTF-8;
  string escaping follows [json.md](json.md), not the host language's
  default JSON writer. A host writer that emits `\/` or lowercase `\u`
  escapes produces bytes the core would not produce.

## The conformance corpus

`tests/golden/vectors/*.json` plus the replay rules in
`tests/common/mod.rs` are the corpus: payloads compare canonically
(key-sorted), error cases compare by contract code only. A wrapper
replays the same corpus against its own binding and must observe the
same outcomes. What the corpus pins — and what it deliberately leaves
to the message wording — is described in [vectors.md](vectors.md).

## What is not covered here

Expression-language parity with survey-style front ends (their
`visibleIf`/`calculate` dialects versus the `op`/`args` AST in
[rules.md](rules.md)) is a translation concern of the integrating layer,
not of this core. This page covers core-to-wrapper fidelity only.

## Next

- The boundary being wrapped: [abi.md](abi.md)
- The value semantics being preserved: [rules.md](rules.md)
- The corpus being replayed: [vectors.md](vectors.md)
