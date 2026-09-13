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
- **Status:** Planned; model settled by `docs/spikes/spike-branches/`, filed
  10 September 2026. Slice 1 designed 13 September 2026 (Decisions 12–19,
  re-grounded after EPIC-11); no phase started.

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

The spike at `docs/spikes/spike-branches/` (zero dependencies, rustc 1.75, 16 tests)
settles the model: a branch is a line of history the fold keeps a tree for;
a merge is a main step whose tree is *the branch's blocks re-applied over
main*; conflicts are plan-time errors; every step's parents are plan values,
so SHAs stay byte-identical across runs and a change on a branch reaches only
what descends from it. `docs/spikes/spike-branches/fixture-book.md` shows the same six
steps as chapter directives.

**This EPIC does not** implement branches off branches, merges into
branches, rebases, or review comments as book content; does not merge on the
forge (Bower creates every merge commit, deterministically); does not make
pull requests byte-reproducible — they are forge artifacts, and § Decisions 8
says exactly how far Bower's contract with them goes.

---

## Status

| Component | Slice | Status |
|---|---|---|
| Kernel: four directive keys, `Line`, per-line fold, merge semantics | 1 | **Planned** |
| Kernel: `MergeConflict` at region granularity | 1 | **Planned** |
| Kernel: `PullRequest` on `RepoPlan.branches`, lock text | 1 | **Planned** |
| Testkit: branch textures and the `line` coverage axis | 1 | **Planned** |
| Replay: parents from the plan, branch refs, `PULLS.md`, trailers | 1 | **Planned** |
| `status`: branch-ref drift | 1 | **Planned** |
| Render: line and compare links in the footer; the merge line | 1 | **Planned** |
| `push`: branch refs with lease, branches before main | 1 | **Planned** |
| Sample book chapter and slow-lane verify | 1 | **Planned** |
| `push`: `Forge::ensure_pull_request`, the PR section of the dry run | 2 | **Planned** |
| *Rust for Failures* `from-char` PR | 2 | **Planned** |

Slice 1 is everything local and deterministic: a reader can check out the
abandoned branch, and `bower push` publishes it. Slice 2 is the pull request
on the forge, which waits on open questions 1 and 2 — both need a real remote
(Decision 12).

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
and forge-side merging. Each is a backlog line when this ships.

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
   `InvalidBranchName` carries the location.
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
9. **Push order is branches, tags, PRs, then main.** A PR needs its head to
   exist and needs a diff against base, so branches go first and main —
   which carries the merge commit — goes last. Both GitHub and Forgejo mark a
   PR merged when its head becomes reachable from base by a push; this is
   the last inch and open question 1.
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
    changes once. Slice 2 is `Forge::ensure_pull_request` and the forge's last
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
    nothing reads is a typo. Twelve error variants, not ten.
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
| Publishing | `Forge::push` `forge.rs:134`, `push_branch_args` `forge.rs:953` | 🟡 per branch |
| Ensuring a PR (slice 2) | `Forge::ensure_pull_request` | 🔴 new |

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
steps and `· merges from-char · [compare](…)` for merges that carry code. A
merge step's directive renders one `step-meta` line where it stands, so a pure
`op="none"` merge is visible too (Decision 18). `LinkTemplates` gains
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

**Slice 2.** The order becomes branches, tags, PRs, then main.
`Forge::ensure_pull_request(repo, &PullRequest) -> Result<PrAction,
ForgeError>` with `PrAction { Created, Updated, Unchanged, LeftMerged }`;
`FakeForge` records it. `plan_push` grows a PR section; dry-run lists what
each PR would get. `GitHubForge`: `gh pr list --head`, `gh pr create`, `gh pr
edit`. `ForgejoForge`: `GET/POST/PATCH /repos/{o}/{r}/pulls`, per
`DESIGN_Forges.md` § 5. A repo that fails the gate gets no PRs.

---

## Work Items

### Phase 0 — Kernel

Every item is slice 1 unless marked **(slice 2)**.

