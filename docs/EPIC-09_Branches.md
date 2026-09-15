# EPIC-09: Branches, merges, and pull requests — history with a shape (BRN)

## Summary

- **Builds:** branches, merges, and pull requests in a generated repo, authored
  with four directive keys: `branch=`, `from=`, `merge=`, `pr=`.
- **Why:** a plan is one straight line, so a book cannot show an abandoned
  experiment or a change as a pull request the reader can check out.
- **Shape:** `plan()` becomes a per-line fold with one tree per open branch; a
  merge is the branch's blocks re-applied over main; `parents` live on the plan
  so replay stays byte-identical.
- **Proves it:** the sixteen spike tests ported by name, plus a golden that a
  change on a branch reaches only what descends from it.
- **Status:** Shipped, 14 September 2026 — slice 1 in PR #6, slice 2 in PR
  #7, and the post-merge review's three fixes in PR #8. The live run on
  `abstecker/hello-playbook` opened `feat/greet-many`'s PR (#1) on the stepping
  stone `step-024-changelog`, and GitHub marked it merged by Bower's own merge
  commit (`4d5f0ee`, `step-025-merge-greet-many`); `try/shout`'s PR (#2) stays
  open. A second push reports `unchanged #2` and `merged #1, left alone` and
  opens nothing (exit criterion 9). Open question 3 stays deferred; nothing
  else is open.

---

## Context

Nine EPICs shipped. A Bower plan is a straight line: `step::order`
(`bower-core/src/step.rs:233`) produces one sequence per repo, `plan()`
(`bower-core/src/plan.rs:141`) folds one `TreeState` forward through it, and
`Replayer::run` (`bower/src/replay.rs:84`) turns it into one chain of commits
on one branch — `BRANCH` (`bower/src/replay.rs:27`) is a constant, and
`Replayer::commit` (`bower/src/replay.rs:170`) takes exactly one parent.

That line cannot say the thing *Rust for Failures* most wants to say: **we
tried it, it failed, we left it there.** A branch that never merges is a
verified broken state a reader can check out and poke at. Nor can it say the
thing *Controllability* wants: **here is the change as a pull request — the
broken first commit, the fix, the merge — every state visible.** Today an
author narrates "imagine a branch" and the reader imagines it. The
generated repository should contain it.

The spike that lived at `docs/spikes/spike-branches/` (zero dependencies,
rustc 1.75, 16 tests) settled the model: a branch is a line of history the
fold keeps a tree for; a merge is a main step whose tree is *the branch's
blocks re-applied over main*; conflicts are plan-time errors; every step's
parents are plan values, so SHAs stay byte-identical across runs and a change
on a branch reaches only what descends from it.
`docs/spikes/spike-branches/fixture-book.md` showed the same six steps as
chapter directives. The directory was deleted in `d99ccf7`, once slice 1
shipped; both files are still readable in the git history at `8f3057f`.

**This EPIC does not** implement branches off branches, merges into
branches, rebases, or review comments as book content; does not merge on the
forge (Bower creates every merge commit, deterministically); does not make
pull requests byte-reproducible — they are forge artifacts, and § Decisions 8
says exactly how far Bower's contract with them goes.

---

## Status

| Component | Slice | Status |
|---|---|---|
| Kernel: four directive keys, `Line`, per-line fold, merge semantics | 1 | **Done** |
| Kernel: `MergeConflict` at region granularity | 1 | **Done** |
| Kernel: `PullRequest` on `RepoPlan.branches`, lock text | 1 | **Done** |
| Testkit: branch textures and the `line` coverage axis | 1 | **Done** |
| Replay: parents from the plan, branch refs, `PULLS.md`, trailers | 1 | **Done** |
| `status`: branch-ref drift | 1 | **Done** |
| Render: line and compare links in the footer; the merge line | 1 | **Done** |
| `push`: branch refs with lease, branches before main | 1 | **Done** |
| Sample book chapter and slow-lane verify | 1 | **Done** |
| `push`: the schedule, stepping stones, `Forge` PR methods, the dry-run schedule | 2 | **Done** |

*Rust for Failures*' `from-char` PR was a slice-2 row until 14 September 2026.
It is book work, not this EPIC's: no EPIC depends on that book, and
`hello-playbook` ch07 is where branches and PRs are proven (Work Item 4b).

Slice 1 is everything local and deterministic: a reader can check out the
abandoned branch, and `bower push` publishes it. Slice 2 is the pull request
on the forge; open questions 1 and 2 needed a real remote (Decision 12), and
`docs/spikes/pr-remote/` answered them for GitHub on 14 September 2026.

---

## Goals

- Let a chapter put steps **on a branch**, **merge** a branch into main, and
  declare a **pull request** for it — with four directive keys and nothing
  else new to learn.
- Keep the kernel **pure and deterministic**: a merge is a fold, a conflict is
  a `BowerError`, parents are numbers in the plan.
- Keep **replay byte-identical** across runs, and keep the blast radius of an
  edit exactly what git's DAG says it is.
- Change **nothing** about verification: every planned step still carries a
  complete tree and its own `expect`.
- Publish branches and PRs through the existing `Forge` seam, decided against
  `FakeForge`, with the network as the last inch — and with the marker gate
  untouched.

## Scope

### Authoring — the four keys

| Key | On | Meaning |
|---|---|---|
| `branch="name"` | any tree step | This step lives on that branch, not on main. |
| `from="step-id"` | a branch's first step | Fork at that main step instead of the nearest preceding one. |
| `merge="name"` | a main step | This step merges that branch. Blocks on it are the **resolution**, applied after the branch's changes. `op="none"` for a pure merge. |
| `pr="Title"` | a block on a branch | Declare a pull request for the branch. The fence is the body (markdown). |

```markdown
<!-- bower repo="failers" step="lookup-table" branch="try/lookup-table" file="src/rank.rs" op="append" expect="test_fail" -->
…
<!-- bower repo="failers" branch="try/lookup-table" pr="Try a lookup table for ranks" -->
```markdown
Faster? Maybe. Correct? The test says no.
```
…
<!-- bower repo="failers" step="merge-from-char" merge="from-char" op="none" -->
```

### What the reader gets

- `git checkout try/lookup-table` — the abandoned experiment, as a branch
  that survives the merge of everything else, with its `test_fail` verified.
- A real merge commit on `main` with two parents, tagged like every step.
- `PULLS.md` beside `STEPS.md`: every PR, its branch, its state, its book
  link — deterministic, in the repo, forge or no forge.
- On the forge: the branches (slice 1), and a PR per declared `pr=`, open or
  merged (slice 2).

### Not in scope

Branches from branches (`FromNotOnMain`), merges into branches
(`MergeOnBranch`), rebases, closing a PR without merging, review comments,
and forge-side merging. Each is a backlog line when this ships — filed 15
September 2026 under `BACKLOG.md` § Deferred from EPIC-09.

---

## Decisions

1. **Four keys, no overloading of `op`.** `merge=` is its own key so a merge
   step can carry ordinary `op="replace"` resolution blocks without `op`
   meaning two things. Spike: `Step { branch, from, merge, pr }`.
