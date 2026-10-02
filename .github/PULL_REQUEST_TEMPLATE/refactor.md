# Refactor

<!--
Use for an internal restructuring that does not change observable behavior. If the public
API, the C ABI, an error code or the canonical serialization changes at all, this is a
`feat`, a `fix` or a `breaking-change` — not a refactor.
-->

Short paragraph: what was restructured and why the previous shape was a problem.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `refactor` — internal restructuring, behavior unchanged

## Structure: before and after

<!-- Show the shape, not the whole file. This crate is deliberately flat, one file per
domain with one submodule per concern, so a refactor that adds a layer, a plugin trait or a
directory hierarchy needs a stronger justification than "it reads better". -->

**Before**

```
<module layout>
```

**After**

```
<module layout>
```

| Unit                  | Before         | After              |
| --------------------- | -------------- | ------------------ |
| `path/to/file` — unit | where it lived | where it lives now |

## Proof that behavior is unchanged

<!-- "It looks the same" is not a proof. Name what already covered the old shape and still
passes. `cargo test` replays frozen vectors across eight groups, so a refactor that touches
the engine, the rules, the JSON layer or the codecs is covered by fixtures that cannot be
regenerated. -->

| Evidence                                 | Detail |
| ---------------------------------------- | ------ |
| Existing tests covering the old behavior |        |
| Frozen vector groups replayed unchanged  |        |
| Characterization tests added             |        |
| Manual comparison performed              |        |

## Public API and observable behavior

<!-- Required. If anything changed for a consumer, this pull request is mislabeled: move it
to `breaking-change` and say what moved. -->

- Public API unchanged: yes / no
- C ABI surface unchanged: yes / no
- Error codes unchanged: yes / no
- Canonical serialization unchanged: yes / no
- If any answer is "no": <!-- what changed, and which template applies instead -->

## Checks run

<!-- These are the checks `cargo make ci` runs for every pull request. All of them must pass,
and the output pasted into the pull request, not summarised. -->

- [ ] `cargo test` — unit and frozen-vector suites
- [ ] `cargo clippy --all-targets -- -D warnings` — lint
- [ ] `cargo make fmt-check` — rustfmt and dprint
- [ ] `cargo make header-check` — generated header matches the sources
- [ ] `cargo build --release` — the cdylib builds
- [ ] Equivalence verified beyond the test suite

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/refactor/split-rule-evaluator`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] Confirmed no public API, error code or canonical output changed
- [ ] No new abstraction was introduced without a second caller to justify it
- [ ] All CI checks pass, with the output pasted