- [ ] **0a.** Directive keys, `Block` and `Step` fields, `ConflictingLineInStep`,
  `InvalidBranchName` (pure ref-name check).
- [ ] **0b.** The fold: `Line`, `parents`, per-line trees, merge tree,
  `BranchSummary`, `FromOnLaterStep`. Port the spike's `plan()` and every
  spike test by name.
- [ ] **0c.** `MergeConflict` at region granularity: lift `composes` out of
  `check_duplicate_files` and call it from both (Decision 14).
- [ ] **0d.** `pr=` key and block forms, `PullRequest`, `PrWithoutBranch`,
  `PrUnknownBranch`, `PrDuplicate`.
- [ ] **0e.** `lock_text` line/parents/branches, printed only where not the
  default; exercises follow the line.
- [ ] **0f.** Testkit fixtures, the `line` coverage axis, the three
  properties. Delete `docs/spikes/spike-branches/` (Decision 19).

### Phase 1 — Replay and status

- [ ] **1a.** `commit` with `Vec` parents and a ref; branch refs; trailers on
  branch and merge commits only.
- [ ] **1b.** The worktree and `STEPS.md` from the last main step; `STEPS.md`
  branch and merge rows; `PULLS.md`; `chapter_ends` on main;
  `expected_branches`.
- [ ] **1c.** Determinism golden (`bower/tests/determinism.rs`) over the branch
  fixture: identical SHAs across runs, and the blast-radius assertion —
  editing a branch step leaves every unrelated tag's SHA alone. A linear
  book's SHAs are unchanged by this EPIC.
- [ ] **1d.** `repo_drift` compares branch names; `status` names a missing or
  unexpected branch.

### Phase 2 — Render

- [ ] **2a.** `compare` and `branch` templates, derived from `github` and
  declarable in `[links]`.
- [ ] **2b.** Footer line and compare link; the merge line for a pure merge;
  preprocessor test.

### Phase 3 — Push and pull requests

- [ ] **3a.** `Forge::push` takes the branches: each with its own lease, then
  tags, then main. `FakeForge` records the order; the dry run lists branches.
- [ ] **3b.** Golden: a refused gate pushes no branch.
- [ ] **3c. (slice 2)** `Forge::ensure_pull_request`, `PrAction`, `FakeForge`
  recording.
- [ ] **3d. (slice 2)** `plan_push` PR section; push order branches → tags →
  PRs → main.
- [ ] **3e. (slice 2)** `GitHubForge` and `ForgejoForge` last inches; the
  no-override test extended (`--merge-pr`, `--close-pr` absent).
- [ ] **3f. (slice 2)** Golden: a refused gate creates no PR.

### Phase 4 — Book and docs

- [ ] **4a.** `books/hello-playbook/src/ch07-try-it-on-a-branch.md`, before
  the appendix: `try/shout` (one `test_fail` step and a `pr=` block, never
  merged) and `feat/greet-many` (two steps, merged by an `op="none"` step
  after one unrelated main step).
- [ ] **4b. (slice 2)** The `from-char` PR in
  `books/rust4failures/src/ch03-rank.md`, once that chapter is listed.
- [ ] **4c.** `.okf/model/branch.md`, `.okf/model/pull-request.md`, the key
  table in `.okf/model/directive.md`, `BACKLOG.md`, `README.md`.
- [ ] **4d.** Flip slice 1's Status rows, append the corrigendum.

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
`replay__a_linear_book_keeps_its_shas`,
`status__reports_a_missing_branch_ref`,
`render__a_pure_merge_renders_its_line`,
`push__branches_go_before_main`,
`push__gate_refusal_pushes_no_branch`.

Slice 2: `push__branches_go_before_prs_and_main_goes_last`,
`push__ensures_a_pr_per_declared_branch_and_never_closes_one`,
`push__gate_refusal_creates_no_pr`.

## Key Files