2. **A branch forks from main's head at its first step in plan order;
   `from=` overrides.** Document order already says "the branch starts here";
   the override exists for the chapter that wants to fork earlier — which is
   also the only way to *demonstrate* a conflict (Decision 3). Spike test:
   `plan__branch_forks_from_nearest_preceding_main_step`,
   `plan__explicit_from_forks_earlier`.
3. **A merge is semantic, not textual.** The merge tree is the branch's blocks
   since the fork re-applied over main as it stands, then the resolution
   blocks. Bower's model is ops, not diffs; a three-way text merge would be
   I/O, would vary by git version, and would make the merge tree the one tree
   in the plan that is not a pure fold. **Conflict rule:** a file — in the
   kernel, a *region* — both sides touched since the fork is `MergeConflict`
   unless the merge step carries a whole-file replacement for it. The
   resolution is therefore a step the book can narrate, which is the thesis.
   Spike tests: `plan__merge_tree_is_branch_blocks_over_main`,
   `plan__conflict_without_resolution_is_an_error`,
   `plan__resolution_block_clears_the_conflict`,
   `plan__a_main_edit_before_the_fork_is_not_a_conflict`.
4. **Region granularity comes from `check_duplicate_files`.** The rule that
   two distinct regions of one file compose and the same region clashes
   already exists at `bower-core/src/step.rs:198`. The conflict check reuses
   it rather than restating it; the spike works at file level and says so.
5. **Tags are unchanged; branches are refs; `-end` tags anchor on main.**
   `PlannedStep::tag` (`plan.rs:126`) stays `step-{seq:03}-{id}` with `seq`
   global across lines, so a merge always has a larger `seq` than both
   parents. Replay writes `refs/heads/<branch>` at each branch step; the ref
   persists after the merge. `chapter_ends` (`replay.rs:366`) anchors
   `<chapter>-end` on the chapter's last **main** step: a reader who checks
   out `ch01-end` expects the mainline, not an abandoned branch.
6. **Branch names are validated at plan time**, against git's ref-name rules
   as a pure function, and may not be `main` or start with `step-`.
   `InvalidBranchName` carries the location. As built, the check also covers
   names that collide as git refs even when neither is literally `main` or
   `step-`-prefixed: `main`/`HEAD` in any letter case, anything under
   `main/`, one branch name that is a `/`-prefix of another, and two names
   equal only in case — each would otherwise fail late as an unlocated IO
   error or silently repoint main on a case-insensitive filesystem (see the
   corrigendum).
7. **A step on a merged branch is an error**, as is merging twice or merging
   a branch the plan has not seen yet. `BranchAlreadyMerged` names the merge
   step; `MergeUnknownBranch` hints at `after=` when the branch is declared
   later in the book. Spike tests: `plan__step_after_merge_is_refused`,
   `plan__merging_twice_is_refused`, `plan__merge_of_unknown_branch_is_refused`.
8. **A pull request is a plan value first and a forge artifact second.** The
   plan carries `PullRequest { branch, title, body, state }` with `state`
   derived — `Merged` if a merge step exists, else `Open`. `PULLS.md` and
   `bower.lock` are its deterministic homes. On the forge, `push` *ensures*
   the PR: creates it when no PR has that head branch, updates title and body
   while it is open, and leaves a merged one alone. Bower never closes,
   merges, or deletes a PR. The book links the **branch** and the
   **compare**, never a PR number — numbers are the forge's to assign.
9. **Push order is branches, PRs, main, then tags — tags always last.** A PR
   needs its head to exist and needs a diff against base, so branches go
   first; main, which carries the merge commit, goes before the tags rather
   than after them, because a lease trip on main must never leave tags
   already force-rewritten, and tags have no bearing on PR merge detection.
   Both GitHub and Forgejo mark a PR merged when its head becomes reachable
   from base by a push; this is the last inch and open question 1. As built,
   a remote with no `main` yet is the one exception: main is pushed first,
   ahead of every branch, then the branches, then the tags — because the
   first branch pushed to an empty repository likely becomes its default,
   and the push gate reads `STEPS.md` from the default branch — pushing a
   side branch first would lock the book out of every later push (see the
   corrigendum).
10. **Verification does not change.** Every `PlannedStep` still carries a
    complete `tree`, and `verify::run` (`bower/src/verify.rs:376`) already
    checks each in isolation. A merge step is verified against its own
    `expect` like any other.
11. **Exercises follow the line.** `bind_exercises` (`plan.rs:313`) currently
    answers with `planned[idx + 1]` (`plan.rs:358`); it becomes the next step
    on the same line, or the merge step for a merged branch's head. A branch
    head that is never merged has no answer — `ExerciseWithoutAnswer`, the
    existing rule. Play cells and recorded outputs (`bind_outputs`,
    `plan.rs:392`, EPIC-11) bind to a step regardless of line and are
    unchanged.

*Decisions 12–19 were added 13 September 2026, when slice 1 was designed
against the code as EPIC-11 left it.*

12. **Two slices.** Slice 1 is Phases 0–2, the branch half of Phase 3, and
    Phase 4: everything a reader needs to check out a branch, and everything
    `bower push` needs to publish one. `pr=` is part of slice 1 as a plan
    value — parsed, bound, in the lock and in `PULLS.md` — so the lock format
    changes once. Slice 2 is the forge's pull requests (Decisions 20–26) and the forge's last
    inch, after open questions 1 and 2 are answered on a real remote.
13. **A linear book is byte-identical.** Nothing new is printed where it would
    say only the default. The lock gains `line=` and `parents=` on branch steps
    and `parents=` and `merges=` on merges, the way `play=N` appears only when
    non-zero; `[<repo>.branches]` appears only for a repo with branches.
    `Bower-Line` and `Bower-Merges` trailers go on branch and merge commits
    only. `PULLS.md` exists only for a repo that declares a PR; compare links
    only on merges. So every existing lock, rendered chapter, and published SHA
    is unchanged by this EPIC. Appending a chapter moves the old last main step
    and its `-end` tag, because `STEPS.md` moves with it — true of any appended
    chapter, not of branches.
14. **One composition rule.** A touch is whole-file, `Region(name)`, or append.
    `composes(a, b)` is lifted out of `check_duplicate_files` (`step.rs:198`)
    and both callers use it: within a step, every pair must compose; at a
    merge, every (branch touch, main-since-fork touch) pair on one file must
    compose, unless the merge step writes that whole file. Two appends
    compose — main's lands first, then the branch's, as the re-application
    order says.
15. **A PR belongs to a branch, not a step.** The key form is `pr="Title"` on a
    branch step's own directive, with no body. The block form is a directive
    carrying `repo=`, `branch=`, and `pr=`, followed by a fenced markdown body,
    and it binds by its `branch=` name — not through `StepIndex::locate`, which
    would find a step and then have to check its line against the block's
    `branch=`. A block naming a branch no step is on is `PrUnknownBranch`.
    `from=` on any step but a branch's first is `FromOnLaterStep`: a key
    nothing reads is a typo. That includes `from=` on a main or merge step,
    not only a later branch step — nothing reads it there either, so it is
    `FromOnLaterStep` too. Twelve error variants, not ten.
