# EPIC-09: Branches, merges, and pull requests — history with a shape (BRN)

## Context

Eight EPICs shipped. A Bower plan is a straight line: `step::order`
(`bower-core/src/step.rs:233`) produces one sequence per repo, `plan()`
(`bower-core/src/plan.rs:123`) folds one `TreeState` forward through it, and
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

| Component | Status |
|---|---|
| Kernel: four directive keys, `Line`, per-line fold, merge semantics | **Planned** |
| Kernel: `MergeConflict` at region granularity | **Planned** |
| Kernel: `PullRequest` on `RepoPlan.branches`, lock text | **Planned** |
| Testkit: branch textures and the `line` coverage axis | **Planned** |
| Replay: parents from the plan, branch refs, `PULLS.md`, trailers | **Planned** |
| `status`: branch-ref drift | **Planned** |
| Render: line and compare links in the footer | **Planned** |
| `push`: branch refs with lease, `Forge::ensure_pull_request` | **Planned** |
| Sample book chapter and slow-lane verify | **Planned** |

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
- On the forge: the branches, and a PR per declared `pr=`, open or merged.

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
   `PlannedStep::tag` (`plan.rs:108`) stays `step-{seq:03}-{id}` with `seq`
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
    complete `tree`, and `verify::run` (`bower/src/verify.rs:145`) already
    checks each in isolation. A merge step is verified against its own
    `expect` like any other.
11. **Exercises follow the line.** `bind_exercises` (`plan.rs:291`) currently
    answers with `planned[idx + 1]`; it becomes the next step on the same
    line, or the merge step for a merged branch's head. A branch head that is
    never merged has no answer — `ExerciseWithoutAnswer`, the existing rule.
    Play cells bind to a step regardless of line and are unchanged.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Which line a step is on | `Line { Main, Branch(String) }` on `PlannedStep` | 🔴 new |
| A step's parents | `PlannedStep::parents: Vec<usize>` (0 = scaffolding) | 🔴 new |
| Fork point | `BranchSummary::forked_from`, `from=` | 🔴 new |
| The merge tree | per-line fold in `plan()` | 🔴 new |
| A conflict | `BowerError::MergeConflict` + `check_duplicate_files` `step.rs:198` | 🟡 rule exists, reuse |
| A pull request | `PullRequest` on `RepoPlan::branches` | 🔴 new |
| One parent per commit | `Replayer::commit` `replay.rs:170` | 🟡 becomes `Vec<ObjectId>` |
| The only branch | `BRANCH` `replay.rs:27` | 🟡 becomes the main line's ref |
| Chapter-end anchors | `chapter_ends` `replay.rs:366` | 🟡 main steps only |
| What must be pushed | `expected_tags` `replay.rs:351` | 🟡 plus `expected_branches` |
| Is the build current? | `repo_drift` `status.rs:132` | 🟡 plus branch refs |
| The footer | `render::footer` `render.rs:286` | 🟡 line, compare |
| Link vocabulary | `LinkTemplates` `config.rs:85` | 🟡 plus `compare`, `branch` |
| Publishing | `Forge::push` `forge.rs:134`, `push_branch_args` `forge.rs:953` | 🟡 per branch |
| Ensuring a PR | `Forge::ensure_pull_request` | 🔴 new |

---

## Design

### Kernel — `bower-core`

`Directive` (`directive.rs:138`) and `Block` (`block.rs:25`) gain `branch`,
`from`, `merge`, `pr`. `Step` (`step.rs:28`) carries them through `group`,
which also rejects a step whose blocks disagree on `branch` or `merge`
(`ConflictingLineInStep`, beside `ConflictingRepoInStep`).

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

`PlannedStep` gains `line`, `parents`, `merges`; `RepoPlan` gains
`branches: Vec<BranchSummary>`. `lock_text` (`plan.rs:369`) prints
`line=… parents=…[ merges=…]` per step and a `[repo.branches]` table — the
spike's `lock_text` is the shape:

```
006 merge-from-char expect=pass line=main parents=003,005 merges=from-char

[failers.branches]
try/lookup-table from=001 head=002 merged=no pr="Try a lookup table for ranks" state=open
from-char from=003 head=005 merged=006 pr="Rank::from(char)" state=merged
```

New `BowerError` variants (`lib.rs:63`), all with a `Location`:
`InvalidBranchName`, `MergeUnknownBranch`, `MergeOnBranch`,
`BranchAlreadyMerged`, `UnknownFrom`, `FromNotOnMain`, `MergeConflict`,
`PrWithoutBranch`, `PrDuplicate`, `ConflictingLineInStep`.