| File | Role |
|---|---|
| `docs/spikes/spike-branches/` | the reference model; keep until Phase 0 lands, then delete |
| `bower-core/src/directive.rs` | four keys |
| `bower-core/src/block.rs`, `step.rs` | fields; `ConflictingLineInStep`; `composes` |
| `bower-core/src/plan.rs` | the fold, `Line`, `parents`, `BranchSummary`, `PullRequest`, lock text |
| `bower-core/src/lib.rs` | twelve error variants |
| `bower-testkit/src/{fixtures,coverage}.rs` | textures, the `line` axis |
| `bower/src/replay.rs` | parents, branch refs, `chapter_ends`, `expected_branches` |
| `bower/src/trailers.rs` | `Bower-Line`, `Bower-Merges`, `pulls_md` |
| `bower/src/status.rs` | branch drift |
| `bower/src/render.rs`, `config.rs` | footer, `compare`/`branch` templates |
| `bower/src/forge.rs`, `push.rs` | `ensure_pull_request`, order, dry-run |

## Reuse (do NOT recreate)

- `step.rs:198` `check_duplicate_files` — the region composition rule *is*
  the conflict rule; lift it into `composes`, do not write a second one.
- `plan.rs:126` `PlannedStep::tag` — the only tag format.
- `tree.rs:72` `apply_block` — the merge re-applies blocks through it; there
  is no second apply.
- `replay.rs:351` `expected_tags` — `expected_branches` sits beside it and
  `status` reads both from replay, never re-derives.
- `forge.rs:953` `push_branch_args` — one lease per branch, same function.
- `push.rs` marker gate — untouched; branches and PRs sit behind it.
- `DESIGN_Forges.md` § 6.1 `{prev_tag}` — the compare link is the same one.

## Compatibility

- **Preserves** every existing book byte for byte: no `branch=` means one
  line and one parent each, and nothing new is printed for either — not in
  the lock, not in a trailer, not in a rendered chapter, so no published SHA
  moves (Decision 13).
- **Adds** four keys, twelve errors, two templates, one generated file
  (`PULLS.md`), and — in slice 2 — one `Forge` method.
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
make book && make failures    # both books through the preprocessor and verify
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
   none of them and `FakeForge` records no call. **(Slice 2:** every PR with
   its action, too.**)**
7. `cargo tree -p bower-core -e normal` still prints one line.
8. `hello-playbook`'s steps 001–019, its existing lock lines, and its existing
   rendered chapters are byte-identical before and after (Decision 13).

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Merged-PR detection.** GitHub marks a PR merged when its head becomes reachable from base by push; Forgejo has "manually merged". Verify both on a real remote before Phase 3c; if either does not, the fallback is `PrAction::LeftOpen` and `status` reporting it. |
| 2 | **PR churn on regeneration.** A merged PR is immutable, and every regeneration gives the branch new SHAs. Decision 8 leaves the old PR alone; the alternative — one fresh PR per regeneration — is a forge full of duplicates. Live with drift reported by `status`, or accept churn? |
| 3 | **Closing without merging.** `pr_state="closed"` for a rejected PR — "reviewed, declined" is a *Failures* story. Cheap to add once Decision 8 stands. |
| 4 | **Review comments as book content.** A PR conversation is pedagogy. It is also a second body of prose the book would have to own; not before a real chapter asks for it. |
| 5 | ~~**Branches from branches, merges into branches.**~~ Settled 13 September 2026: `FromNotOnMain` and `MergeOnBranch` are the v1 fences (Decision 19). Lift when a chapter needs it; the fold generalises (a `BranchState` for main is the only change). |
| 6 | ~~**The spike's home.**~~ Settled 13 September 2026: its `rank_saga` becomes a `bower-testkit` fixture, and the spike is deleted once its tests pass as ported (Decision 19). |

---

*Spike: `docs/spikes/spike-branches/` — 16 tests green on rustc 1.75, three hand-mutations
each caught, `cargo run` prints the proposed lock and the graph. Drafted 9
September 2026 against `ImperialBower/bower` @ HEAD ("docs: file the forge
design and add it to the backlog").*