16. **The worktree and `STEPS.md` come from the last main step.** Today
    `Replayer::run` and `final_blobs` take them from `plan.steps.last()`
    (`replay.rs:121`, `replay.rs:336`); a
    book ending on an unmerged branch step would otherwise give main a branch's
    tree and check the abandoned experiment out as HEAD. `STEPS.md` lists every
    step, since tags resolve on any line; a branch row's subject gains
    `· on <branch>`, a merge row's `· merges <branch>`.
17. **`status` compares branch names, not branch SHAs.** `repo_drift`
    (`status.rs:132`) compares tags by name, read from loose refs; branches
    follow the same rule. "Moved" would need the expected SHA, and that needs
    a replay, which `status` never does — so `status` reports a missing or an
    unexpected branch, and not a moved one.
18. **A pure merge renders a line of its own.** `footer` (`render.rs:316`) is
    per code block, so an `op="none"` merge would render nothing but its
    anchor. A merge step's directive therefore renders one `step-meta` line
    where it stands: the step, the branch it merges, the compare link, and the
    checkout command. Other `op="none"` steps are unchanged. The compare is
    `{parent0_tag}...{parent1_tag}` — three dots, so it shows what the branch
    did since the fork — and is omitted when parent 0 is the scaffolding.
19. **Open questions 5 and 6 are settled.** The v1 fences stay:
    `FromNotOnMain` and `MergeOnBranch`. The spike is deleted on the slice-1
    branch once its sixteen tests pass as ported into `bower-core`.

*Decisions 20–26 were added 14 September 2026, when slice 2 was designed
against the `pr-remote` spike's findings (open questions 1 and 2).*

20. **A merged PR is opened on a stepping stone (open question 7).** A book
    declares a branch, its PR, and its merge together, so on a first publish
    the merge is in main before a PR could be opened, and a forge will not
    open a PR with no commits ahead of its base. For each declared PR whose
    branch the book has merged and which has no PR on the forge yet, the push
    first moves main to the merge step's main parent — the commit just before
    the merge — opens the PR while the branch still has commits ahead, and
    then moves main on; the forge marks the PR merged once main passes the
    merge (open question 1). Stones go in merge order. Every declared PR thus
    appears on the forge. The cost: on an already-published repository main
    rewinds for a few seconds and CI runs once per stone. A merge whose main
    parent is the scaffolding commit has no step tag to stand on; its PR is
    reported `NotOpened`.
21. **The push is a schedule.** A pure function —
    `schedule(plan, remote heads, forge PRs)` in `bower/src/schedule.rs` —
    decides every move: push a branch, move main to a step, open a PR, edit a
    PR, push the tags. The dry run prints the schedule; `--execute` runs it
    through small `Forge` methods that replace `Forge::push`: `remote_heads`
    and `pull_requests` (read-only, after the gate), `push_ref`, `push_tags`,
    `open_pull_request`, `edit_pull_request`, and `set_default_branch`
    (Decision 26). Execution stops at the first
    failed move and names the moves already done — and if it stops while main
    stands on a stepping stone, it first moves main back to its head: a stone
    is an older commit with no `STEPS.md`, and a remote left there would fail
    the marker gate on every later push. On an existing remote the
    order is: branches; then, per stone, main to the stone and the PR opened;
    main to its head; the other PRs opened or edited; tags. The other PRs come
    after main's head because a PR can only be opened against a base that
    shares its history, and after a rebuild that moves every SHA the remote's
    old main shares none. On a remote with no main, main goes first — to the
    first stone, or its head — then the rest as before. Tags are always last (corrigendum item 4). Each push of
    main leases against the one before it: the remote's value first, then the
    stone Bower just pushed. GitHub closes an open PR when main alone is
    force-pushed to a history its head shares no commits with (open question
    2); an open PR survives a rebuild that moves every SHA when its branch and
    main move in one push (the spike's Q2b). So the branches and main's first
    move go out in one atomic push, which is what `execute` does (corrigendum
    item 13).
22. **What `ensure` does, per declared PR** (Decision 8, made exact). The
    forge's PRs are looked up by head branch with base `main`, in any state.
    None → `Created` (via a stone if the book merged the branch). Open →
    `Updated` if the title or body differs, else `Unchanged`. Merged →
    `LeftMerged`, noting when its head is no longer the branch's head — the
    drift open question 2 found, reported and never repaired. Closed by a
    person → `LeftClosed`. Bower never closes, merges, reopens, or deletes a
    PR, and has no flag to.
23. **A PR's body is the book's description, then a footer:** a line —
    "Opened by Bower from the book *<name>*. It is merged by a push to main,
    never on the forge." — and an HTML comment `<!-- bower-pr: <digest> -->`,
    the FNV digest (`publish::digest`) of the title and description. `Updated`
    versus `Unchanged` compares digests, read back with `gh pr list --jq`: the
    base binary parses no JSON (`serde_json` is the preprocessor's alone), and
    GitHub may rewrite a body's line endings, so comparing bodies would be
    fragile where comparing a digest is not. A PR whose body has no digest — one
    a person wrote, or edited away — reads as different and is `Updated`.
24. **Forge drift is reported by the push, not by `status`.** `status`
    touches no network (EPIC-04), so a merged PR whose head moved, or a PR a
    person closed, shows in the dry run's schedule instead.
25. **Not in slice 2:** closing a PR from the book (open question 3, until a
    chapter needs "declined"); `ForgejoForge` (no Forgejo forge exists yet —
    `DESIGN_Forges.md`); and *Rust for Failures*' `from-char` PR (its chapter
    is not listed yet). *Amended 14 September 2026:* that PR left the EPIC
    altogether — an EPIC is proven on `hello-playbook` only, so it never waits
    on the state of another book.
26. **A fresh remote's default branch is `main` (open question 8).** On a
    push to a remote with no `main`, main goes first (Decision 21), and the
    schedule then sets the repository's default branch to `main`
    (`Forge::set_default_branch`; GitHub: `PATCH /repos/{owner}/{repo}`
    with `default_branch=main`). It does not rely on the forge happening to
    make the first pushed branch its default: the marker gate reads
    `STEPS.md` from the default branch, and only main carries it. The move is
    scheduled only when the remote had no `main`, so a repository whose owner
    chose another default is never changed underneath them — the rule
    `enable_pages` already follows.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Which line a step is on | `Line { Main, Branch(String) }` on `PlannedStep` | 🔴 new |
| A step's parents | `PlannedStep::parents: Vec<usize>` (0 = scaffolding) | 🔴 new |
| Fork point | `BranchSummary::forked_from`, `from=` | 🔴 new |
| The merge tree | per-line fold in `plan()` | 🔴 new |
| A conflict | `BowerError::MergeConflict` + `composes`, lifted from `check_duplicate_files` `step.rs:198` | 🟡 rule exists, reuse |
| A pull request | `PullRequest` on `RepoPlan::branches` | 🔴 new |
| One parent per commit | `Replayer::commit` `replay.rs:170` | 🟡 becomes `Vec<ObjectId>` |
| The only branch | `BRANCH` `replay.rs:27` | 🟡 becomes the main line's ref |
| The final tree | `final_blobs` `replay.rs:330` | 🟡 last *main* step |
| Chapter-end anchors | `chapter_ends` `replay.rs:366` | 🟡 main steps only |
| What must be pushed | `expected_tags` `replay.rs:351` | 🟡 plus `expected_branches` |
| Is the build current? | `repo_drift` `status.rs:132` | 🟡 plus branch names |
| The footer | `render::footer` `render.rs:316` | 🟡 line, compare |
| A pure merge's line | beside `checkout_line` `render.rs:372` | 🔴 new |
| Link vocabulary | `LinkTemplates` `config.rs:94` | 🟡 plus `compare`, `branch` |
| Publishing | slice 1: `Forge::push`, `push_branch_args`; slice 2: `Forge::{push_ref, push_refs, push_tags}`, `push_ref_args`, `push_refs_args` (`forge.rs`) | 🟡 per ref, leased |
| What a push does, in order (slice 2) | `schedule::schedule`, `schedule::pr_action`, `push::execute` | 🔴 new |
| The forge's PRs (slice 2) | `Forge::{pull_requests, open_pull_request, edit_pull_request}` | 🔴 new |

---

## Design

### Kernel — `bower-core`

`Directive` (`directive.rs:188`) and `Block` (`block.rs:25`) gain `branch`,
`from`, `merge`, `pr`. `Step` (`step.rs:28`) carries them through `group`,
which also rejects a step whose blocks disagree on `branch` or `merge`
(`ConflictingLineInStep`, beside `ConflictingRepoInStep`). As with `expect`,
any block of a step may carry the value; two different values conflict.

`plan()` restructures the per-repo loop into a fold with state — the spike's
`plan()` is the reference implementation:

```rust
struct Fold {
    main: TreeState,
    main_head: usize,                        // seq; 0 = scaffolding
    main_touches: Vec<(usize, Touch)>,       // (seq, file+region) main changed
    branches: BTreeMap<String, BranchState>, // one tree per open line
}
struct BranchState {
    forked_from: usize, head: usize, tree: TreeState,
    blocks: Vec<Block>,                      // replayed over main at merge
    touched: BTreeSet<Touch>, merged_at: Option<usize>, pr: Option<Pr>,
}
```

The fold runs over the raw `TreeState` (region markers kept), and each step's
tree is materialized (`tree.rs:55`) and its display ranges resolved exactly as
today. Re-applying the branch's blocks at a merge goes through the same
`apply_block` (`tree.rs:72`); a failure there — a region main removed, an
append to a file main deleted — is reported at the merge step.

`PlannedStep` gains `line`, `parents`, `merges`; `RepoPlan` gains
`branches: Vec<BranchSummary>`. `lock_text` (`plan.rs:446`) prints
`line=` and `parents=` on branch steps, `parents=` and `merges=` on merges,
nothing new on any other step, and a `[repo.branches]` table only when the
repo has a branch (Decision 13):

```
002 lookup-table expect=test_fail anchor=… files=src/rank.rs line=try/lookup-table parents=001
006 merge-from-char expect=pass anchor=… files= parents=003,005 merges=from-char

[failers.branches]
try/lookup-table from=001 head=002 merged=no pr="Try a lookup table for ranks" state=open
from-char from=003 head=005 merged=006 pr="Rank::from(char)" state=merged
```

New `BowerError` variants (`lib.rs:67`), all with a `Location`:
`InvalidBranchName`, `MergeUnknownBranch`, `MergeOnBranch`,
`BranchAlreadyMerged`, `UnknownFrom`, `FromNotOnMain`, `FromOnLaterStep`,
`MergeConflict`, `PrWithoutBranch`, `PrUnknownBranch`, `PrDuplicate`,
`ConflictingLineInStep`.

The `pr=` block form is partitioned out in `plan()` beside play, output, and
exercise blocks (`plan.rs:147-151`) and binds by its `branch=` name; the key
form rides on a branch step's own directive (Decision 15).

### Testkit — `bower-testkit`

Fixtures: the spike's `rank_saga` as a real chapter (it is
`docs/spikes/spike-branches/fixture-book.md`), plus one fixture per error variant. The
coverage report (`coverage.rs`) gains a `line` axis — `(op × expect × line
∈ {main, branch, merge})` — so "a `compile_fail` on a branch" and "a
`test_fail` on a merge" are states the corpus is known to visit.

