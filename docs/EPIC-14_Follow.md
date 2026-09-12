# EPIC-14: `bower follow` — the reader's CLI (FLW)

## Summary

- **Builds:** `bower-follow`, a reader's CLI (`start`, `where`, `diff`, `next`,
  `check`) that walks a generated repo and runs the book's own verifier.
- **Why:** every Bower tool faces the author and needs `bower.toml`; the reader
  gets `git checkout {tag}` and nothing else.
- **Shape:** a `.bower/steps` manifest in the final tree, and the verification
  matrix moved into `bower-core/src/verdict.rs` so `verify`, the exercise box,
  and `follow check` share one definition.
- **Proves it:** `bower-follow start --from test-that-fails && bower-follow
  check` exits 1, and `cargo tree -p bower-follow` lists only two crates.
- **Status:** Planned, filed 10 September 2026, nothing started.

---

## Context

Every Bower tool faces the author. `bower` loads `bower.toml` before it
dispatches anything (`bower/src/main.rs:145`), so every subcommand assumes a
book on disk (`bower/src/main.rs:47`). The reader gets a generated repository
and one instruction under each block: `git checkout {tag}`
(`bower/src/render.rs:342`, derived at `bower/src/config.rs:201`). That works
for a reader who is comfortable in git. For one who is not, it is a wall.

The generated repo already says a lot about itself. A real build of
`books/hello-playbook` (21 commits, 26 tags) carries:

- **Tags.** `step-{seq:03}-{id}` per step (`bower-core/src/plan.rs:108`), plus
  `<chapter-stem>-end` per chapter (`bower/src/replay.rs:146`, `:396`).
- **Trailers.** `Book-Source`, `Book-Url` only when a `site` is set,
  `Bower-Step: <repo>/<seq>`, and `Generated-By` (`bower/src/trailers.rs:19`–`35`).
  The scaffolding commit is untagged and has no `Book-Source`
  (`bower/src/trailers.rs:42`).
- **`STEPS.md`.** A markdown table of `# | Tag | Expect | Subject | Source`
  (`bower/src/trailers.rs:82`). It exists only in the final tree
  (`bower/src/replay.rs:326`–`343`), because a file listing every step cannot
  sit in a commit that comes before those steps.

**The question this EPIC turns on: can a reader rebuild a step's
verification from the repo alone? No.** Four things are missing:

1. **The commands.** `check` and `verify` live only in `bower.toml`
   (`bower/src/config.rs:70`–`75`). Their defaults are `cargo check` and `cargo test`
   (`bower/src/config.rs:22`–`23`). Nothing in the repo records either one. The
   book prints one of them in the exercise box (`bower/src/render.rs:382`,
   `:396`), and only there.
2. **`expect`, machine-readably.** It appears only as a column of a markdown
   table. The table is not escaped: `step.msg` goes into the Subject cell as
   written (`bower/src/trailers.rs:97`–`103`), so a subject containing `|`
   breaks the row. An exercise's task is folded into that same cell
   (`bower/src/trailers.rs:93`–`96`). No trailer carries `expect`.
3. **What a step touched, by region.** `PlannedStep::files` is a list of
   paths (`bower-core/src/plan.rs:47`). The regions live on
   `Step.blocks` (`bower-core/src/step.rs:31`) and never reach the plan. With
   `keep_region_markers = false` the markers are stripped from the tree too.
   In the sample book, step 011 writes region `tests` of `src/lib.rs` and
   step 012 writes region `greet` of the same file (`books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md:70`, `:108`).
   The repo shows two edits to one file and cannot tell you which is the
   test.
4. **The exercise's answer.** `Exercise::answer` (`bower-core/src/plan.rs:84`)
   is implicit in the repo ("the next row"). The shipped binding sets it to
   `planned[idx + 1]` (`bower-core/src/plan.rs:336`). EPIC-13 plans
   `Bower-Exercise-Tests`, `Bower-Solution`, and `Bower-Solves` trailers,
   but only for strict exercises, and a trailer sits on one commit. Nothing
   lists the steps in order, with their commands, where every checkout can
   read it.

The verdict logic cannot be reused as it stands, either. `verdict_for`
(`bower/src/verify.rs:232`) is a pure function, but it sits in the `bower`
crate, next to `gix`, `clap`, `serde`, `time`, and `toml` (`bower/Cargo.toml`).
A reader who wants to know "is my tree right?" would have to build all of
that first. Idea A5 names that cost (`docs/bower-ideas.md:265`–`267`, Part F Q6
at `:844`).

