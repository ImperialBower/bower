# The sandbox

`run.sh` replaces `@RUN@` with a run id before building, so every run gets
branch names of its own and no run can meet another's pull requests.

<!-- bower repo="sandbox" step="base" file="notes.txt" msg="base: a file on main" -->

```text
base
```

<!-- bower repo="sandbox" step="try-it" branch="try/abandon-@RUN@" file="try.txt" msg="try: an experiment we abandon" -->

```text
tried
```

<!-- bower repo="sandbox" branch="try/abandon-@RUN@" pr="An abandoned experiment" -->

```markdown
Left open on purpose.
```

<!-- bower repo="sandbox" step="feat-one" branch="feat/merge-me-@RUN@" file="feat.txt" msg="feat: first commit on the branch" -->

```text
one
```

<!-- bower repo="sandbox" step="feat-two" branch="feat/merge-me-@RUN@" file="feat.txt" op="append" msg="feat: second commit on the branch" -->

```text
two
```

<!-- bower repo="sandbox" branch="feat/merge-me-@RUN@" pr="A branch Bower merges" -->

```markdown
Merged by a push to main, never on the forge.
```

<!-- bower repo="sandbox" step="main-moves" file="notes.txt" op="append" msg="main: meanwhile" -->

```text
meanwhile
```

<!-- bower repo="sandbox" step="merge-it" merge="feat/merge-me-@RUN@" op="none" msg="merge: feat/merge-me" -->