Properties: the merge tree equals the branch head tree whenever main touched
nothing since the fork; every `parents` entry is smaller than its step's
`seq`; a branch ref's step is the largest `seq` on that line.

### Replay — `bower`

`Replayer::commit` takes `parents: &[ObjectId]` and the ref to advance;
`run` keeps `sha_of: BTreeMap<usize, ObjectId>` and resolves each step's
parents from it; parent 0 is the scaffolding commit, or no parent when there
is none. Branch steps advance `refs/heads/<name>`; main steps and merges
advance `BRANCH`. `trailers::commit_message` (`trailers.rs:19`) adds
`Bower-Line: <line>` on branch commits and `Bower-Merges: <branch>` on merges
— nothing on a main step (Decision 13). `final_blobs` (`replay.rs:330`) and the
worktree take the last *main* step (Decision 16); `STEPS.md` marks branch and
merge rows; `trailers::pulls_md` writes `PULLS.md` into the same final tree
when the repo declares a PR. `chapter_ends` anchors on main steps only.
`expected_branches(plan)` sits beside `expected_tags`, and `repo_drift`
compares both by name (Decision 17).

### Render — `bower`

`footer` (`render.rs:316`) appends `· on [try/lookup-table](…)` for branch
steps. Every merge step — pure or carrying resolution code — renders one
`step-meta` line at its anchor, `step 006 of failers · merges [from-char](…) ·
[compare](…)`, followed by its checkout line; the merge is said once, there,
and never again in a footer (Decision 18). `LinkTemplates` gains
`compare` (`{base}/compare/{from}...{to}` — the shape `DESIGN_Forges.md`
§ 6.1 proposes, filled with the merge's two parents' tags) and `branch`
(`{base}/tree/{branch}`), both derived from `github` like `tree` and `commit`
and both declarable in `[links]`. The reader's `checkout` line is unchanged:
tags resolve on any line.

### Push — `bower`

**Slice 1.** `Forge::push` takes the plan's branches: it pushes each with its
own lease via `remote_head` and `push_branch_args` — already parameterised by
branch — then the tags, unchanged, then main with its lease. `FakeForge`
records the order. `plan_push` (`push.rs:162`) lists every branch in the dry
run. A remote branch the plan does not name is left alone; Bower never
deletes. The marker gate is not touched — a repo that fails it gets no
branches either.

**Slice 2** (Decisions 20–24). `bower/src/schedule.rs` is pure:
`ForgePr { number, branch, state, title, body, head }`;
`PrAction { Created, Updated, Unchanged, LeftMerged { moved }, LeftClosed,
NotOpened }`; `pr_action(&PullRequest, &[ForgePr])`; and
`schedule(plan, heads, prs) -> Schedule { moves, prs }` with
`Move { Branch, Main { at }, DefaultMain, OpenPr, EditPr, Tags }`, `at` being a step tag.
`plan_push` passes the gate, then reads `Forge::remote_heads` and
`Forge::pull_requests`, and carries the schedule in `PushPlan::Ready`; the dry
run prints it as a numbered `schedule` block, and a book without branches
prints nothing new. `push::execute` runs the moves through `push_ref`,
`push_tags`, `set_default_branch`, `open_pull_request`, and
`edit_pull_request`, chaining main's
leases, and stops at the first failure naming what went out. `FakeForge`
records every call. `GitHubForge`: `git ls-remote --heads`, `gh pr list
--base main --state all --json … --jq` emitting one tab-separated line per PR
(number, state, head branch, head SHA, digest — parsed by a pure, tested
function),
`gh pr create`, `gh pr edit`, `gh api -X PATCH` for the default branch. A repo that fails the gate gets no read and no
PR.

