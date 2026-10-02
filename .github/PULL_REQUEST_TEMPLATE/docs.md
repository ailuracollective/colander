# Documentation

<!--
Use for documentation-only changes. If anything outside documentation changes in any way —
a code comment, a snippet in a doc, a config default, a generated header — this is not a
`docs` pull request.
-->

Short paragraph: which documentation gap or inaccuracy this closes.

## Linked issue (required)

<!-- The linked issue MUST carry the `status:approved` label, applied by a maintainer once
it is triaged. Put exactly one closing keyword on its own line. `Refs #N` does NOT close
the issue and does NOT satisfy this check. -->

Closes #

## Type (required)

<!-- Check exactly ONE. Labels are bare Conventional Commit types with no `type:` prefix:
feat, fix, docs, chore, refactor, test, ci, build, perf, breaking-change -->

- [ ] `docs` — documentation only, no code changed

## Documentation changed

| File                   | Change                                |
| ---------------------- | ------------------------------------- |
| `path/to/docs/file.md` | What was added, corrected, or removed |

<!-- Include the reason whenever a section is removed or a recommendation is reversed, so
the reviewer does not have to infer it from the diff. -->

## Why the previous text was wrong

<!-- If the old text was wrong rather than merely missing, say what it claimed and what is
actually true. Quote the sentence, do not paraphrase it — a reader who trusted the old
wording needs to see which claim died. If the section is new, write "Nothing was wrong; the
text did not exist." -->

## Anything else that referenced the old text

<!-- Documentation drifts silently when only one copy is updated. Name every other place
that repeated the corrected claim: a README, an issue form, a pull request template, a
comment in `src/`, a `SPEC.md` clause, a schema in `schemas/`. Write "Nothing" when the
claim appeared once. -->

- Files or strings to update alongside this one: <!-- list, or "nothing" -->

## Read-through check

<!-- A documentation review, not a test run. Confirm the whole changed file reads correctly
in context, not only the edited lines. -->

- [ ] Read the full changed file top to bottom after the edit
- [ ] New instructions were followed literally on a clean checkout
- [ ] Terminology, headings and code fences match the surrounding documents
- [ ] Every command quoted is a real command in `Makefile.toml` or a real `cargo` invocation
- [ ] Every symbol, key and file named in the text exists in the tree
- [ ] Nothing in the diff contradicts `AGENTS.md`, `docs/README.md` or `SPEC.md`

## Link and anchor verification

<!-- Record the checks actually performed. Write "No links or anchors changed" when the diff
introduces none. -->

- [ ] Checked every added internal link resolves
- [ ] Checked every added anchor (`#heading`) matches a real heading
- [ ] Checked external links return a success status
- [ ] No links or anchors changed

## Runtime code untouched

<!-- The defining property of a `docs` pull request. -->

- [ ] Confirmed no code changed: the diff touches only `.md` files and documentation assets
- Files in the diff outside documentation: <!-- list them, or "none" -->

## Contributor checklist

- [ ] Linked an approved issue with `Closes #N`, `Fixes #N` or `Resolves #N`
- [ ] The linked issue carries the `status:approved` label
- [ ] Branch is named `<github-username>/<type>/<description>`, all lowercase
      (for example `janedoe/docs/codec-round-trip-example`)
- [ ] Added exactly one label from the allowed set, with no `type:` prefix
- [ ] Commit messages follow Conventional Commits with one of the nine allowed types
- [ ] No `Co-Authored-By` trailers
- [ ] `cargo make md-check` passes
- [ ] All CI checks pass
