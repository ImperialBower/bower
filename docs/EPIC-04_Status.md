# EPIC-04: `bower status` — the drift report (STA)

## Context

Three EPICs shipped. `bower plan` resolves a book and writes `bower.lock`
(`bower/src/main.rs:142`); `bower build` replays that plan into a git
repository; `bower verify` checks each step's declared `expect` against a real
compiler; `mdbook-bower` rewrites chapters at render time.

**Nothing answers the question a person actually asks between those commands:
is any of it still true?** A book edited after `bower plan` leaves a stale
`bower.lock` with no sign it is stale. A repository built before a chapter
changed still sits on disk looking finished. The only way to find out today is
to re-run everything and read the output, which is exactly the manual diffing
this project exists to replace.

The materials are all present. `lock_text()` (`bower-core/src/plan.rs:216`)
renders a lock from a plan deterministically, so a *regenerated* lock and the
one on disk can simply be compared. `materialize::blobs_of`
(`bower/src/materialize.rs:26`) and `read_dir_recursive`
(`bower/src/materialize.rs:44`) already turn a step's tree and a directory into
the same shape, which is what a worktree comparison needs. `trailers::steps_md`
(`bower/src/trailers.rs:50`) is the one file a built repo holds that the
kernel's tree does not, and `PlannedStep::tag()` names every tag that should
exist.

**This EPIC does not** talk to a network or a forge — comparing against a
GitHub remote is `bower push`'s business, not this command's. It does not
*repair* drift; reporting is the whole job, and the fix is always to re-run
`plan` or `build`. It does not compare commit SHAs, for the reason in Design
below. And it does not touch `bower-core`.

---

## Status

| Component | Status |
|---|---|
| `LockDrift` — regenerated vs on-disk lock | **Complete** |
| `RepoDrift` — tags and worktree | Planned |
| `StatusReport` and its verdict | Planned |
| `bower status` CLI + exit codes | Planned |
| Goldens: clean, stale lock, stale repo | Planned |

---

## Goals

- Answer one question in one command: **what here is out of date?**
- Name the drift precisely — which **steps**, which **tags**, which **files** —
  so the answer is actionable rather than a red light.
- Be **honest about absence**. A book never planned and a repo never built are
  normal states, not errors.
- Exit **non-zero on drift**, so `bower status` works in CI as a rot alarm.
- Compute everything from what already exists. **No new kernel code.**

## Scope

`bower status [--repo R] [-o DIR]` compares three things:

| Compared | How |
|---|---|
| **book ⇄ lock** | Resolve the book, render `lock_text()`, compare line-by-line with `bower.lock` on disk |
| **book ⇄ repo tags** | Every `step-NNN-<id>` tag the plan expects, against the tags the repo has |
| **book ⇄ repo worktree** | The final step's blobs plus `STEPS.md`, against the files on disk |

- No `bower.lock` → reported as **never planned**, not an error.
- No repo at `DIR`, or a `DIR` with no `.git` → **never built**, not an error.
- Exit `0` when everything compared is in sync; `1` when anything drifted.
  Absent artifacts do not by themselves mean drift — a book that has never been
  built is not out of date, it is unbuilt. This distinction is the whole reason
  the command is worth having in CI.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| The plan, now | `plan()` `bower-core/src/plan.rs:82` | ✅ done |
| The lock, as text | `lock_text()` `bower-core/src/plan.rs:216` | ✅ done |
| A tree, as files | `blobs_of` `bower/src/materialize.rs:26` | ✅ done |
| A directory, as files | `read_dir_recursive` `bower/src/materialize.rs:44` | ✅ done |
| The repo's extra file | `steps_md` `bower/src/trailers.rs:50` | ✅ done |
| Lock drift | `LockDrift` | ✅ done |
| Repo drift | `RepoDrift` | ❌ absent |
| The verdict | `StatusReport` | ❌ absent |

---

## Design

### Why not compare commit SHAs

Replay is deterministic, so the SHAs a book *should* produce are knowable — but
only by replaying it, which means writing a whole repository to disk to answer a
question about the one already there. Comparing **tags and file contents**
answers the same question without building anything, and gives a better report
when it fails: "`src/lib.rs` differs" is actionable, and "HEAD is `a1b2c3d`, not
`e4f5a6b`" is not.

### `LockDrift`

`bower/src/status.rs` (new):

