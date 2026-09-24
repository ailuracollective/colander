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

## The cost a wrapper inherits

Measured on 2026-09-25 (same request, same library, native C ABI vs
`wasm32-unknown-unknown` driven from Node):

| Workload             | Native | WASM (call + free) | Ratio |
| -------------------- | -----: | -----------------: | ----: |
| evaluate, 3 fields   | 6.0 µs |            30.9 µs |  5.2× |
| evaluate, 300 fields | 202 µs |           1 679 µs |  8.3× |

The transport is not the cost: `TextEncoder` + `TextDecoder` for the same
payload is 1.5 µs, and the full WASM turn (encode, `colander_alloc`, call,
read, `colander_free_string`, `colander_free_buffer`) is 34.3 µs against
30.9 µs for the call alone. The 5–8× is the core compiled to wasm32. A
wrapper that keeps the schema and answers hot (and recompiles rarely) pays it
once per validation, not per keystroke.

Two obligations belong to the wrapper, and the core cannot enforce either:

- **Free what you allocate.** A caller that forgets
  `colander_free_buffer` for its request buffer leaks roughly 440 bytes per
  call, unbounded (measured: 1.5 MB → 45.5 MB over 100 000 calls). With
  correct pairing, 20 000 calls on each of five entry points grew the WASM
  heap by 0 bytes. `colander_free_buffer` must receive the _same_ length that
  was requested; a wrong length corrupts the heap (it is undefined behaviour,
  not a recoverable error).
- **Do not free twice, and do not free foreign memory.**
  `colander_free_string` takes only pointers from a `colander_*` return.

## The conformance corpus

`tests/golden/vectors/*.json` plus the replay rules in
`tests/common/mod.rs` are the corpus: payloads compare canonically
(key-sorted), error cases compare by contract code only. A wrapper
replays the same corpus against its own binding and must observe the
same outcomes. What the corpus pins — and what it deliberately leaves
to the message wording — is described in [vectors.md](vectors.md).

## Running the corpus

`tests/conformance_corpus.rs` pins the expected envelope of every case below.
A wrapper reproduces them by driving its own binding with the same requests
and comparing bytes. On 2026-09-25 all 14 cases matched byte-for-byte between
the native C ABI (driven from CPython) and the wasm32 build (driven from
Node) — that equivalence is the asset, not the Rust test suite.

## What is not covered here

Expression-language parity with survey-style front ends (their
`visibleIf`/`calculate` dialects versus the `op`/`args` AST in
[rules.md](rules.md)) is a translation concern of the integrating layer,
not of this core. This page covers core-to-wrapper fidelity only.

## Next

- The boundary being wrapped: [abi.md](abi.md)
- The value semantics being preserved: [rules.md](rules.md)
- The corpus being replayed: [vectors.md](vectors.md)
