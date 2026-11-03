# The golden vectors

`tests/golden/vectors/` holds 254 recorded cases in eight files: `canonical`,
`hash`, `semver`, `rules`, `validate`, `compile`, `errors` and `describe`. One
file is one group and is replayed by one test binary:

```bash
cargo test                            # everything, vectors included
cargo test --test golden_rules -- --nocapture
```

`--nocapture` prints the per-group tally. The eight groups make 1,524 checks: 19
canonical, 25 hash, 36 semver, 1,105 rules, 189 validate, 108 compile, 29 errors
and 13 describe.

## What is asserted

A case is replayed by calling the entry point named in the vector and comparing
what comes back:

- **Payloads are compared canonically.** Object keys are sorted before
  comparison, so document key order is not asserted; number literals and string
  escaping still are.
- **Errors are compared by contract code only.** A case that failed on the
  recording side must fail on this side with the same `SCREAMING_SNAKE` code —
  `RULE_UNKNOWN_FIELD`, `DISABLED_FIELD_VALUE`, and so on. The surrounding
  message is colander's own prose and is not compared. If a case yields no code,
  only "both sides rejected it" is asserted.

## Vocabulary

The fixtures were recorded from an implementation that used different key names
for the same documents. They now use colander's names, so a document reaches the
code under test exactly as recorded and the harness no longer bridges two
vocabularies.

The `hash` group stores a digest computed over the payload text, so renaming a
key invalidates it. Those digests were recomputed from the documented canonical
payload by an independent implementation, which first reproduced every digest
already recorded. The group therefore still checks colander against a number
colander did not produce. Only names moved: never a value, an ordering or an
error code.

## Frozen against accidental drift

The harness that produced these files is not part of this repository, and the
files cannot be regenerated. A case that fails on its own means colander's
behaviour moved, not that the fixture is stale. Nothing in the repository depends
on where the recording came from.

A **decided** contract change is the one exception. Update the affected
expectation in the same commit, name the decision it implements (the `SPEC.md`
clause or the triage entry) in the commit message, and leave every other case
untouched. Payload cases compare canonically and error cases by code only, so a
decided change usually moves one code and nothing else. No automatic guard covers
these files, so the affected cases are enumerated by hand first.

## Skipped entries

Five entries are not replayed, each listed with its reason in
`tests/common/mod.rs::exclusions`. Three are errors raised by a test helper the
engine never sees, and two are version cases that threw before echoing their
input.

## See also

- Back to the documentation index: [README.md](README.md)
- What a wrapper must preserve: [conformance.md](conformance.md)