```rust
/// The lock on disk against the lock this book would produce now.
pub enum LockDrift {
    /// No `bower.lock`. Normal for a book nobody has planned yet.
    NeverPlanned,
    InSync,
    /// Line-level differences, in file order, capped for display.
    Stale { added: Vec<String>, removed: Vec<String> },
}

pub fn lock_drift(book_root: &Path, plan: &BookPlan) -> Result<LockDrift, StatusError>;
```

Comparing rendered text rather than parsing the lock is deliberate: `lock_text`
is the only definition of that format, and a parser would be a second one, free
to disagree. The cost is that a formatting change to `lock_text` reads as drift
in every book until each is re-planned — which is correct, since the file on
disk really is not what the tool would write now.

### `RepoDrift`

```rust
pub enum RepoDrift {
    /// No directory, or a directory with no `.git`.
    NeverBuilt,
    InSync,
    Stale {
        missing_tags: Vec<String>,
        unexpected_tags: Vec<String>,
        differing_files: Vec<String>,
        missing_files: Vec<String>,
        unexpected_files: Vec<String>,
    },
}

pub fn repo_drift(dir: &Path, plan: &RepoPlan, expected: &Blobs) -> Result<RepoDrift, StatusError>;
```

`expected` is passed in rather than computed here, because the caller already
builds the final blobs to write `STEPS.md` and there is no reason to do it
twice.

Tags are read from `.git/refs/tags` as loose refs rather than through `gix`, the
same choice `bower/tests/determinism.rs` made and for the same reason: a drift
report that shares a library with the thing it inspects can share a bug with it.
Packed refs are the one case this misses; a freshly replayed repository has none,
and the report says so rather than silently passing.

### `StatusReport`

```rust
pub struct StatusReport {
    pub repo: String,
    pub steps: usize,
    pub lock: LockDrift,
    pub repo_state: RepoDrift,
}

impl StatusReport {
    /// True when something compared is out of date. Absent artifacts are not
    /// drift — unplanned and unbuilt are honest states, not failures.
    pub fn has_drift(&self) -> bool;
}
```

Output shape:

```text
hello-playbook — 20 steps
  lock      in sync
  repo      /tmp/hp — 26 tags, worktree matches

hello-playbook — 20 steps
  lock      STALE — 3 lines differ; re-run `bower plan`
              - 011 test-that-fails expect=test_fail …
              + 011 test-that-fails expect=pass …
  repo      STALE — 1 missing tag, 2 files differ
              missing tag  step-021-new-thing
              differs      src/lib.rs, Makefile
```

---

## Work Items

### Phase 0 — Lock drift

- [x] **0a.** `bower/src/status.rs`: `StatusError`, `LockDrift`, `lock_drift()`.
  **Deviation:** the design named the drift fields `added` and `removed`, which
  do not say which side they belong to. They are `only_in_lock` (stale lines on
  disk) and `only_in_book` (what a fresh plan would write). The comparison is a
  line-set difference rather than a positional diff, because the lock is one
  line per step and a step that moved already shows it in its own `NNN` prefix.
- [x] **0b.** Registered in `bower/src/lib.rs`. Worth noting that the EPIC-03
  library restructure paid off here: `lock_drift` is public API of a library, so
  it does not warn as dead code while waiting for its Phase 2 caller — the
  problem that forced Phases 1–3 of EPIC-02 to be carried through in one pass.
- [x] **0c.** Four tests. **Addition:**
  `lock__an_unreadable_lock_is_an_error_not_a_verdict` — a directory where the
  lock should be. Reporting "in sync" there would be a lie, and "never planned"
  would hide a real problem. Verified on branch `docs/epic-01`, 1 September
  2026: 140 tests pass, clippy silent.

### Phase 1 — Repo drift

- [ ] **1a.** `RepoDrift`, `repo_drift()`: read loose tags from
  `.git/refs/tags`, compare against `PlannedStep::tag()` for every step plus the
  `<chapter>-end` tags, and compare the worktree against the expected blobs.
- [ ] **1b.** Report a repository whose tags are packed rather than loose as
  its own state, not as "no tags" — a wrong answer is worse than an admission.
- [ ] **1c.** Tests: `repo__missing_directory_is_never_built`,
  `repo__directory_without_git_is_never_built`,
  `repo__missing_tag_is_named`, `repo__changed_file_is_named`,
  `repo__unexpected_file_is_named`.

### Phase 2 — The command

