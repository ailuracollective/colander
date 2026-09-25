# The golden vectors

`tests/golden/vectors/` holds 254 recorded cases in eight files —
`canonical`, `hash`, `semver`, `rules`, `validate`, `compile`, `errors`,
`describe`. One file is one group and is replayed by one test binary:

```bash
cargo test                            # everything, vectors included
cargo test --test golden_rules -- --nocapture
```

`--nocapture` prints the per-group tally, and the eight groups make 1524 checks:
19 canonical, 25 hash, 36 semver, 1105 rules, 189 validate, 108 compile, 29
errors, 13 describe. (The total was 1511 across the seven groups before
`describe` was added; the page previously said 1648, which nothing had checked
and which no run reproduced.)

## What is asserted

A case is replayed by calling the entry point named in the vector and comparing
what comes back:

- **Payloads are compared byte-for-byte.** The comparison happens on the raw
  text, so key order, number literals and string escaping are all part of the
  contract, not incidental formatting.
- **Errors are compared by contract code only.** A case that failed on the
  recording side has to fail on this side with the same `SCREAMING_SNAKE` code —
  `RULE_UNKNOWN_FIELD`, `DISABLED_FIELD_VALUE`, and so on. The surrounding
  message is colander's own prose and is not compared. If a case yields no code at
  all, only "both sides rejected it" is asserted.

## Vocabulary

The fixtures were recorded from an implementation that used different key names
for the same documents. They now speak colander's own names, so a document reaches
the code under test exactly as recorded and the harness no longer bridges two
vocabularies.

The `hash` group stores a digest computed over the payload text, so renaming a
key invalidates it. Those digests were recomputed from the documented canonical
payload by an independent implementation, which reproduced every digest that had
already been recorded before it was trusted with the new ones. The group
therefore still checks colander against a number colander did not produce. Only names
moved: never a value, an ordering or an error code.

## Frozen against accidental drift

The harness that produced these files is not part of this repository, and the
files cannot be regenerated. A case that fails on its own means colander's
behavior moved, not that the fixture is stale. Nothing in the repository depends
on where the recording came from.

A **decided** contract change is the one exception. When the behavior is changed
on purpose, the affected expectation moves with it: update it in the same commit,
name the decision it implements (the `SPEC.md` clause or the triage entry) in the
commit message, and leave every other case untouched. Payload cases are compared
byte-for-byte, and an error case is compared by its `SCREAMING_SNAKE` code only,
so a decided change usually moves one code and nothing else. No automatic guard
covers these files, so the affected cases are enumerated by hand first.

## Skipped entries

Five entries are not replayed. Each one is listed with its reason in
`tests/common/mod.rs::exclusions`; three are errors raised by a test helper the
engine never sees, and two are version cases that threw before echoing their
input.

## Next

- Back to the documentation index: [README.md](README.md)
