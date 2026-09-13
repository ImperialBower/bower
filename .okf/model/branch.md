---
type: Domain Concept
title: Branch
description: A line of history the fold keeps a tree for — forked from main, merged back with a real two-parent commit, and left as a ref a reader can check out.
tags: [model, kernel, git]
timestamp: '2026-09-13T00:00:00Z'
---

# What it is

A book is normally one straight line: every [step](/model/step.md) folds onto
the one before it. `branch="name"` on a tree step's directive puts that step
on a line of its own instead. `Line` is `Main` or `Branch(String)`, carried on
every `PlannedStep`; `plan()` becomes a fold with one tree per open line, not
one tree for the whole repo. This is the model the reader gets a *shape* of
history from, not just a diary of it: an abandoned experiment survives as a
branch, and a change can be shown as the sequence a pull request would be.

# Forking

A branch forks from main's head at its first step in plan order — document
order already says "the branch starts here". `from="step-id"` overrides it,
naming the main step to fork from instead; it is the only way to demonstrate
a conflict, since forking earlier lets main move underneath the branch before
the merge. `from=` on anything but a branch's first step is refused —
`FromNotOnMain` when it names a step that is itself on a branch,
`FromOnLaterStep` everywhere else a step that is not a branch's first step
carries it, because nothing reads it there and a key nothing reads is a typo.
Naming a step that does not exist is `UnknownFrom`.

# Merging

`merge="name"` on a main step's directive makes that step the branch's merge:
a commit with two parents — main's head and the branch's — whose tree is the
branch's blocks re-applied over main as it stands, then any resolution blocks
the merge step itself carries (`op="none"` for a pure merge with no
resolution). The merge is semantic, not textual: Bower's model is ops, a
three-way text merge would be I/O varying by git version, and re-application
keeps the merge tree a pure fold like every other tree in the plan.

A file — in the kernel, a *region* — that both sides touched since the fork is
a conflict unless the merge step replaces it whole; the `composes` rule that
decides is the same one that lets two ordinary steps touch a file peaceably
(EPIC-09 Decision 14). An unresolved conflict is `MergeConflict`, naming the
step, branch, file, and region. A merge's resolution trims only the paths it
actually resolves out of the branch's blocks before re-applying them — a
multi-path `op="delete"` block that resolves one path still drops the rest,
rather than being skipped whole and silently losing them (`trim_resolved`,
`bower-core/src/branch.rs`). A step on a branch after it has merged is
`BranchAlreadyMerged`; merging a branch twice, or one the plan has not seen
yet, is refused the same way (`MergeUnknownBranch`).

# In the repository

Branch names are validated at plan time against git's own ref-name rules,
and against Bower's: `main`/`HEAD` in any letter case, anything under
`main/`, a name that is a `/`-prefix of another branch in the book, and two
names equal only in case are all `InvalidBranchName` — each would either
collide with main's own ref or make one branch's ref directory swallow
another's (EPIC-09 Decision 6). Replay writes `refs/heads/<branch>` at every
branch step and advances it there; the ref persists after the branch merges,
so `git checkout <branch>` still works. Every branch commit carries a
`Bower-Line: <branch>` trailer and a merge carries `Bower-Merges: <branch>`; a
commit on a straight line carries neither, so a book without branches replays
to the SHAs it always did. `STEPS.md` lists every step on every line, a
branch row's subject gaining `· on <branch>` and a merge row's `· merges
<branch>`. [`bower status`](/commands/status.md) compares branches by name,
the way it already compares tags — a branch pointing at the wrong commit
reads as in sync until a rebuild (Decision 17).

# On the page

A branch step's footer names its branch with a link; a pure `op="none"`
merge has no code block and so would render nothing at all without help, so
every merge step renders one `step-meta` line at its anchor naming the step,
the branch it merges, and a compare link built from its two parents' tags —
shown once, there, never repeated in a later footer.

# Citations

[1] `bower-core/src/branch.rs`, `bower-core/src/plan.rs`
[2] `docs/EPIC-09_Branches.md`