---

## Work Items

### Phase 0 — Kernel

Every item is slice 1 unless marked **(slice 2)**.

- [x] **0a.** Directive keys, `Block` and `Step` fields, `ConflictingLineInStep`,
  `InvalidBranchName` (pure ref-name check).
- [x] **0b.** The fold: `Line`, `parents`, per-line trees, merge tree,
  `BranchSummary`, `FromOnLaterStep`. Port the spike's `plan()` and every
  spike test by name.
- [x] **0c.** `MergeConflict` at region granularity: lift `composes` out of
  `check_duplicate_files` and call it from both (Decision 14).
- [x] **0d.** `pr=` key and block forms, `PullRequest`, `PrWithoutBranch`,
  `PrUnknownBranch`, `PrDuplicate`.
- [x] **0e.** `lock_text` line/parents/branches, printed only where not the
  default; exercises follow the line.
- [x] **0f.** Testkit fixtures, the `line` coverage axis, the three
  properties. Delete `docs/spikes/spike-branches/` (Decision 19).

### Phase 1 — Replay and status

- [x] **1a.** `commit` with `Vec` parents and a ref; branch refs; trailers on
  branch and merge commits only.
- [x] **1b.** The worktree and `STEPS.md` from the last main step; `STEPS.md`
  branch and merge rows; `PULLS.md`; `chapter_ends` on main;
  `expected_branches`.
- [x] **1c.** Determinism golden (`bower/tests/determinism.rs`) over the branch
  fixture: identical SHAs across runs, and the blast-radius assertion —
  editing a branch step leaves every unrelated tag's SHA alone. A linear
  book's SHAs are unchanged by this EPIC.
- [x] **1d.** `repo_drift` compares branch names; `status` names a missing or
  unexpected branch.

### Phase 2 — Render

- [x] **2a.** `compare` and `branch` templates, derived from `github` and
  declarable in `[links]`.
- [x] **2b.** Footer line and compare link; the merge line for a pure merge;
  preprocessor test.

### Phase 3 — Push and pull requests

- [x] **3a.** `Forge::push` takes the branches: each with its own lease, then
  tags, then main. `FakeForge` records the order; the dry run lists branches.
- [x] **3b.** Golden: a refused gate pushes no branch.
- [x] **3c. (slice 2)** `bower/src/schedule.rs`: `ForgePr`, `PrAction`,
  `pr_action`, `schedule`, stepping stones — pure (Decisions 20–22).
- [x] **3d. (slice 2)** `Forge` methods replace `push`, including
  `set_default_branch` (Decision 26); `FakeForge` records each; `GitHubForge` last inches with pure `gh` JSON parsing; the PR body
  footer (Decision 23); `push::execute`.
- [x] **3e. (slice 2)** `plan_push` reads heads and PRs after the gate;
  `PushPlan::Ready` carries the schedule; the dry-run `schedule` block; the
  no-override test extended (`--merge-pr`, `--close-pr` absent).
- [x] **3f. (slice 2)** Goldens: a refused gate reads no PR and creates none;
  a book without branches pushes main, then tags, as before.
- [x] **3g. (slice 2)** Live: `make ship-hello-execute` opens `try/shout`'s
  PR, opens `feat/greet-many`'s on a stepping stone and sees it merged; a
  second push reports `Unchanged` and `LeftMerged` and opens nothing.

### Phase 4 — Book and docs

- [x] **4a.** `books/hello-playbook/src/ch07-try-it-on-a-branch.md`, before
  the appendix: `try/shout` (one `test_fail` step and a `pr=` block, never
  merged) and `feat/greet-many` (two steps, merged by an `op="none"` step
  after one unrelated main step).
- **4b. (slice 2)** ~~The `from-char` PR in
  `books/rust4failures/src/ch03-rank.md`, once that chapter is listed.~~
  *Withdrawn 14 September 2026.* No EPIC depends on *Rust for Failures*; 4a's
  two branches and two PRs are the EPIC's proof, and the book adopts branches
  as its own work (`BACKLOG.md`).
- [x] **4c.** `.okf/model/branch.md`, `.okf/model/pull-request.md`, the key
  table in `.okf/model/directive.md`, `BACKLOG.md`, `README.md`.
- [x] **4d.** Flip slice 1's Status rows, append the corrigendum.

---

## Test Plan

Carried from the spike, one per name:
`plan__branch_forks_from_nearest_preceding_main_step`,
`plan__explicit_from_forks_earlier`,
`plan__merge_has_two_parents_and_is_on_main`,
`plan__merge_tree_is_branch_blocks_over_main`,
`plan__unmerged_branch_survives_with_an_open_pr`,
`plan__conflict_without_resolution_is_an_error`,
`plan__resolution_block_clears_the_conflict`,
`plan__a_main_edit_before_the_fork_is_not_a_conflict`,
`plan__step_after_merge_is_refused`, `plan__merge_of_unknown_branch_is_refused`,
`plan__merging_twice_is_refused`, `plan__from_must_name_a_main_step`,
`plan__lock_text_shows_lines_parents_and_branches`,
`replay__is_byte_identical_across_runs`,
`replay__merge_commit_has_two_parents_and_branches_are_refs`,
`replay__a_change_on_the_branch_changes_only_what_descends_from_it`.

New here: `plan__conflict_is_per_region_not_per_file`,
`plan__two_appends_compose_at_a_merge`,
`plan__pr_on_a_main_step_is_refused`, `plan__two_prs_on_one_branch_is_refused`,
`plan__pr_block_for_an_unknown_branch_is_refused`,
`plan__from_on_a_later_step_is_refused`,
`plan__branch_named_main_is_refused`, `plan__exercise_on_a_merged_head_answers_with_the_merge`,
`plan__lock_text_of_a_linear_book_is_unchanged`,
`replay__chapter_end_tag_never_points_at_a_branch`,
`replay__worktree_is_main_when_the_book_ends_on_a_branch`,
`replay__a_linear_book_has_one_parent_and_no_line_trailers`,
`status__reports_a_missing_branch_ref`,
`render__a_pure_merge_renders_its_line`,
`push__branches_go_before_main`,
`push__gate_refusal_pushes_no_branch`.

Slice 2: `schedule__a_straight_line_is_main_then_tags`,
`schedule__an_existing_remote_pushes_branches_then_prs_then_main_then_tags`,
`schedule__a_fresh_remote_puts_main_first_and_makes_it_the_default`,
`schedule__an_existing_remote_never_changes_its_default`,
`schedule__a_merged_pr_without_a_forge_pr_gets_a_stepping_stone`,
`schedule__stones_go_in_merge_order`,
`schedule__a_forge_pr_that_exists_needs_no_stone`,
`pr_action__an_open_pr_is_updated_only_when_it_differs`,
`pr_action__a_merged_pr_is_left_alone_and_notes_a_moved_head`,
`pr_action__a_closed_pr_is_left_closed`,
`pr_action__a_stone_on_the_scaffolding_is_not_opened`,
`execute__runs_moves_in_order_and_chains_main_leases`,
`execute__stops_at_the_first_failure_and_names_what_went_out`,
`push__gate_refusal_reads_and_creates_no_pr`,
`push__has_no_merge_or_close_flag`.

