# Breaking change

<!--
Use when a consumer must change their code to keep working. In this crate that means the
C ABI: an exported symbol, a declaration in `include/colander.h`, an error code, the
canonical serialization, or a default. Every breaking change needs a migration path; a pull
request that only says "this changed" cannot be reviewed.
-->

Short paragraph: what breaks, for whom, and which version it lands in.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `breaking-change` — C ABI consumers must change their code

## The old contract

<!-- Exactly what a consumer has today, quoted from the released version — not from the
branch. Name the version this was true for. -->

```c
/* the current released call, symbol by symbol */
```

## The new contract

<!-- Exactly what replaces it. Same shape as the "old contract" table so the two can be
read side by side. -->

```c
/* the new call */
```

## What a consumer must change

<!-- The migration table. Every affected symbol, what it was, what it is now, and the
mechanical change required. A consumer should be able to apply this without reading the
diff. -->

| Symbol / key                         | Before | After | Required change |
| ------------------------------------ | ------ | ----- | --------------- |
| `colander_…` in `include/colander.h` |        |       |                 |

<!-- Name the migration step by step, and say what a caller must do when it has no
affordable fix — a wrapper to keep, a pin to an older version, a codemod. -->

## What was decided

<!-- The ABI, the error codes and the canonical serialization are a published contract:
changing one is a breaking change, not a refactor. Name the `SPEC.md` clause, the triage
entry or the issue this implements, and say which frozen vectors moved as a result. The
vectors are compared byte-for-byte for payloads and by `SCREAMING_SNAKE` code for errors,
so a decided change usually moves one expectation and nothing else. -->

- Decision this implements:
- `tests/golden/vectors/` files touched: <!-- list, or "none" -->

## Affected version range

<!-- The versions that keep working, the versions that break, and the first version with
the new behavior. Note that this crate is `0.1.0` and bumps pre-1.0 without crossing
1.0.0, so a breaking change here is a minor bump, not a major one. -->

- **First version with the change:**
- **Works unchanged from:** <!-- version range -->
- **Breaks from:** <!-- version -->
- **Old path deprecated, not removed:** <!-- version, or "not scheduled" -->

## Deprecation plan

<!-- If the old call is kept temporarily, name how it is kept, what the default is, and when
it disappears. Otherwise write "No deprecation window: the old call is removed in the same
release." -->

## Checks run

<!-- These are the checks `cargo make ci` runs for every pull request. All of them must pass,
and the output pasted into the pull request, not summarised. -->

- [ ] `cargo test` — unit and frozen-vector suites
- [ ] `cargo clippy --all-targets -- -D warnings` — lint
- [ ] `cargo make fmt-check` — rustfmt and dprint
- [ ] `cargo make header-check` — generated header matches the sources
- [ ] `cargo build --release` — the cdylib builds
- [ ] The migration step above was applied and verified against the new API

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/breaking-change/rename-describe-form`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages use Conventional Commits with the `!` marker (for example
      `feat!: …`) or a `BREAKING CHANGE:` footer, so release-please sees the bump
- [ ] No `Co-Authored-By` trailers
- [ ] `include/colander.h` regenerated with cbindgen, never hand-edited
- [ ] The migration path is copy-pasteable and was tested
- [ ] `docs/abi.md` and the affected `docs/` pages updated with the change
- [ ] All CI checks pass, with the output pasted
