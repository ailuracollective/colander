# Build

<!--
Use for the build system, the toolchain and packaging: `Cargo.toml`, `Cargo.lock`,
`Makefile.toml`, `cbindgen.toml`, the pinned tool versions, the cdylib, the published crate
contents. A workflow or automation change is `ci`; a documentation-only change is `docs`.
-->

Short paragraph: what in the build changed and why.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `build` — build system, toolchain, packaging

## What changed

| File           | Change                                         |
| -------------- | ---------------------------------------------- |
| `path/to/file` | Task, target, pin or manifest field that moved |

## Why it was needed

<!-- The build problem this solves: a task that does not exist, a version that drifted, a
crate that shipped a file it should not, a header that can no longer be regenerated. -->

## Version changes

<!-- This crate pins its tooling by version: cargo-make 0.37.24, dprint 0.57.4, cbindgen
0.29.4, and a moving tag for every `uses:` in a workflow. A version bump must be listed
here with the reason, and it must be the same number the workflow installs and the
developer has locally. -->

| Tool | Before | After | Why |
| ---- | ------ | ----- | --- |
|      |        |       |     |

- [ ] Every version bumped here is also bumped in `.github/workflows/ci.yml` and in the
      local setup notes
- [ ] No dependency was added without raising it as a decision first

## Packaging and the published crate

<!-- `Cargo.toml`'s `exclude` decides what a consumer downloads. If it changed, say what
left or entered the published crate, and confirm `benches/` is still shipped — the three
`harness = false` bench targets have to have their sources in the package. -->

- [ ] The published file set is unchanged, or the change is intended
- [ ] `include/colander.h` was regenerated with cbindgen if the ABI surface moved, never
      hand-edited
- [ ] `cargo package` succeeds and the packed file list was inspected

## Verified

```bash
cargo make ci
```

- Result: <!-- paste the summary, do not summarise -->

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/build/pin-cbindgen-version`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] `Makefile.toml` is still the only place a command line is written
- [ ] All CI checks pass, with the output pasted