- [ ] **2a.** `StatusReport`, `has_drift()`, and the printer.
- [ ] **2b.** `bower status [--repo R] [-o DIR]` in `bower/src/main.rs`,
  reusing `resolve()`; exit `1` on drift, `0` otherwise.
- [ ] **2c.** Tests: `report__absent_artifacts_are_not_drift`,
  `report__a_stale_lock_is_drift`.

### Phase 3 — Goldens

- [ ] **3a.** `bower/tests/status.rs`: `plan` then `build` then `status` — in
  sync, exit `0`.
- [ ] **3b.** Touch a chapter in a copied book, re-`status`: the lock is stale
  and the offending step is named. Exit `1`.
- [ ] **3c.** Delete a file from a built repo, re-`status`: the file is named
  as missing. Exit `1`.
- [ ] **3d.** A book never planned and never built reports both, and still
  exits `0`.

### Phase 4 — Documentation

- [ ] **4a.** `README.md`: add `bower status` to the commands.
- [ ] **4b.** Update `BACKLOG.md` — `bower status` moves to completed.
- [ ] **4c.** Flip this EPIC's Status rows and append the corrigendum.

---

## Test Plan

- `lock__absent_is_never_planned` — an unplanned book is not a broken one.
- `lock__identical_text_is_in_sync` — the happy path, which must not be noisy.
- `lock__a_changed_step_shows_both_lines` — a report that says "something
  changed" without saying what is not a report.
- `lock__an_unreadable_lock_is_an_error_not_a_verdict` — an unreadable lock is
  neither "in sync" nor "never planned".
- `repo__missing_directory_is_never_built`,
  `repo__directory_without_git_is_never_built` — the two shapes of unbuilt.
- `repo__missing_tag_is_named`, `repo__changed_file_is_named`,
  `repo__unexpected_file_is_named` — one per drift kind, each naming its thing.
- `report__absent_artifacts_are_not_drift` — the distinction that makes this
  usable in CI. Without it, every fresh checkout is a red build.
- `status__a_freshly_built_book_is_clean` — the golden.
- `status__an_edited_chapter_makes_the_lock_stale` — the golden's negative.
- `status__a_deleted_file_is_reported` — the repo's negative.

## Key Files

| File | Role |
|---|---|
| `bower/src/status.rs` | new; `LockDrift`, `RepoDrift`, `StatusReport` |
| `bower/src/lib.rs` | registers the module |
| `bower/src/main.rs:142` | the `status` subcommand, beside `plan` |
| `bower/tests/status.rs` | new; the goldens |
| `README.md`, `BACKLOG.md` | the command, and the backlog entry it closes |

## Reuse (do NOT recreate)

- `bower-core/src/plan.rs:216` — `lock_text()` is the only definition of the
  lock format. Do not write a parser.
- `bower-core/src/plan.rs:67` — `PlannedStep::tag()` names the tags to expect.
  Never construct a tag name here.
- `bower/src/materialize.rs:26,44` — `blobs_of` and `read_dir_recursive`
  already produce the same `Blobs` shape from a tree and from a directory,
  which is what makes the worktree comparison a one-liner.
- `bower/src/trailers.rs:50` — `steps_md` is the one file a built repo has that
  the kernel's tree does not. Expect it, or every clean repo reports drift.
- `bower/src/main.rs` — `resolve()` already loads and plans a book.

## Compatibility

- **Preserves** every existing command and `bower-core`'s purity.
- **Adds** one subcommand and one module. No new dependency: `gix` is not used
  here at all.
- **Breaks** nothing.

## Dependencies

- **Blocks:** nothing. `bower push` (spec § 5.5) will want a drift check before
  force-pushing, but does not need one to exist.
- **Built on:** EPIC-01 for the lock, the tags, and the materialization rules.
- **Related:** spec § 7 (the CLI surface), § 5.3 (tags are the stable anchor).

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook status            # never built
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hp
cargo run -p bower -- --book books/hello-playbook status -o /tmp/hp # in sync, exit 0
rm /tmp/hp/Makefile
cargo run -p bower -- --book books/hello-playbook status -o /tmp/hp # names it, exit 1
```

Exit criteria:

1. A freshly planned and built book reports everything in sync and exits `0`.
2. Editing one chapter makes the lock stale and names the step that changed.
3. Deleting one file from a built repo names that file and exits `1`.
4. A book never planned and never built says so and still exits `0`.
5. `cargo tree -p bower-core -e normal` still prints one line.