## Key Files

| File | Role |
|---|---|
| `docs/spikes/spike-branches/` | the reference model; deleted in `d99ccf7`, readable in the git history at `8f3057f` |
| `bower-core/src/directive.rs` | four keys |
| `bower-core/src/block.rs`, `step.rs` | fields; `ConflictingLineInStep`; `composes` |
| `bower-core/src/plan.rs` | the fold, `Line`, `parents`, `BranchSummary`, `PullRequest`, lock text |
| `bower-core/src/lib.rs` | twelve error variants |
| `bower-testkit/src/{fixtures,coverage}.rs` | textures, the `line` axis |
| `bower/src/replay.rs` | parents, branch refs, `chapter_ends`, `expected_branches` |
| `bower/src/trailers.rs` | `Bower-Line`, `Bower-Merges`, `pulls_md` |
| `bower/src/status.rs` | branch drift |
| `bower/src/render.rs`, `config.rs` | footer, `compare`/`branch` templates |
| `bower/src/forge.rs`, `push.rs` | the `Forge` methods, `execute`, the dry run |
| `bower/src/schedule.rs` (slice 2) | `schedule`, `pr_action`, stepping stones — pure |

## Reuse (do NOT recreate)

- `step.rs:198` `check_duplicate_files` — the region composition rule *is*
  the conflict rule; lift it into `composes`, do not write a second one.
- `plan.rs:126` `PlannedStep::tag` — the only tag format.
- `tree.rs:72` `apply_block` — the merge re-applies blocks through it; there
  is no second apply.
- `replay.rs:351` `expected_tags` — `expected_branches` sits beside it and
  `status` reads both from replay, never re-derives.
- `forge.rs` `push_ref_args` / `push_refs_args` (slice 2's successors to
  `push_branch_args`) — one lease per ref, one argument shape for one ref or
  an atomic batch.
- `push.rs` marker gate — untouched; branches and PRs sit behind it.
- `DESIGN_Forges.md` § 6.1 `{prev_tag}` — the compare link is the same one.

## Compatibility

- **Preserves** every existing book byte for byte: no `branch=` means one
  line and one parent each, and nothing new is printed for either — not in
  the lock, not in a trailer, not in a rendered chapter, so no published SHA
  moves (Decision 13).
- **Adds** four keys, twelve errors, two templates, one generated file
  (`PULLS.md`), and — in slice 2 — seven `Forge` methods in place of
  `Forge::push`.
- **Breaks** nothing in `bower-core`'s purity: `cargo tree -p bower-core -e
  normal` still prints one line.

## Dependencies

- **Built on:** EPIC-01 (replay, tags), EPIC-04 (`repo_drift`), EPIC-05
  (`Forge`, the gate), exercises design (binding, answers).
- **Coordinates with:** `DESIGN_Forges.md` (per-forge template derivation,
  `ForgejoForge` last inch) — land Phase 2's templates in whichever ships
  first.
- **Unblocks:** *Rust for Failures* chapters that need an abandoned attempt;
  the *Controllability* "change as a PR" chapters.

## Verification

```bash
make ayce                     # clean, fmt, build, test, lint, security-scan, docs
make slow                     # the #[ignore]d lanes
make book                     # the sample book through the preprocessor
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hp
git -C /tmp/hp log --graph --oneline --all --decorate --date-order
cargo run -p bower -- --book books/hello-playbook verify
cargo run -p bower -- --book books/hello-playbook push -o /tmp/hp   # dry run
```

Exit criteria (slice 1 unless marked):

1. The sample book's graph shows one unmerged branch and one two-parent merge;
   `git checkout try/shout && cargo test` fails as the book claims.
2. Two builds of the same book produce identical SHAs for every ref, branches
   included.
3. Editing a branch step changes the SHAs of that step, its successors on the
   line, and the merge — and nothing else.
4. A merge with a conflicting region and no resolution is a plan-time error
   naming the step, the branch, the file, and the region.
5. `verify` reports the same verdicts with branches as without; a merge step
   is verified on its own tree.
6. A dry-run `push` lists every branch and the order; a gate refusal lists
   none of them and `FakeForge` records no call. **(Slice 2:** the numbered
   schedule, with every PR and its action.**)**
7. `cargo tree -p bower-core -e normal` still prints one line.
8. `hello-playbook`'s steps 001–019 keep their SHAs and its existing lock
   lines are unchanged; rendered output changes only on branch and merge steps
   (Decision 13). mdBook's sidebar gains the new chapter on every page, which
   is the chapter, not the EPIC.
9. **(Slice 2)** `bower push --execute` against `abstecker/hello-playbook`
   leaves `try/shout`'s PR open and `feat/greet-many`'s merged, opened on a
   stepping stone; a second push reports `Unchanged` and `LeftMerged` and
   opens no PR.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | ~~**Merged-PR detection.**~~ **Settled for GitHub, 14 September 2026** (`docs/spikes/pr-remote/`, two runs against `abstecker/bower-sandbox`): a plain fast-forward push of main to Bower's merge commit turns the open PR `MERGED` within about ten seconds, and GitHub records Bower's own commit as the PR's `mergeCommit`. Decision 8 holds on GitHub with no forge-side merge. **Forgejo is still open** ("manually merged" may need a repository setting); verify it with a Forgejo container before `ForgejoForge` ships — carried as `DESIGN_Forges.md` § 11 question 5. |
| 2 | ~~**PR churn on regeneration.**~~ **Settled for GitHub, 14 September 2026** (same spike). An **open** PR follows its branch through any force-push, even when every SHA changes: its head moves to the new commit and it lists the new commits against the new main — no churn, no duplicate. A **merged** PR is frozen: after its branch and main are force-pushed it stays `MERGED` with its old head and old merge commit, which are then no longer on main. `gh pr list --head <branch> --state all` still finds it, so the push sees it and leaves it alone (`PrAction::LeftMerged`) instead of opening a duplicate. Decision 8 stands: live with the drift, and let the push's dry run report a merged PR whose head is no longer on main (Decision 24). **And one hazard, found in the second run:** GitHub **closes** an open PR by itself when main is force-pushed to a history its head shares no commits with — PR #1 was closed in the same second as the base force-push, credited to the pusher. An open PR survives a rebuild when its branch and main are updated in one push — the spike's Q2b pushed both in a single `git push` — so `execute` sends the branches and main's first move as one atomic push (Decision 21, corrigendum item 13 of slice 2); a force-push of main alone to unrelated history closes it. |
| 3 | **Closing without merging.** `pr_state="closed"` for a rejected PR — "reviewed, declined" is a *Failures* story. Cheap to add once Decision 8 stands. Deferred from slice 2 on 14 September 2026 (Decision 25): an abandoned branch's PR stays open until a chapter needs "declined". |
| 4 | **Review comments as book content.** A PR conversation is pedagogy. It is also a second body of prose the book would have to own; not before a real chapter asks for it. |
| 5 | ~~**Branches from branches, merges into branches.**~~ Settled 13 September 2026: `FromNotOnMain` and `MergeOnBranch` are the v1 fences (Decision 19). Lift when a chapter needs it; the fold generalises (a `BranchState` for main is the only change). |
| 6 | ~~**The spike's home.**~~ Settled 13 September 2026: its `rank_saga` becomes the `bower-testkit` fixture `branch_saga` (the testkit already has a `rank_saga`), and the spike is deleted once its tests pass as ported (Decision 19). |
| 7 | ~~**A merged branch's PR on a first publish.**~~ **Settled 14 September 2026: stepping stones** (Decision 20). For each declared PR the book has already merged and the forge has no PR for, main is moved to the commit just before the merge, the PR is opened, and main moves on; the forge marks it merged as main passes the merge. The alternative — record it and open nothing — was rejected: a book written all at once would never show a merged PR on the forge. |
| 8 | ~~**The first branch pushed to an empty repository.**~~ **Settled 14 September 2026: the default branch is `main`, by rule** (Decision 26). Bower pushes main first to a remote with no `main` and then sets the default branch to `main` itself, so what a forge does with the first pushed branch no longer matters and needs no test. |

---

## Corrigendum — as built, slice 1

Deviations from this plan the tasks reported during execution, with where and
why. The first four change behaviour from the plan's text; items 5 through 8
are mechanical; item 9 clarifies the scope of an existing exit criterion
rather than changing anything.

1. **A merge's resolution skips only the resolved paths of a branch block,
   not the whole block.** The plan skipped a whole branch block when any file
   it touched was resolved; a multi-path `op="delete"` then lost its
   unresolved paths silently — a wrong merge tree with no error. `Fold::merge`
   now re-applies a trimmed clone (`trim_resolved`, `bower-core/src/branch.rs`),
   extending Decision 3. Test:
   `plan__a_resolution_does_not_drop_the_rest_of_a_branch_block`.
2. **`from=` on a main or merge step is refused** with `FromOnLaterStep` (its
   `branch` field reads `main`, its message "…this step is on `main`"). The
   plan text refused it only on a later *branch* step; Decision 15 already
   said any step but a branch's first, and the corrigendum makes that
   explicit. Tests: `plan__from_on_a_main_step_is_refused`,
   `plan__from_on_a_merge_step_is_refused`.
3. **Branch names that collide as git refs are refused** as
   `InvalidBranchName`: `main`/`HEAD` in any letter case, anything under
   `main/`, a name that is a `/`-prefix of another in the repo, and two names
   equal only in case. Found in review: `main/x` alongside `a` and `a/b` made
   replay fail with an unlocated IO error, and `Main` on a case-insensitive
   filesystem silently repointed main. `branch_name_problem` and a repo-level
   `check_branch_refs` (`bower-core/src/plan.rs`), extending Decision 6.
4. **Tags always push last.** On an existing remote, each branch pushes with
   its own lease, then main with its own lease, then the tags, forced —
   never tags before main, because a lease trip on main must not leave tags
   already force-rewritten, and tags have no bearing on PR merge detection.
   On a remote with no `main`, main is pushed first, then the branches, then
   the tags — the first branch pushed to an empty GitHub or Forgejo
   repository likely becomes its default, and the push gate reads
   `STEPS.md` from the default branch, so a side branch first would lock the
   book out of every later push. A straight-line book then pushes main, then
   tags, on either remote — exactly the pre-branch order. `push_commands`
   (`bower/src/forge.rs`), extending Decision 9.
5. `cargo fmt --all` was not in the plan's per-task gate; it was added after
   Task 10's review found `make fmt-check` (which CI runs) failing on this
   branch. Task 10's commit carries formatting-only hunks in files from
   earlier tasks.
6. Clippy-driven restructurings along the way: `#[allow]`s on `Block`
   (`struct_excessive_bools`), `block::resolve` (`too_many_lines`), and
   `branch_name_problem` (`case_sensitive_file_extension_comparisons`);
   `render::push_merge_line` extracted from `chapter`; `PushPlan::Ready::release`
   boxed (`large_enum_variant`); `describe_release` extracted from
   `report_push`.
