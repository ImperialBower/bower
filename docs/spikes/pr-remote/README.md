# pr-remote — the experiment behind EPIC-09 slice 2

Slice 2 of `docs/EPIC-09_Branches.md` puts pull requests on the forge. Two of
its open questions are about the forge, not about Bower, and only a real
remote can answer them:

- **Q1** — Bower merges by pushing a main that already contains the merge
  commit, never on the forge. Does GitHub then mark the PR **merged**?
- **Q2** — a rebuild gives every branch new SHAs, and `bower push`
  force-pushes them. What happens to an open PR, and to a merged one?
- **Q3** (optional) — does the first branch pushed to an *empty* repository
  become its default? `push_commands` pushes main first on a fresh remote on
  the assumption that it does (EPIC-09 corrigendum item 4).

`book/` is a one-chapter book: an abandoned branch with a PR, a branch that
is merged, and main moving in between. `run.sh` builds it three times — as
written, with a change on the merged branch, and with a change to the first
step — and pushes each build to `abstecker/bower-sandbox` with plain git,
opening PRs with `gh` along the way. Every run gets branch names of its own
(`@RUN@` in the chapter becomes a timestamp), so runs never meet.

```sh
DRY=1 docs/spikes/pr-remote/run.sh        # build and print; pushes nothing
docs/spikes/pr-remote/run.sh              # Q1 and Q2, against the sandbox
EMPTY_REPO=owner/empty docs/spikes/pr-remote/run.sh   # also Q3
```

It force-pushes the sandbox's `main`, and refuses any other repository. Each
run writes `results-<run>.md` here; the findings go into the EPIC's open
questions table, and the results files need not be kept.