This EPIC gives the generated repo what it lacks and gives the reader a small
binary that reads it. `.bower/steps` is `STEPS.md`'s machine-readable twin,
written by a pure kernel function. `bower-follow` depends on `bower-core` and
nothing else. It drives the reader's own `git` and runs the book's own
verification matrix, which is promoted into the kernel so that `verify`, the
exercise box, and `follow check` are one definition.

**This EPIC does not** send anything anywhere. Reader signals (idea C7,
`docs/bower-ideas.md:745`–`766`) are deferred by design: `bower-follow` has no
network code, and its only remote operation is the `git clone` it asks git to
do. It does not grade, collect, or link a reader's answer (§ Scope, *the
reader's answer*). It does not follow a chapter across two repos. It does not
generate a classroom (C3). It does not open a browser. It never commits,
pushes, or fetches for the reader. It adds nothing to the author's
`bower` subcommands except `verify --hands-on`, and it changes no tag, trailer,
or step tree. The final tree gains one file.

---

## Status

| Component | Status |
|---|---|
| Kernel: `Touch` per planned step (file, region, op) | **Planned** |
| Kernel: the verification matrix promoted (`Outcome`, `Verdict`, `judge`, `reader_command`) | **Planned** |
| Kernel: `.bower/steps` writer and parser, round-trip | **Planned** |
| Kernel: hands-on classification and `hands_on_tree` | **Planned** |
| Replay: `[repos.X.follow]` config, `.bower/steps` in the final tree, `status` coverage | **Planned** |
| `bower verify --hands-on`: which steps are real exercises | **Planned** |
| `bower-follow` crate: git adapter, state file, dependency-light gate | **Planned** |
| `follow start` / `where` / `diff` | **Planned** |
| `follow next` / `check` (canonical) | **Planned** |
| `follow next --hands-on`, attempts, authored exercises | **Planned** |
| Lines (EPIC-09), stale tags, `bower follow` dispatch | **Planned** |
| Sample golden, README, spec § 7 / § 16.3 | **Planned** |

---

## Goals

- Make the generated repo a **guided path**. A reader always knows **which
  step** they are on, can **diff** against the known-good tree, and can run
  **the book's own verifier** on their own work.
- Everything comes from the **repo's own metadata**. There is no second
  download and no `bower.toml`, and the book source is not needed.
- **Any step can be an exercise** (`--hands-on`). Bower hands over the tests
  and the build files and holds back the rest. Authored exercises (EPIC-13)
  are the verified special case.
- Installing it should not be the first hurdle. `bower-follow` builds from
  **`bower-core` alone**, and `cargo tree` proves it.
- The **kernel stays pure**. The manifest is a `String` in and a value out,
  the matrix is a function, and git and processes stay at the edge.

## Scope

### Things

| Thing | What it is |
|---|---|
| **Manifest** | `.bower/steps`: per repo, the commands and hands-on patterns; per step, seq, id, tag, expect, line, chapter, anchor, URL, and the given/yours/mixed split, plus the exercise and its answer. |
| **Canonical tree** | The tree at a step's tag. It is byte-identical to the tree `verify` judged (§ Decisions 3). |
| **Position** | Where the reader is: `at` (a step) and optionally `attempting` (the step they are writing). |
| **Hands-on tree** | `at`'s tree with only the *given* files of the next step laid over it. |
| **Verdict** | The spec § 6 matrix applied to the reader's tree, for the target step's `expect`. |

### Business requirements

1. `follow` never needs anything the repo does not carry. If `.bower/steps` is
   absent, the repo predates this EPIC or is not Bower's, and `follow` says
   so and stops.
2. `follow` never destroys reader work. A write that would overwrite a file
   the reader changed is refused unless they pass `--force`, and the refusal
   lists the files.
3. `follow` writes only two things: the working tree, and one state file
   inside the git directory. The only ref it moves is the one `start` creates.
4. `check` answers with the same verdict `bower verify` gives for the same
   tree and the same `expect`. That is one function, not two copies.
5. A hands-on state that already meets the target step's claim is reported,
   because such a step gives the reader nothing to prove.
6. A step that mixes given and held-back changes in one file is refused for
   hands-on, by name. It is never half-applied.

### Business logic

§ Decisions and § Design, driven out test-first by § Test Plan. The pure
half (manifest, matrix, classification, hands-on tree) lands in the kernel;
`git` and processes stay in `bower-follow`.

### The six commands

```
bower-follow start [<url>] [--from <step>]  # clone if a URL is given; branch `follow` at the step
bower-follow where                          # step k of n, tag, chapter, book URL, exercise, mode
bower-follow diff  [--stat]                 # working tree vs the canonical tree of the position
bower-follow next  [--force] [--no-check] [--reveal]  # show the diff, write its files, run check
bower-follow next --hands-on                # write only the given files; you write the rest
bower-follow check                          # the book's verification, against your tree
```

