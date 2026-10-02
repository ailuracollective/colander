# Chore

<!--
Use for maintenance and housekeeping: repository files, metadata, a rename, a housekeeping
script, a non-behavioral cleanup. A build-system or toolchain change is `build`; a workflow
or automation change is `ci`; a reformat that only reformats is still a `chore` here,
because this repository has no `style` type.
-->

Short paragraph: what this changes and why it is needed. Two or three sentences.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `chore` — maintenance and housekeeping, no behavior change

## What changed

| File           | Change                    |
| -------------- | ------------------------- |
| `path/to/file` | What changed, in one line |

<!-- Group mechanical edits into a single row rather than listing every file. -->

## Why it was needed

<!-- The trigger, not the diff: the dead reference, the stale label, the duplicated block,
the housekeeping that keeps falling to somebody. If the honest answer is "tidying", write
that and say what it costs a reviewer to skip. -->

## How it was verified

<!-- The exact command and what it printed. "Ran the tests" is not verification. -->

```bash
cargo make ci
```

- Result: <!-- paste the summary line, do not summarise -->

## Behavior and contract

<!-- A chore must not move the contract. Confirm it rather than assert it. -->

- [ ] No exported symbol, error code or canonical serialization changed
- [ ] `include/colander.h` unchanged, or regenerated with cbindgen if the ABI moved
- [ ] No file under `tests/golden/vectors/` was modified
- [ ] No new runtime dependency was added

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/chore/remove-retired-reference`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] The real command output is pasted, not summarised
- [ ] All CI checks pass
