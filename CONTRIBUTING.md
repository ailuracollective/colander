# Contributing

`AGENTS.md` is the operating manual for this repository: the frozen contract, the
commands, and the conventions that differ from the defaults. This file covers one
thing only — what a pull request has to satisfy to be mergeable.

## Start with an issue

Every pull request links an issue, and a maintainer applies `status:approved` to
it once it is triaged. A pull request may only link an issue that carries that
label, so the issue is where the decision gets made and the pull request is where
it gets delivered. Close it with `Closes #N`; `Refs #N` does not close an issue
and does not satisfy the check.

## The two gates

Both live in `.github/workflows/policy.yml` and both run on every pull request
event that can change their verdict, including a label being added or removed.

| Job                   | What it enforces                                                                                                                                                                                              |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `branch-validation`   | The head ref is `<github-username>/<type>/<short-description>`, with `<username>` matching the pull request author.                                                                                           |
| `pull-request-policy` | The title is a Conventional Commit of 15 to 72 characters, exactly one type label is present, the linked issue carries `status:approved`, and the body has every section the template for that type asks for. |

A third job, `triage`, is not a gate: it applies `status:needs-review` to a new
issue so a triager can find it.

### Exactly one type label

Required, and never more than one. Bare names, no `type:` prefix:

`feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`, `build`, `perf`,
`breaking-change`

The list is declared in `.github/labels.yml`, which is a manifest and not read by
any automation, so it is mirrored in `policy.yml` and in the `commit-msg` hook.
Change one and mirror the others.

### Templates

One template per type in `.github/PULL_REQUEST_TEMPLATE/`; pick the file that
matches your label. A type without one falls back to
`.github/pull_request_template.md`. The body check reads the `##` headings of
whichever template applies and requires the body to have all of them, so delete
nothing structural — a section you do not need can be answered in one line.

### The one exemption

`AiluraKitty` is exempt from both gates. It is the machine identity that owns
`RELEASE_PLEASE_TOKEN`, so it is the author of every release pull request
release-please opens. Those pull requests have a generated head ref with no type
segment, no linked issue, no type label and a generated body, so they cannot pass
by construction and no release could ever merge. Nothing a person wrote is
exempted by that account, and a human still reviews and merges the release pull
request.

## Before you push

The git hooks are `hk`; `hk.pkl` wires them up.

- `pre-commit` and `pre-push` run the same checks CI does. Paste the real output
  into the pull request. "Tests pass" is not evidence.
- `commit-msg` runs `hk util check-conventional-commit` with the nine allowed
  types. A `fixup!`, `squash!` or `amend!` subject is skipped by that validator.

The full local CI set is `cargo make ci`; the fast pre-commit set is
`cargo make precommit`.