`StepIndex::locate` (`plan.rs:241`) binds the `pr=` block form exactly as it
binds play cells; the key form rides on a branch step's own directive.

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
parents from it. Branch steps advance `refs/heads/<name>`; main steps and
merges advance `BRANCH`. `trailers::commit_message` (`trailers.rs:19`) adds
`Bower-Line: <line>` and, on merges, `Bower-Merges: <branch>`.
`trailers::pulls_md` writes `PULLS.md`; `final_blobs` (`replay.rs:330`)
includes it. `expected_branches(plan)` sits beside `expected_tags`, and
`repo_drift` compares both.

### Render — `bower`

`footer` (`render.rs:286`) appends `· on try/lookup-table` for branch steps
and `· merges from-char · [compare]` for merges. `LinkTemplates` gains
`compare` (`{base}/compare/{prev_tag}...{tag}` — the same shape
`DESIGN_Forges.md` § 6.1 proposes) and `branch` (`{base}/tree/{branch}`).
The reader's `checkout` line is unchanged: tags resolve on any line.

### Push — `bower`

`Forge::push` pushes every branch in `expected_branches` with its own lease
via `push_branch_args` — already parameterised by branch — then tags, then
ensures PRs, then main. `Forge::ensure_pull_request(repo, &PullRequest) ->
Result<PrAction, ForgeError>` with `PrAction { Created, Updated, Unchanged,
LeftMerged }`; `FakeForge` records it. `plan_push` (`push.rs:162`) grows a
PR section; dry-run lists what each PR would get. `GitHubForge`: `gh pr list
--head`, `gh pr create`, `gh pr edit`. `ForgejoForge`: `GET/POST/PATCH
/repos/{o}/{r}/pulls`, per `DESIGN_Forges.md` § 5. The marker gate is not
touched — a repo that fails it gets no branches and no PRs either.

---

## Work Items

### Phase 0 — Kernel

- [ ] **0a.** Directive keys, `Block` and `Step` fields, `ConflictingLineInStep`,
  `InvalidBranchName` (pure ref-name check).
- [ ] **0b.** The fold: `Line`, `parents`, per-line trees, merge tree,
  `BranchSummary`. Port the spike's `plan()` and every spike test by name.
- [ ] **0c.** `MergeConflict` at region granularity, reusing
  `check_duplicate_files`'s composition rule.
- [ ] **0d.** `pr=` binding, `PullRequest`, `PrWithoutBranch`, `PrDuplicate`.
- [ ] **0e.** `lock_text` line/parents/branches; exercises follow the line.
- [ ] **0f.** Testkit fixtures, the `line` coverage axis, the three properties.

### Phase 1 — Replay and status

- [ ] **1a.** `commit` with `Vec` parents and a ref; branch refs; trailers.
- [ ] **1b.** `chapter_ends` anchors on main; `expected_branches`; `PULLS.md`.
- [ ] **1c.** Determinism golden (`bower/tests/determinism.rs`) over the branch
  fixture: identical SHAs across runs, and the blast-radius assertion —
  editing a branch step leaves every unrelated tag's SHA alone.
- [ ] **1d.** `repo_drift` checks branch refs; `status` names a missing or
  moved branch.

### Phase 2 — Render

- [ ] **2a.** `compare` and `branch` templates, derived per forge kind.
- [ ] **2b.** Footer line and compare link; preprocessor test.

### Phase 3 — Push and pull requests

- [ ] **3a.** `Forge::ensure_pull_request`, `PrAction`, `FakeForge` recording.
- [ ] **3b.** `plan_push` PR section; push order branches → tags → PRs → main.
- [ ] **3c.** `GitHubForge` and `ForgejoForge` last inches; the
  no-override test extended (`--merge-pr`, `--close-pr` absent).
- [ ] **3d.** Goldens: a refused gate creates no PR and pushes no branch.

### Phase 4 — Book and docs

- [ ] **4a.** A branch chapter in `books/hello-playbook` (the abandoned
  experiment) and the `from-char` PR in `books/rust4failures/src/ch03-rank.md`.
- [ ] **4b.** `.okf/model/branch.md`, `.okf/model/pull-request.md`, the key
  table in `.okf/model/directive.md`, `BACKLOG.md`, `README.md`.
- [ ] **4c.** Flip Status rows, append the corrigendum.

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
`plan__pr_on_a_main_step_is_refused`, `plan__two_prs_on_one_branch_is_refused`,
`plan__branch_named_main_is_refused`, `plan__exercise_on_a_merged_head_answers_with_the_merge`,
`replay__chapter_end_tag_never_points_at_a_branch`,
`status__reports_a_missing_branch_ref`,
`push__branches_go_before_prs_and_main_goes_last`,
`push__ensures_a_pr_per_declared_branch_and_never_closes_one`,
`push__gate_refusal_creates_no_pr`.

