# Feature

<!--
Use for new capability a consumer can now reach. A change that alters or removes existing
behavior is NOT a feature — use `breaking-change`. A change that corrects existing behavior
is a `fix`.
-->

Short paragraph: the problem this solves and the capability it delivers. Two or three
sentences, and do not repeat the title.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `feat` — new user-facing capability

## What is new

<!-- The new surface. In this crate that is an exported symbol, a C ABI entry point, a
declaration in `include/colander.h`, a new rule id, a new document or response key, or a
new codec — not a UI component. Link the file that defines each one. -->

| Surface              | Location                            | Purpose        |
| -------------------- | ----------------------------------- | -------------- |
| `colander_something` | `src/ffi/…`, declared in the header | What it is for |

<!-- A new entry point means the generated header changed. Say so here and regenerate it
with cbindgen; never hand-edit `include/colander.h`. -->

## How to try it

<!-- Give the reviewer a runnable path. Prefer a snippet they can paste: a C call against
`target/release/libcolander.so`, a JSON request for an existing entry point, or the exact
`cargo` command. State what they should observe. -->

```c
/* or the exact cargo command, and what output proves it works */
```

## Compatibility impact

<!-- State explicitly whether existing code keeps working unchanged. If it does not, this
belongs in `breaking-change`. -->

- Backward compatible: yes / no
- ABI surface change: <!-- none, additive only, or breaking — and if breaking, this is the
  wrong template -->
- New opt-in behavior: <!-- the flag, key or default, or "none" -->
- Deprecations introduced: <!-- or "none" -->

## Test plan

<!-- These are the checks `cargo make ci` runs for every pull request. All of them must pass,
and the output pasted into the pull request, not summarised. -->

- [ ] `cargo test` — unit and frozen-vector suites
- [ ] `cargo clippy --all-targets -- -D warnings` — lint
- [ ] `cargo make fmt-check` — rustfmt and dprint
- [ ] `cargo make header-check` — generated header matches the sources
- [ ] `cargo build --release` — the cdylib builds
- [ ] Manually exercised the new capability end to end

<!-- Did this add a rule or an entry point? Name the vector group or test that covers it.
`tests/golden/vectors/*.json` is frozen: a new case is a decided contract change and must
name the decision it implements, not be edited to make a failure pass. -->

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/feat/response-projection-cache`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] `include/colander.h` regenerated with cbindgen if the ABI surface changed
- [ ] Documentation under `docs/` updated for the new capability
- [ ] All CI checks pass, with the output pasted