### The reader's answer

`BACKLOG.md` lists this gap: "Exercises have no place for a *reader's*
answer" (exercises spec § 2). `follow check` covers **half** of it. It judges
the reader's answer, on their machine, with the verifier that gated the book:
a verdict, not a place. The reader's own fork can carry a workflow that runs
`follow check` on push, which gives the answer a home in the reader's repo
(Phase 4, open question 3). The book still links no Discussion and no
submission, and Bower still collects nothing. The gap entry should be
narrowed, not closed.

### Not in scope

Reader telemetry (C7), classroom export (C3), following across repos,
following an EPIC-09 branch past its head, region-granular hands-on (open
question 1), and running the generated repo's own gate (`make ayce` in
`.github/workflows/ci.yml`). `follow check` runs Bower's `check`/`verify`,
which is the same gap `docs/TECHNICAL_DEBT.md:27` records for `bower verify`.

---

## Decisions

1. **A manifest, not scraping.** Parsing `STEPS.md` would fail on a pipe in a
   subject and on the exercise suffix in a cell, and it would still leave out
   the commands and the regions. `.bower/steps` is written beside
   `STEPS.md` by `final_blobs` (`bower/src/replay.rs:330`) and has the same
   final-tree-only rule, for the same reason. `follow` reads it from
   `main`'s tree (`git show <main>:.bower/steps`), never from the working
   tree, so it works at any checkout.
2. **Line text, not TOML.** The kernel promises "no format crate in any
   signature" (`bower-core/src/lib.rs:13`), and `bower-follow` must not pull in
   `serde_derive`. The format has `bower.lock`'s shape (`bower-core/src/plan.rs:369`):
   one record per line, with `key=value` fields and `{:?}`-quoted strings. A
   `format 1` header versions it.
3. **The canonical tree is the verified tree.** Replay commits scaffolding
   plus the step's tree (`bower/src/replay.rs:129`–`130`), and verify judges
   exactly the same set (`bower/src/verify.rs:194`–`195`). The final step
   differs only by `STEPS.md` and `.bower/steps`. A test pins this, so "your
   tree is canonical and still fails" can only mean the toolchain differs
   (spec § 16).
4. **One matrix, in the kernel.** `verdict_for`, `Outcome`, and `Verdict`
   (`bower/src/verify.rs:44`–`65`, `:232`) move to `bower-core/src/verdict.rs`.
   So does the rule that decides whether `verify` runs at all
   (`bower/src/verify.rs:202`), as `judge(expect, run)`. The runner is a
   closure the caller provides, which keeps the kernel free of processes.
   `reader_command` (`bower/src/render.rs:396`) moves with them, so the
   exercise box names the same command `check` runs. `bower::verify`
   re-exports all of it, and no caller changes.
5. **Hands-on is file-granular, classified by region.** A block counts as
   *given* if its file matches a `given` path pattern or its region matches
   a `given` region name. A file the next step changes is **given** if every
   block touching it is given, **yours** if no block touching it is, and
   **mixed** otherwise. The hands-on tree is `at`'s tree plus the given
   files from the next tree. That is a pure function over two `TreeState`s,
   and git can compute it from two tags, so the author's
   `verify --hands-on` and the reader's `next --hands-on` use one
   definition. Mixed steps are refused, not guessed at (open question 1).
6. **Default given set.** When a repo declares no pattern it gets
   `Cargo.toml`, `Cargo.lock`, `tests/`, `benches/`, and the region `tests`.
   These defaults sit beside `DEFAULT_CHECK` (`bower/src/config.rs:22`). An
   *appended* test with no region (step 010, `op="append"`, `ch04…md:48`) is
   classified *yours*, and the README says so.
