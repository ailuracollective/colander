# Continuous integration

<!--
Use for pipelines, workflows and automation: `.github/workflows/*`, `hk.pkl`,
`release-please-config.json`, `.github/labels.yml`, `.github/dependabot.yml`, the git hooks
they wire up. A build-system or toolchain change is `build` instead.
-->

Short paragraph: what automation this changes and why.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `ci` — pipelines, workflows, automation

## What changed

| File                  | Change                                             |
| --------------------- | -------------------------------------------------- |
| `.github/workflows/…` | Job, trigger, permission or `uses:` pin that moved |

## Why it was needed

<!-- The failure this prevents, or the manual step it removes. A CI change justified only by
"it failed once" needs the run link, because the next person will ask whether it can fail
again. -->

- Failure it prevents: <!-- or "none, this is housekeeping" -->
- Link to the run or issue that prompted it: <!-- or "none" -->

## Permissions and secrets

<!-- The part reviewers get wrong. State exactly what each touched job can now do, and
whether a new grant is required. In this repository the only write grant is `issues: write`
in the `triage` job of `policy.yml`; `RELEASE_PLEASE_TOKEN` exists for authorship, not
permissions. -->

| Job or workflow | Permission or secret | Why it is needed |
| --------------- | -------------------- | ---------------- |
|                 |                      |                  |

- [ ] No new write grant was introduced, or each one is justified above
- [ ] Fork pull requests still cannot reach a privileged job

## Verified

<!-- Say how you know the workflow is valid. Parsing the YAML is the floor, not the proof;
where a gate moved, name the job and say which event triggers it. -->

- [ ] Every changed workflow parses as YAML
- [ ] The gate or job was exercised on a real pull request, or the run link is above
- [ ] `Makefile.toml` remains the single source of truth for command lines; no cargo command
      was restated into a workflow or a hook
- [ ] `hk.pkl` and the workflow were not left with two copies of the same rule

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/ci/adopt-policy-gates`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] Any mirrored vocabulary list was updated in every mirror, not only here
- [ ] All CI checks pass