## Key Files

| File | Role |
|---|---|
| `docs/spikes/spike-branches/` | the reference model; keep until Phase 0 lands, then delete |
| `bower-core/src/directive.rs` | four keys |
| `bower-core/src/block.rs`, `step.rs` | fields; `ConflictingLineInStep`; conflict rule reuse |
| `bower-core/src/plan.rs` | the fold, `Line`, `parents`, `BranchSummary`, `PullRequest`, lock text |
| `bower-core/src/lib.rs` | ten error variants |
| `bower-testkit/src/{fixtures,coverage}.rs` | textures, the `line` axis |
| `bower/src/replay.rs` | parents, branch refs, `chapter_ends`, `expected_branches` |
| `bower/src/trailers.rs` | `Bower-Line`, `Bower-Merges`, `pulls_md` |
| `bower/src/status.rs` | branch drift |
| `bower/src/render.rs`, `config.rs` | footer, `compare`/`branch` templates |
| `bower/src/forge.rs`, `push.rs` | `ensure_pull_request`, order, dry-run |

## Reuse (do NOT recreate)

- `step.rs:198` `check_duplicate_files` — the region composition rule *is*
  the conflict rule; do not write a second one.
- `plan.rs:241` `StepIndex::locate` — binds `pr=` blocks; do not invent a
  second binding rule.
- `plan.rs:108` `PlannedStep::tag` — the only tag format.
- `replay.rs:351` `expected_tags` — `expected_branches` sits beside it and
  `status` reads both from replay, never re-derives.
- `forge.rs:953` `push_branch_args` — one lease per branch, same function.
- `push.rs` marker gate — untouched; branches and PRs sit behind it.
- `DESIGN_Forges.md` § 6.1 `{prev_tag}` — the compare link is the same one.

## Compatibility

- **Preserves** every existing book: no `branch=` means one line, one parent
  each, `parents=` in the lock is the only visible change. `bower.lock`
  changes shape for every book, once.
- **Adds** four keys, ten errors, two templates, one `Forge` method, one
  generated file (`PULLS.md`).
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
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hp
git -C /tmp/hp log --graph --oneline --all --decorate --date-order
cargo run -p bower -- --book books/hello-playbook verify
cargo run -p bower -- --book books/hello-playbook push -o /tmp/hp   # dry run
```

Exit criteria:

1. The sample book's graph shows one unmerged branch and one two-parent merge;
   `git checkout try/lookup-table && cargo test` fails as the book claims.
2. Two builds of the same book produce identical SHAs for every ref, branches
   included.
3. Editing a branch step changes the SHAs of that step, its successors on the
   line, and the merge — and nothing else.
4. A merge with a conflicting region and no resolution is a plan-time error
   naming the step, the branch, the file, and the region.
5. `verify` reports the same verdicts with branches as without; a merge step
   is verified on its own tree.
6. A dry-run `push` lists every branch, every PR with its action, and the
   order; a gate refusal lists none of them and `FakeForge` records no call.
7. `cargo tree -p bower-core -e normal` still prints one line.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Merged-PR detection.** GitHub marks a PR merged when its head becomes reachable from base by push; Forgejo has "manually merged". Verify both on a real remote before Phase 3c; if either does not, the fallback is `PrAction::LeftOpen` and `status` reporting it. |
| 2 | **PR churn on regeneration.** A merged PR is immutable, and every regeneration gives the branch new SHAs. Decision 8 leaves the old PR alone; the alternative — one fresh PR per regeneration — is a forge full of duplicates. Live with drift reported by `status`, or accept churn? |
| 3 | **Closing without merging.** `pr_state="closed"` for a rejected PR — "reviewed, declined" is a *Failures* story. Cheap to add once Decision 8 stands. |
| 4 | **Review comments as book content.** A PR conversation is pedagogy. It is also a second body of prose the book would have to own; not before a real chapter asks for it. |
| 5 | **Branches from branches, merges into branches.** `FromNotOnMain` and `MergeOnBranch` are the v1 fences. Lift when a chapter needs it; the fold generalises (a `BranchState` for main is the only change). |
| 6 | **The spike's home.** Keep `docs/spikes/spike-branches/` out of the workspace members (edition 2021, no clippy-pedantic), or promote it to a `bower-testkit` fixture and delete it? Delete after Phase 0 is the lean. |

---

*Spike: `docs/spikes/spike-branches/` — 16 tests green on rustc 1.75, three hand-mutations
each caught, `cargo run` prints the proposed lock and the graph. Drafted 9
September 2026 against `ImperialBower/bower` @ HEAD ("docs: file the forge
design and add it to the backlog").*
