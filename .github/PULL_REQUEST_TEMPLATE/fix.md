# Bug fix

<!--
Use for a correction of existing behavior. If the defect only exists on unreleased code,
this is usually a `chore` or a `revert` instead.
-->

Short paragraph: the defect this corrects, in one or two sentences.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `fix` — correction of existing behavior

## Symptom

<!-- What a caller observes: the wrong envelope, the wrong `SCREAMING_SNAKE` error code, the
panic, the hang. Observable behavior only — the root cause does not belong here. -->

## Root cause

<!-- Why the code produced the symptom, including the conditions that trigger it. A fix
that only patches the symptom is not reviewable. If the fix changes a frozen vector's
expectation, this is a decided contract change: name the `SPEC.md` clause or triage entry
it implements. -->

## Minimal reproduction

<!-- The smallest input or call sequence that reproduces the defect on the base revision. -->

1.
2.
3.

```json
{ "request": "the smallest request that shows it" }
```

## Expected vs actual

|                   | Behavior                                               |
| ----------------- | ------------------------------------------------------ |
| **Expected**      |                                                        |
| **Actual**        |                                                        |
| **Base revision** | <!-- commit or version where the defect is present --> |

## Regression coverage

- [ ] Added a regression test
- [ ] Not added — <!-- explain why no test can cover this defect; a frozen-fixture or
      environment limitation is a valid reason, "no time" is not -->

<!-- If added, name the file and the case. A unit test in `src/` or a case in an existing
`tests/` file both count. Note explicitly if the coverage is a frozen vector: those files
cannot be regenerated, so a new case there is a decided contract change. -->

| Test file      | Test case       |
| -------------- | --------------- |
| `path/to/file` | What it asserts |

## How to verify the fix

<!-- The reviewer should be able to re-run the reproduction and see it stop failing. -->

- [ ] The reproduction above no longer reproduces on this branch
- [ ] The reproduction still fails on the base revision

## Test plan

<!-- These are the checks `cargo make ci` runs for every pull request. All of them must pass,
and the output pasted into the pull request, not summarised. -->

- [ ] `cargo test` — unit and frozen-vector suites
- [ ] `cargo clippy --all-targets -- -D warnings` — lint
- [ ] `cargo make fmt-check` — rustfmt and dprint
- [ ] `cargo make header-check` — generated header matches the sources
- [ ] Confirmed the reproduction steps no longer reproduce the defect

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/fix/canonical-key-order`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] No file under `tests/golden/vectors/` was edited, or the decided contract change is
      named
- [ ] Documentation updated if observable behavior changed
- [ ] All CI checks pass, with the output pasted
