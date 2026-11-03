# Contributing

`AGENTS.md` is the operating manual: the frozen contract, the commands and the
conventions. This file covers one thing: what a pull request must satisfy to be
mergeable.

## Start with an issue

Every pull request links an issue, and a maintainer applies `status:approved` once
it is triaged. A pull request may only link an issue that carries that label, so
the decision happens on the issue and the delivery on the pull request. Close it
with `Closes #N`; `Refs #N` neither closes the issue nor satisfies the check.

## The two gates

Both live in `.github/workflows/policy.yml` and run on every pull request event
that can change their verdict, including a label being added or removed.

| Job                   | What it enforces                                                                                                                                                                                              |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `branch-validation`   | The head ref is `<github-username>/<type>/<short-description>`, with `<username>` matching the pull request author.                                                                                           |
| `pull-request-policy` | The title is a Conventional Commit of 15 to 72 characters, exactly one type label is present, the linked issue carries `status:approved`, and the body has every section the template for that type asks for. |

A third job, `triage`, is not a gate: it applies `status:needs-review` to a new
issue so a triager can find it.

### Exactly one type label

Required, never more than one, as bare names with no `type:` prefix:

`feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`, `build`, `perf`,
`breaking-change`

The list is declared in `.github/labels.yml`, a manifest no automation reads, so
it is mirrored in `policy.yml` and in the `commit-msg` hook. Change one and mirror
the others.

### Templates

There is one template per type in `.github/PULL_REQUEST_TEMPLATE/`; pick the file
that matches your label. A type without one falls back to
`.github/pull_request_template.md`. The body check reads the `##` headings of the
template that applies and requires all of them, so delete nothing structural. A
section you do not need can be answered in one line.

### The one exemption

`AiluraKitty` is exempt from both gates. It is the machine identity that owns
`RELEASE_PLEASE_TOKEN`, so it authors every release pull request release-please
opens. Those pull requests have a generated head ref with no type segment, no
linked issue, no type label and a generated body, so they cannot pass by
construction and no release could merge. Nothing a person wrote is exempted, and
a human still reviews and merges the release pull request.

## Before you push

The git hooks are `hk`, wired by `hk.pkl`.

- `pre-commit` and `pre-push` run the same checks CI does. Paste the real output
  into the pull request; "tests pass" is not evidence.
- `commit-msg` runs `hk util check-conventional-commit` with the nine allowed
  types. A `fixup!`, `squash!` or `amend!` subject is skipped.

The full local CI set is `cargo make ci`; the fast pre-commit set is
`cargo make precommit`.