7. `bower/tests/preprocessor.rs` also pinned the sample book's chapter list
   and was updated in Task 11 to add `ch07-try-it-on-a-branch.md`.
8. The Task 3 commit was first recorded with an unrelated message and
   amended by the user (`b6b9688`).
9. **The blast-radius promise (exit criterion 3) holds for a branch step's
   *content*.** Editing any branch step's `msg` or id also moves main's last
   commit, because `STEPS.md` on main lists every step, branch steps
   included (Decision 16) — so a branch-only edit still touches one main SHA
   even though nothing on main's own tree changed.

None of the nine change the shape of what shipped; each is recorded here
because a task reported it as a deviation from this document's text at the
moment it happened.

---

## Corrigendum — as built, slice 2

Deviations from this plan the tasks reported during execution, with where and
why.

1. **`ExecuteError` gains a `stranded` field.** The plan had the put-back
   push — moving main off a stepping stone after a later move failed — drop
   its own failure silently through `.is_ok()`. A review found that a remote
   left on a stepping stone would then fail the marker gate on every later
   `bower push`, with no word to the user. `execute` (`bower/src/push.rs`)
   now reports a failed put-back with the stone, the head it could not
   restore, and the forge's error, and `Display` adds "push `<head>` to main
   by hand before the next bower push". The tuple is boxed
   (`Option<Box<(String, String, ForgeError)>>`) so `ExecuteError` — and
   `execute`'s `Result::Err` — stays under clippy's `result_large_err`
   threshold.
2. **`FakeForge::unreachable()` rebuilt on `new()`.** The new fields
   (`heads`, `prs`, `fail_on`, `next_pr`) have to be initialized at every
   struct-literal site that builds a `FakeForge`, and `unreachable()` is a
   third such site beside `new` and `with_site`. Rather than repeat the four
   initializers, `unreachable()` now builds on
   `Self::new(RemoteState::HasContent, None)` and overrides `site_state` and
   `unreachable`, keeping `new` the single source of truth for the new
   fields' defaults (`bower/src/forge.rs`).
3. **`PushPlan::Ready::schedule` is boxed.** Clippy's `large_enum_variant`
   fired once `Schedule` sat unboxed beside `Blocked`'s much smaller payload;
   `schedule: Box<Schedule>` closes it with no change to any caller (`&schedule`
   still derefs to `&Schedule`) (`bower/src/push.rs`).
4. **`bower::push::execute` is called by its fully qualified path in
   `main.rs`, not imported bare.** `report_push`'s own parameter is named
   `execute: bool`; importing a bare `execute` function into the same
   function would shadow it and fail to compile. Behaviour is identical
   (`bower/src/main.rs`).
5. **`bower/tests/push.rs` gained `#![allow(non_snake_case)]`** on its
   existing allow line, needed for the new test
   `push__has_no_merge_or_close_flag`'s double-underscore name — this file
   has no enclosing `#[allow(non_snake_case)]` module the way `push.rs`'s own
   `plan_tests` does.
6. **One test assertion dropped a debug message.**
   `plan__our_own_remote_is_ready_without_creation` (`bower/src/push.rs`)
   changed `assert!(!create, "{got:?}");` to `assert!(!create);` — the literal
   port failed to borrow-check, because `Schedule` is not `Copy` and `got` had
   already been partially moved by the destructure. No coverage was lost; the
   panic message on the `else` arm immediately above it already covers the
   failure case.