7. **`check` judges the target, not the position.** At a plain step the
   target is `at`. After `next --hands-on`, and on any step carrying an
   exercise, the target is the step being attempted, which for an exercise is
   `Exercise::answer` (EPIC-13's `solution=`, defaulting to the next step).
   Judging an exercise step by its own `test_fail` would declare a reader
   correct for doing nothing. On a strict exercise (EPIC-13 `tests=`),
   `check` uses EPIC-13's named-tests verdict: the named tests ran and
   passed. That parser is pure too, so it moves into the kernel with the
   matrix (Decision 4).
8. **A hidden solution stays hidden until asked for.** When an exercise is
   `reveal="never"` (EPIC-13), `next` from the exercise does not print the
   solution's diff or write its files unless the reader passes `--reveal`.
   `check` going green does not need it: an accepted attempt never shows the
   book's answer. The solution tag is still in the repo, because Bower hides
   nothing from `git`. It only declines to be the one that spoils it.
9. **Position lives in the git directory.** The state file is
   `$(git rev-parse --git-dir)/bower-follow`, with three lines: repo, at, and
   attempting. It is never committed and never pushed. If it is lost,
   `where` can recover the position whenever `HEAD`'s tree equals a step's
   tree.
10. **`bower-follow` shells out to `git`.** The reader already has git, since
   they cloned with it. It handles packed refs, which the loose-tag reader
   in `status` does not (`bower/src/status.rs:148`, `:206`), and a fresh
   clone always has packed refs. Linking `gix` would make `bower-follow`
   as heavy as `bower`. `forge.rs` already shells out to `git` for push
   (`bower/src/forge.rs:591`), so there is precedent.
11. **One line at a time (EPIC-09).** `next` walks the manifest's `line=`
    order, not `seq + 1`, because EPIC-09 makes `seq` global across lines.
    The main line is the default. `start --from <branch-step>` is allowed,
    and `next` stops at the branch head. Branch refs and `gh-pages` are
    never read.
12. **No network, no browser.** `where` prints the URL, and the reader
    opens it. A stale clone is detected, not fixed. A step tag that is not
    an ancestor of `main` means the book was regenerated after the clone, and
    `follow` prints the `git fetch --tags --force` the reader should run.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| A step's tag | `PlannedStep::tag` `plan.rs:108` | ✅ reuse |
| The human step list | `steps_md` `trailers.rs:73` | ✅ unchanged |
| The machine step list | `manifest_text` / `parse_manifest` | 🔴 new |
| Files in the final tree only | `final_blobs` `replay.rs:330` | 🟡 plus `.bower/steps` |
| What a step touched | `PlannedStep::touches: Vec<Touch>` | 🔴 new (shared with EPIC-09) |
| Commands a step is judged by | `RepoConfig::{check, verify}` `config.rs:73` | 🟡 now also written to the repo |
| The § 6 matrix | `verdict_for` `verify.rs:232` | 🟡 moves to `bower-core` |
| Which command the reader runs | `reader_command` `render.rs:396` | 🟡 moves to `bower-core` |
| Given / yours / mixed | `HandsOn`, `classify` | 🔴 new |
| The hands-on tree | `hands_on_tree` over `TreeState` `tree.rs` | 🔴 new |
| An exercise and its answer | `Exercise` `plan.rs:71` | ✅ reuse; EPIC-13 may rebind `answer` |
| The reader's position | `.git/bower-follow` | 🔴 new |
| The marker | `book_named_in` `trailers.rs:65` | ✅ reuse, `start` refuses a non-Bower clone |

---

## Design

### Kernel — `bower-core`

`plan()` records each block's footprint as it builds the `PlannedStep`
(`bower-core/src/plan.rs:167`):

```rust
pub struct Touch { pub file: String, pub region: Option<String>, pub op: Op }
// PlannedStep gains:
pub touches: Vec<Touch>,          // one per block × path, document order
```

EPIC-09's fold keeps `(seq, Touch)` for its conflict rule. Whichever EPIC
lands first defines the type, and the other one imports it.

`bower-core/src/verdict.rs` (new, moved from `bower/src/verify.rs`):

```rust
pub struct Outcome { pub command: String, pub success: bool, pub stderr: String }
pub enum Verdict { Upheld, Broken { happened: String, command: String, stderr: String }, Skipped }
pub enum Which { Check, Verify }
pub fn verdict_for(expect: Expect, check: &Outcome, verify: Option<&Outcome>) -> Verdict;
/// The whole matrix, with the runner supplied: `run` is called for `Check`,
/// then for `Verify` only when the claim needs it. No I/O here.
pub fn judge<E>(expect: Expect, run: impl FnMut(Which) -> Result<Outcome, E>) -> Result<Verdict, E>;
pub fn reader_command(expect: Expect) -> Option<Which>;
```

`bower-core/src/follow.rs` (new):

```rust
pub struct HandsOnSpec { pub files: Vec<String>, pub regions: Vec<String> } // "dir/", exact, "*.ext"
pub struct HandsOn { pub given: Vec<String>, pub yours: Vec<String>, pub mixed: Vec<String> }
pub fn classify(step: &PlannedStep, prev: &TreeState, spec: &HandsOnSpec) -> HandsOn;
pub fn hands_on_tree(prev: &TreeState, next: &TreeState, given: &[String]) -> TreeState;

pub struct RepoHeader { pub book: String, pub repo: String, pub site: Option<String>,
                        pub check: String, pub verify: String, pub given: HandsOnSpec }
pub fn manifest_text(plan: &RepoPlan, header: &RepoHeader) -> String;
pub struct Manifest { pub header: RepoHeader, pub steps: Vec<ManifestStep> }
pub struct ManifestStep { pub seq: usize, pub id: String, pub tag: Option<String>, pub expect: Expect,
                          pub line: String, pub chapter: String, pub anchor: String,
                          pub url: Option<String>, pub hands_on: HandsOn,
                          pub exercise: Option<(String /*task*/, String /*answer*/)> }
pub fn parse_manifest(text: &str) -> Result<Manifest, ManifestError>;
```

The header's strings come from the caller. The kernel never sees
`bower.toml`, and `RepoSpec` stays `Copy` (`bower-core/src/source.rs:100`).
The URL rule is `book_url` (`bower/src/trailers.rs:110`). It moves to the
kernel so that the trailer and the manifest cannot disagree.

The file itself, for the sample book (real steps, abridged):

```
# .bower/steps — generated by bower v0.1.0; read by bower-follow. Do not edit.
format 1
book "hello-playbook"
repo "hello-playbook"
site "https://abstecker.github.io/hello-playbook"
check "cargo check"
verify "cargo test"
given files="Cargo.toml,Cargo.lock,tests/,benches/" regions="tests"

000 scaffolding line=main
011 test-that-fails tag=step-011-test-that-fails expect=test_fail line=main chapter="src/ch04-tests-and-failing-on-purpose.md" anchor=step-test-that-fails given=src/lib.rs
    exercise answer=test-that-passes tests=greet_ignores_stray_whitespace reveal=fold task="Make the test pass"
012 test-that-passes tag=step-012-test-that-passes expect=pass line=main chapter="src/ch04-tests-and-failing-on-purpose.md" anchor=step-test-that-passes yours=src/lib.rs
013 wont-compile tag=step-013-wont-compile expect=compile_fail line=main chapter="src/ch04-tests-and-failing-on-purpose.md" anchor=step-wont-compile yours=src/scratch.rs,src/lib.rs
```

`000` is the untagged scaffolding commit. `follow` resolves it as the root
commit of `main`, which lets a reader follow step 001 hands-on. The
`tests=` and `reveal=` fields appear only once EPIC-13 lands. The exercise
line and EPIC-13's trailers come from the same `Exercise` value, and a test
asserts that they agree.

### Replay — `bower`

`[repos.X.follow]` in `bower.toml` takes `given = ["…"]`, where
`region:<name>` entries are regions. The table is optional, and
`deny_unknown_fields` still applies (`bower/src/config.rs:264`).
`final_blobs` inserts `.bower/steps` beside `STEPS.md`. `status` compares
`final_blobs` (`bower/src/main.rs:448`), so it reports a stale manifest with
no new code. `verify.rs` calls `judge` with its existing runner
(`bower/src/verify.rs:289`).

`bower verify --hands-on` builds each step's hands-on tree from the plan and
runs the target's matrix on it. It reports one of four results per step:
**exercise** (the claim does not yet hold, so the reader has something to
do), **trivial** (the claim already holds), **mixed**, or **reading**
(nothing yours). It reports and never fails the build. Whether it should gate
is open question 2.

### Reader — `bower-follow` (new crate)

`bower-follow/Cargo.toml` depends on `bower-core` and nothing else.
Arguments are parsed by hand. There are six verbs and four flags, and
`clap` would be most of the build.

```rust
trait Git {                               // the only side-effect seam; FakeGit in tests
    fn show(&self, rev: &str, path: &str) -> Result<Option<Vec<u8>>, FollowError>;
    fn tree(&self, rev: &str) -> Result<BTreeMap<String, (Vec<u8>, bool)>, FollowError>; // exec bit kept
    fn is_ancestor(&self, a: &str, b: &str) -> Result<bool, FollowError>;
    fn git_dir(&self) -> Result<PathBuf, FollowError>;
    fn switch_new(&self, branch: &str, rev: &str) -> Result<(), FollowError>; // `start` only
    fn clone_into(&self, url: &str, dir: &Path) -> Result<(), FollowError>;   // `start <url>` only
}
```

- **`start`** clones if a URL is given, refuses a repo whose `STEPS.md` fails
  `book_named_in`, refuses a dirty tree, and creates branch `follow` at the
  step (default: the first step). If `follow` already exists it refuses
  unless given `--restart`. It writes the state file and then prints `where`.
- **`where`** prints `step 11 of 20 · step-011-test-that-fails ·
  ch04-tests-and-failing-on-purpose`, the book URL (or the `Book-Source` path
  when the book has no site), the exercise task, the mode, and a stale-clone
  warning when there is one.
- **`diff`** runs `git diff <target-tag>`. When the reader is attempting a
  step, the target is that step. `STEPS.md` and `.bower/steps` are left out.
- **`next`** prints the step's `--stat` and diff, then writes the next tree's
  changed files. It refuses any file the reader changed away from `at`'s
  tree unless `--force`, and then runs `check` unless `--no-check`. If the
  reader is attempting a step and `check` is green, `next` *accepts* their
  tree: `at` becomes the attempted step and the reader's version stays.
  After that, `diff` shows how their answer differs from the book's.
- **`next --hands-on`** refuses a mixed step by name. Otherwise it writes the
  given files, sets `attempting`, and runs `check` once. A green result there
  is the *trivial* case: `follow` says so and suggests following the step as
  reading.
- **`check`** runs `judge(target.expect, …)` in the working tree, using the
  manifest's commands, split on whitespace exactly as `verify` splits them
  (`bower/src/verify.rs:289`). It prints the verdict, and says whether the
  tree is canonical, so a failure on a canonical tree reads as "your
  toolchain differs from the book's", not "you are wrong". Exit codes: 0
  when the claim holds, 1 when it does not, 2 when `follow` itself cannot
  run.

---

## Work Items

### Phase 0 — The metadata the repo lacks

- [ ] **0a.** `Touch` on `PlannedStep`, filled in `plan()` (`plan.rs:167`);
  `lock_text` is unchanged. Test: `plan__touches_name_file_region_and_op`.
- [ ] **0b.** Move `Outcome`, `Verdict`, `verdict_for`, `reader_command` into
  `bower-core/src/verdict.rs`; add `judge`; re-export from `bower::verify`
  and `bower::render`. Every `matrix__*` test moves with its function.
- [ ] **0c.** `bower-core/src/follow.rs`: `RepoHeader`, `manifest_text`,
  `parse_manifest`, `ManifestError` (line-located). `book_url` moves in.
- [ ] **0d.** `[repos.X.follow] given` in `config.rs`; defaults beside
  `DEFAULT_CHECK`; `config__follow_given_defaults_and_region_entries`.
- [ ] **0e.** `final_blobs` writes `.bower/steps`; `STEPS.md` unchanged.
  Update the determinism golden (`bower/tests/determinism.rs`) and
  `HELLO_PLAYBOOK_FINAL_PATHS` (`bower-testkit/src/fixtures.rs`), since the
  last commit's SHA changes once.
- [ ] **0f.** Testkit: `parse_manifest(manifest_text(p)) ≅ p` over `arb_book`
  (`bower-testkit/src/generators.rs:52`); a subject with `|` round-trips.
- [ ] **0g.** `make purity` green: `bower-core` still has no dependencies.

### Phase 1 — `bower-follow`: the crate, `start`, `where`, `diff`

- [ ] **1a.** New workspace member `bower-follow` (`Cargo.toml:3`); `Git`
  trait, `ShellGit`, `FakeGit`; the state file.
- [ ] **1b.** A `make follow-light` target, folded into `lint` like `purity`,
  that asserts `cargo tree -p bower-follow -e normal` names only
  `bower-core`.
- [ ] **1c.** `start` (marker check, dirty-tree refusal, `follow` branch,
  `--from`, `--restart`), `where`, `diff`.
- [ ] **1d.** Integration tests that build the sample book with the real
  `bower` binary into a temp dir, as `determinism.rs` does, then drive
  `bower-follow` against it.

### Phase 2 — `next` and `check`, canonical

- [ ] **2a.** `check` through `judge` with a process runner; exit codes 0/1/2.
- [ ] **2b.** `next`: diff, file-level write that keeps the exec bit,
  refusal list, `--force`, `--no-check`.
- [ ] **2c.** `follow__canonical_tree_equals_the_verified_tree` for every
  step of the sample book (Decision 3).
- [ ] **2d.** A test book with `check = "true"`/`verify = "false"` drives
  every row of the matrix through `check` without cargo.

### Phase 3 — Hands-on and exercises

- [ ] **3a.** `classify` and `hands_on_tree` in the kernel; properties: an
  all-given split gives the next tree; an empty given set gives the previous tree.
- [ ] **3b.** `next --hands-on`, `attempting`, the trivial-case message, and
  accept-on-green.
- [ ] **3c.** `check` targets `Exercise::answer` on an exercise step
  (Decision 7), built on whatever binding EPIC-13 ships.
- [ ] **3d.** `bower verify --hands-on` with its four-way report; the slow
  lane runs it on the sample book.

### Phase 4 — Lines, staleness, book, docs

- [ ] **4a.** `line=` in the manifest; `next` follows the line; a branch
  head ends the walk (EPIC-09 Decision 5).
- [ ] **4b.** Stale-clone detection (Decision 12) and a recovered position
  when the state file is gone.
- [ ] **4c.** `bower follow …` handled as an external subcommand that execs
  `bower-follow`, dispatched **before** `BookConfig::load` (`main.rs:145`).
- [ ] **4d.** Template README: a "Follow along" section beside the devenv
  path (spec § 16.3); a documented fork workflow running `follow check`.
- [ ] **4e.** Spec § 7 CLI surface; `.okf` model for the manifest;
  `BACKLOG.md` narrows the reader's-answer gap; flip the Status rows.

---

## Test Plan

Kernel: `plan__touches_name_file_region_and_op` (011 → `tests`, 012 →
`greet`), every `matrix__*` test moved with its function,
`judge__skips_verify_after_a_compile_fail_claim`,
`judge__never_runs_anything_for_none`,
`manifest__round_trips_every_generated_book` (property over `arb_book`),
`manifest__pipe_in_a_subject_survives` (the `STEPS.md` weakness, pinned),
`manifest__unknown_format_version_is_refused`,
`classify__region_tests_is_given_and_region_greet_is_yours`,
`classify__two_regions_of_one_file_split_is_mixed`,
`hands_on_tree__all_given_is_next_and_none_given_is_prev` (property).

Replay: `replay__final_tree_carries_the_manifest`,
`status__reports_a_stale_manifest`.

Reader: `follow__refuses_a_repo_without_a_manifest`,
`follow__refuses_a_repo_not_generated_by_bower`,
`follow__start_refuses_a_dirty_tree`,
`follow__next_refuses_to_overwrite_a_reader_change`,
`follow__next_force_overwrites_and_names_the_files`,
`follow__canonical_tree_equals_the_verified_tree`,
`follow__check_on_an_exercise_judges_the_answer_not_the_exercise`,
`follow__hands_on_on_a_mixed_step_is_refused_by_name`,
`follow__hands_on_reports_a_trivial_step`,
`follow__green_attempt_is_accepted_and_diff_shows_the_readers_version`,
`follow__stale_tags_are_detected_not_fetched`,
`follow__hidden_solution_needs_reveal`,
`manifest__exercise_agrees_with_epic13_trailers`,
`follow__never_invokes_push_fetch_or_commit` (`FakeGit` records every call).

Slow lane (`#[ignore]`): start at `test-that-fails`; `check` fails as the
book claims; apply the fix; `check` passes; `next` lands on 012 with the
reader's tree accepted.

## Key Files

| File | Role |
|---|---|
| `bower-core/src/plan.rs` | `Touch`, `PlannedStep::touches` |
| `bower-core/src/verdict.rs` | new; the § 6 matrix and `judge` |
| `bower-core/src/follow.rs` | new; manifest text and parse, `classify`, `hands_on_tree` |
| `bower-core/src/lib.rs` | modules, prelude |
| `bower/src/verify.rs` | calls `judge`; `--hands-on` report |
| `bower/src/render.rs` | `reader_command` from the kernel |
| `bower/src/trailers.rs` | `book_url` from the kernel |
| `bower/src/replay.rs` | `.bower/steps` in `final_blobs` |
| `bower/src/config.rs` | `[repos.X.follow] given`, defaults |
| `bower/src/main.rs` | `verify --hands-on`; `follow` dispatch before config |
| `bower-follow/` | new crate: `Git`, state, six verbs |
| `bower-testkit/src/{fixtures,generators}.rs` | final paths, round-trip property |
| `Makefile` | `follow-light` |

## Reuse (do NOT recreate)

- `verify.rs:232` `verdict_for` — moved, not copied. Two matrices would be
  two answers to "did the claim hold".
- `render.rs:396` `reader_command` — the box and `check` name one command.
- `replay.rs:330` `final_blobs` — the one place final-tree files are
  decided. `status` follows it for free.
- `trailers.rs:65` `book_named_in` — the push gate's marker test is also
  `start`'s.
- `plan.rs:108` `PlannedStep::tag` — the manifest writes tags; it never
  re-derives them.
- `plan.rs:71` `Exercise` — `follow` reads `answer`; it never computes one.
- `verify.rs:289` — split on whitespace, no shell. The reader's `check` must
  run what `verify` ran.
- `determinism.rs` — its build-into-temp pattern is the follow tests'
  fixture.

## Compatibility

- **Preserves** every tag, trailer, and step tree; `STEPS.md` byte for byte;
  every `bower` subcommand's behaviour; the push gate.
- **Changes** the final commit of every generated repo once, since it gains
  `.bower/steps`. Every earlier SHA is untouched.
- **Adds** one config table, one kernel module and one moved one, one
  `verify` flag, one crate with one binary, and one make target.
- **Breaks** nothing in kernel purity: `cargo tree -p bower-core -e normal`
  still prints one line.

## Dependencies

- **Built on:** EPIC-13 (exercises: `exercise`/`solution`, exercise verified
  `test_fail`, solution `pass`). `--hands-on` generalises it. `check` judges
  `Exercise::answer` as EPIC-13 binds it and uses EPIC-13's named-tests
  verdict. `next` honours `reveal="never"`. EPIC-13 Decision 10 names
  `follow check` as "the only place a reader's answer is ever judged", and
  this EPIC accepts that role. Also EPIC-01 (tags,
  trailers, `STEPS.md`), EPIC-02 (the matrix), and EPIC-04 (`status`).
- **Coordinates with:** EPIC-09 (branches). `Touch` is shared, and `line=`
  and `parents` decide what `next` means. `follow` ignores branch refs and
  `PULLS.md`. EPIC-16 (editions): an edition reader's manifest comes from
  the edition's ref, not `main` (open question 4).
- **Related:** EPIC-11 (captured diagnostics). Once stderr is recorded,
  `check` can say "fails for the stated reason", but v1 matches `expect`
  only. EPIC-15 (scrubber) is the browser's `follow` and should read the
  same per-step file lists. EPIC-12 (back matter): its failure index is a
  list of good `start --from` points.
- **Defers:** C7 reader signals, and C3 classroom, which consumes Phase 4d.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
make purity
cargo tree -p bower-follow -e normal        # bower-follow, then bower-core; nothing else
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hp
cargo run -p bower -- --book books/hello-playbook status -o /tmp/hp
cargo run -p bower -- --book books/hello-playbook verify --hands-on
cargo build -p bower-follow
(cd /tmp/hp && "$OLDPWD/target/debug/bower-follow" start --from test-that-fails \
            && "$OLDPWD/target/debug/bower-follow" check)   # exits 1: the exercise is open
make slow
```

Exit criteria:

1. In a fresh build of the sample book, `bower-follow start --from
   test-that-fails && bower-follow check` exits 1 and names the answer step.
   After the fix it exits 0, and `next` lands on 012.
2. `follow check` and `bower verify --step <id>` give the same verdict on
   every canonical step of the sample book.
3. `next --hands-on` from 011 writes nothing (012 is all *yours*), and
   `check` fails until the reader writes `greet`.
4. Deleting `bower.toml` and the book source changes nothing about what
   `bower-follow` can do in the generated repo.
5. Two builds give identical SHAs, and only the final commit differs from
   the pre-EPIC build.
6. `cargo tree -p bower-follow -e normal` lists two crates;
   `cargo tree -p bower-core -e normal` lists one.
7. `FakeGit` records no push, fetch, or commit across the whole test suite.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Region-granular hands-on.** A *mixed* step (a test region and an impl region of one file, in one step) is refused today. The fix is for the kernel to emit a hands-on tree per step, which replay would have to ship, because `follow` cannot rebuild it from two tags. The candidates are a ref namespace (not fetched by a plain clone) or blobs in the manifest. Wait for a real chapter that needs it, or split such steps by authoring rule? |
| 2 | **Should `verify --hands-on` gate?** A *trivial* step is a hands-on step that teaches nothing. It is an authoring smell, not a false claim. Report only (the lean), or fail steps the author has marked as exercises? |
| 3 | **How does a fork's CI know the step?** The state file stays in `.git`. The options are `check --at <step>` in the workflow, a step recorded in the `follow` branch name, or a committed marker. Each one gives the reader a file or a convention to keep. |
| 4 | **Editions (EPIC-16).** An edition reader's tags are the edition's. Should the manifest come from `main` or from the edition ref, and does `start` take `--edition`? Decide when EPIC-16 names the ref. |
| 5 | **`bower follow` or `bower-follow`?** Part F Q6, `docs/bower-ideas.md:844`. The lean: ship `bower-follow`, and let `bower follow` exec it (4c) for authors who already have `bower`. Is the alias worth the `main.rs` reordering? |
| 6 | **Toolchain in the verdict.** "Canonical tree, claim fails" means your toolchain differs from the book's, but `follow` cannot name the book's toolchain unless `verify` records one. That belongs with devenv (spec § 16) and editions, not here. |
| 7 | **C7, stated.** `bower-follow` has no network code. If reader signals ever ship, do they arrive as a separate opt-in binary so this one can keep saying that? |

---

*Grounded against `ImperialBower/bower` @ HEAD `4ad21bf` ("docs: file the
forge design and add it to the backlog"), 10 September 2026, and a fresh
`bower build` of `books/hello-playbook` (21 commits, 26 tags; `STEPS.md` and
trailers read back with `git log` and `git tag`). Drafted from idea A5,
`docs/bower-ideas.md:234`–`273`.*
