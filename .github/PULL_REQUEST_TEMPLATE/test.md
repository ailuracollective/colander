# Tests

<!--
Use for test-only changes: new coverage, test harness work, new fixture files, or new
benchmarks. If the code under test also changes, say so explicitly below and pick the label
that matches the dominant intent instead.
-->

Short paragraph: which behavior is now covered that was not before.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `test` — tests only, or tests plus non-behavioral test scaffolding

## Scope of this change

<!-- State plainly whether the code under test was modified. A test pull request that
silently changes source is not reviewable. -->

- [ ] Test files only — the code under test is unchanged
- [ ] Test files plus the code under test — behavior is expected to stay identical; explain
      below why the source change was unavoidable

| Source file changed | Why the test change required it |
| ------------------- | ------------------------------- |
| `path/to/file`      |                                 |

## Newly covered

<!-- One row per behavior, not per assertion. "Behavior" here means an internal property of
the crate — a branch in the engine, a rule kind, a codec round trip, a JSON edge case. A
test pull request does not have to describe what a consumer sees; that belongs to `feat`
and `fix`. -->

| Behavior covered | Test file      | Test case | Kind                  |
| ---------------- | -------------- | --------- | --------------------- |
|                  | `path/to/file` |           | unit / vector / bench |

## Frozen vectors

<!-- `tests/golden/vectors/*.json` were recorded once from an external implementation and
cannot be regenerated. A vector that fails on its own means behavior moved, not that the
fixture is stale. -->

- [ ] No file under `tests/golden/vectors/` was touched
- [ ] A vector was added or an expectation moved — <!-- this is a decided contract change:
      name the `SPEC.md` clause or triage entry it implements, and say it is not a `test` PR -->

## Existing coverage that moved

<!-- What was reorganized, deleted or weakened. Silently dropping a case is a regression in
the test suite even when the suite still passes. -->

- [ ] Nothing was removed or weakened
- [ ] Something moved or was removed — <!-- what, where it went, and why -->

## Why library behavior is unchanged

<!-- The defining claim of a `test` pull request. Be concrete: which source files are
byte-identical, or what the only source edit was and why it cannot change a result. -->

-

## Deliberately uncovered

<!-- What you chose not to test, and why. "Nothing" is a valid answer only when the edges
were actually considered. Acceptable: nondeterminism, a boundary that is genuinely
platform-specific, behavior already covered at a higher level, cost out of proportion to
risk. Unacceptable: no reason given. -->

| Uncovered behavior | Why it is left untested |
| ------------------ | ----------------------- |
|                    |                         |

## Checks run

<!-- These are the checks `cargo make ci` runs for every pull request. All of them must pass,
and the output pasted into the pull request, not summarised. -->

- [ ] `cargo test` — unit and frozen-vector suites
- [ ] `cargo clippy --all-targets -- -D warnings` — lint
- [ ] `cargo make fmt-check` — rustfmt and dprint
- [ ] `cargo make header-check` — generated header matches the sources
- [ ] Confirmed each new test fails without the change it protects

<!-- A new test that passes on the base revision is not testing the thing it claims to. If
one was kept, say which and why. -->

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/test/unicode-escape-cases`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] Stated what is deliberately left uncovered, and why
- [ ] All CI checks pass, with the output pasted
