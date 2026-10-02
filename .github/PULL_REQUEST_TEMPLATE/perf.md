# Performance

<!--
Use for a deliberate, measured improvement. A `perf` pull request without numbers cannot be
reviewed: "it feels faster" is not evidence. If the numbers below cannot be produced, open
a discussion issue instead of opening this pull request.
-->

Short paragraph: which workload this makes cheaper and why that workload matters.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `perf` — measurable performance improvement

## Measurements

<!-- Every cell is a measured value, not an estimate. If a metric did not move, say so
explicitly; a row of unchanged numbers is a useful result. -->

| Metric                                    | Before | After | Delta | Noise margin |
| ----------------------------------------- | ------ | ----- | ----- | ------------ |
| `benches/engine.rs` — `evaluate_rules`    |        |       |       |              |
| `benches/codecs.rs` — `decode` / `encode` |        |       |       |              |
| `benches/schema.rs` — `compile`           |        |       |       |              |

## Measurement method

<!-- The exact, copy-pasteable command, and how the numbers were taken. The three bench
targets are `harness = false` and time operations with `std::time::Instant` around
`std::hint::black_box`, so a run is a plain `cargo bench` and `BENCH_SCALE` lengthens the
target batch for a more stable sample. Record the scale you used — a number from a
different scale is not comparable. -->

```bash
# exact benchmark command, with BENCH_SCALE and the base revision
```

## Environment

<!-- Numbers are not comparable across machines. Record everything below. -->

- Machine / CPU:
- Memory:
- OS and kernel:
- `rustc --version`:
- Branch and commit measured for "before":
- Branch and commit measured for "after":
- Samples and `BENCH_SCALE` used:

## Regression threshold

<!-- Name the threshold that would have made this a regression, and confirm the new value is
above it. A threshold that was chosen after seeing the number proves nothing. -->

- Regression threshold:
- This change's value against that threshold:

## What was made cheaper

<!-- The mechanism, not the numbers: what work was removed, which allocation disappeared,
which pass no longer happens. 1 to 3 bullets. -->

-

## Behavior is unchanged

<!-- A speedup that changes output is a `feat` or a `fix`, not a `perf`. -->

- [ ] `cargo test` passes unchanged, including every frozen vector group
- [ ] The canonical serialization is byte-for-byte identical
- [ ] No error code changed

## Checks run

<!-- These are the checks `cargo make ci` runs for every pull request. All of them must pass,
and the output pasted into the pull request, not summarised. -->

- [ ] `cargo test` — unit and frozen-vector suites
- [ ] `cargo clippy --all-targets -- -D warnings` — lint
- [ ] `cargo make fmt-check` — rustfmt and dprint
- [ ] `cargo make header-check` — generated header matches the sources
- [ ] `cargo bench` — before and after, recorded above

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/perf/cache-compiled-triples`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] Before and after numbers measured, with the environment and the scale
- [ ] No dependency added to obtain the improvement
- [ ] All CI checks pass, with the output pasted