7. **The dry run's `order` line for branched books is gone.** The numbered
   `schedule` block (`push::schedule_lines`) replaces it.
8. **`--execute` prints one `pushed    <move>` line per move, for every
   book** — not `pushed    N commits, M tags` as before — because Decision 13
   (a straight-line book's report is unchanged) binds only the dry run, not
   `--execute`'s own report.
9. **`push__branches_go_before_main` tested `push_commands`, which slice 2
   removed**; its guarantee — branches no later than main — is now
   `schedule__an_existing_remote_pushes_branches_then_prs_then_main_then_tags`,
   and `execute__runs_moves_in_order_and_chains_main_leases` pins that they go
   out together (item 13).

*Items 10–14 were added by the final whole-branch review's fix wave,
14 September 2026. Item 10 records shapes the Design section still shows as
planned; items 11–14 are changes the review ruled.*

10. **The as-built shapes differ from § Design's Push section and Decision
    21**, which are left as written:
    - `ForgePr { number, branch, state, head, digest }` — no `title` or
      `body`. A decision reads only the body's `bower-pr:` digest (Decision
      23), and the base binary reads no JSON to get a body back with.
    - `pr_action(pr, on_forge, local_head)` — three arguments, not two: the
      built repository's branch head decides `LeftMerged { moved }`.
    - `schedule(plan, remote_heads, local_heads, on_forge, book)` — not
      `schedule(plan, heads, prs)`: local heads for `moved`, and the book's
      name for the PR body's footer.
    - `Schedule { moves, prs, head }` — `head`, main's last step tag, is
      where `execute` puts main back when it stops on a stone; and
      `Move::Main { at, stone_for }` names the branch a stone is for.
11. **The sample book declares a PR for `feat/greet-many`** (review:
    Critical). Only `try/shout` had one, so the live run (exit criterion 9)
    could never open a PR on a stepping stone. `ch07-try-it-on-a-branch.md`
    now carries a block-form `pr="Greet many names at once"` between
    `greet-many-test` and `changelog`, and the lock reads `feat/greet-many
    from=020 head=023 merged=025 pr="Greet many names at once" state=merged`.
    No step number moved; steps 001–024 keep their SHAs, and only
    `step-025-merge-greet-many`, `ch07-try-it-on-a-branch-end`, and main move,
    because `PULLS.md` in main's last tree gains a row (exit criterion 8
    holds). The stone is `step-024-changelog`. Test:
    `plans_the_branches_the_chapter_declares`
    (`bower-testkit/tests/sample_book.rs`).
12. **PRs from forks are never Bower's** (review: Important). `gh pr list
    --base main` lists fork PRs too, and matching by head branch name alone
    would have let Bower edit a reader's fork PR (no digest, so `Updated`) or
    be blocked for good by a closed one (`LeftClosed`). The exact
    `PR_LIST_JQ` is now
    `.[] | select(.isCrossRepository | not) | [(.number|tostring), .state, .headRefName, .headRefOid, ((.body // "") | (capture("bower-pr: (?<d>[0-9a-f]+)").d // "-"))] | join("\t")`,
    and `--json` asks for `number,state,headRefName,headRefOid,body,isCrossRepository`
    (`bower/src/forge.rs`). Test: `pr_list_args__ask_for_every_state_against_main`.
13. **The branches and main's first move go out in one atomic push**
    (review: Important). `execute` pushed each branch, then main, separately;
    after a rebuild that changes every SHA there was a window where a PR's
    head was new history and its base still old — the case the spike never
    tested. The spike's Q2b showed an open PR survives when branch and main
    move in one push. `Forge` gains an eighth method, `push_refs(dir, repo,
    &[RefPush])` — one `git push --atomic`, each ref with its own lease — so
    § Compatibility's "seven `Forge` methods" is eight. `execute` sends the
    maximal run of consecutive branch moves, plus the main move right after
    them if there is one, as one `push_refs`; if it fails, nothing in the
    batch went out, and main is where it was. `push_ref` stays for main's
    later moves and the put-back, and `push_ref_args` is now the non-atomic,
    one-ref case of `push_refs_args`, its output unchanged. The dry run adds
    `            (moves 1–<k> go out as one atomic push)` after the numbered
    moves when the schedule opens with such a batch of two or more moves; a
    straight-line book has no branch, so its dry run is unchanged (Decision
    13). Tests: `push_refs_args__one_atomic_push_with_a_lease_per_leased_ref`,
    `fake_forge__records_an_atomic_push_as_one_call`,
    `execute__leases_each_branch_against_its_remote_head`,
    `execute__a_fresh_remote_pushes_main_first_then_makes_it_the_default`,
    `schedule_lines__say_which_moves_go_out_as_one_atomic_push`, and the
    three earlier `execute__…` tests in the batched form.
14. **"main may be left on the stepping stone"** (review: Minor). After a
    lease trip, main may be somewhere other than the stone, so
    `ExecuteError`'s `Display` no longer says main *was* left there.

*Items 15–17 came from a review of all EPIC-09 work after slice 2 merged,
14 September 2026, and shipped in PR #8, each with a test written first.*

15. **A branch off the last main step keeps `STEPS.md` and `PULLS.md`.** Only
    the last main commit's tree held them, so a branch forking from it showed
    both deleted — in its first commit, its `[diff]` link, and its PR. A step
    now carries them when it is the last main step or descends from it
    (`bower/src/replay.rs`); every existing SHA is unchanged. Test:
    `replay__a_branch_off_the_last_main_step_changes_only_its_own_files`.
16. **Regions may not nest** (`BowerError::RegionNested`). `composes` treats
    distinct region names as disjoint, but a region op replaces everything
    between its markers: a branch rewriting `outer` over main's edit to an
    `inner` inside it dropped that edit with no `MergeConflict`. Any text op
    that leaves one region inside another is now a plan-time error
    (`bower-core/src/tree.rs`; `bower-spec.md` § 3.3, § 6.1). Tests:
    `plan__a_region_inside_another_is_refused_before_a_merge_can_drop_it`, the
    `RegionNested` broken fixture, and two `region__` unit tests.
17. **`push` blocks when `site_branch` is `main` or a book branch.** The site
    is force-pushed after the repository, so it would replace that branch on
    every push, and a PR for it would have the site as its head. Decided
    before any forge call (`push::site_branch_clash`). Tests:
    `plan__a_book_branch_named_like_the_site_branch_blocks`,
    `plan__a_site_branch_named_main_blocks`. *Widened 15 September 2026* (PR
    #10): now `push::site_branch_problem`, which also runs `site_branch`
    through `branch_name_problem`, so a name git would read as an option
    (`--mirror`) is refused too.

---

*Spike: `docs/spikes/spike-branches/` — 16 tests green on rustc 1.75, three hand-mutations
each caught, `cargo run` prints the proposed lock and the graph. Drafted 9
September 2026 against `ImperialBower/bower` @ HEAD ("docs: file the forge
design and add it to the backlog"). Deleted in `d99ccf7` once slice 1
shipped; readable in the git history at `8f3057f`.*
