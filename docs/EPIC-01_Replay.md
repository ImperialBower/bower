# EPIC-01: Replay — book plan to git history (RPL)

## Context

Phase 1 shipped. `bower-core` is a pure kernel: it parses `<!-- bower … -->`
directives, groups blocks into steps, orders them, and folds each step into a
complete `TreeState`. Its entry point is `plan()` at `bower-core/src/plan.rs:82`,
returning a `BookPlan` (`bower-core/src/plan.rs:15`) of `RepoPlan`s
(`bower-core/src/plan.rs:28`) of `PlannedStep`s (`bower-core/src/plan.rs:37`).
Each `PlannedStep` already carries everything a commit needs: `seq`, `id`, `msg`,
`expect`, `anchor` (chapter + line), `files`, and the materialized `tree`.
`PlannedStep::tag()` (`bower-core/src/plan.rs:67`) renders the stable tag name.
`lock_text()` (`bower-core/src/plan.rs:216`) renders `bower.lock` as a `String`.

`bower-testkit` proves the kernel against a real book. `books/hello-playbook/`
is a six-chapter mdBook that generates a twenty-step Rust repository;
`fixtures::hello_playbook()` (`bower-testkit/src/fixtures.rs:45`) loads it with
`include_str!`, and `bower-testkit/tests/sample_book.rs` asserts the tags, the
final tree, the goldens, and that planning twice is byte-identical
(`bower-testkit/tests/sample_book.rs:170`). Grounded at commit `90fe086`,
1 September 2026.

**Nothing turns a `BookPlan` into a repository.** The workspace has two members,
`bower-core` and `bower-testkit` (`Cargo.toml:3`). There is no binary crate, no
configuration parser, and no code anywhere that reads a book off disk. The
`RepoSpec` the kernel accepts carries exactly one field, `keep_region_markers`
(`bower-core/src/source.rs:101`); every other setting in spec § 7 — `epoch`,
`identity`, `site`, `github`, `template`, `verify` — has no home yet.

**This EPIC does not** run any verification. `expect="test_fail"` and
`expect="compile_fail"` are *recorded* by the kernel (`bower-core/src/directive.rs:92`)
and this EPIC writes them into trailers and `bower.lock`, but nothing executes
`cargo` to confirm them — that is spec § 11 Phase 3. It also does not push to
GitHub, does not build the mdBook preprocessor, and does not touch the publishing
targets. **`bower-core` gains no I/O and no new dependency**; its purity contract
(`bower-core/src/lib.rs:11`) is preserved exactly.

---

## Status

| Component | Status |
|---|---|
| `bower` binary crate + workspace member | **Complete** |
| `BookConfig` — `bower.toml` parser | Planned |
| `BookLoader` — disk → `BookSource` | Planned |
| `bower plan` + `bower.lock` on disk | Planned |
| `Replayer` — empty tree → commits | Planned |
| Deterministic identity and timestamps | Planned |
| Annotated step tags + chapter-end tags | Planned |
| Commit trailers | Planned |
| `STEPS.md` generation | Planned |
| Step-0 template scaffolding | Planned |
| Byte-identical-SHA golden test | Planned |

---

## Goals

- Give the book a **replay layer**: one command rebuilds a target **repository**
  from scratch, one **commit** per teaching step.
- Make regeneration **byte-identical**. Same book in, same **SHAs** out, every
  time, on every machine.
- Make the links real in both directions: every commit carries **trailers**
  naming the chapter that produced it; every step carries an annotated **tag**
  the book can link to.
- Keep the **kernel pure**. All filesystem, git, and configuration concerns live
  in the new binary crate.
- Prove it against **`hello-playbook`**, the book that already exists, rather
  than a fixture written to flatter the implementation.

## Scope

The rules this EPIC must obey, from spec § 5 and § 7:

- Replay **always starts from an empty tree** and replays the whole plan.
  Incremental regeneration is a non-goal.
- Author and committer are **fixed identities** read from `bower.toml`.
- Timestamps are **synthetic**: `book_epoch + seq × 60s`. No wall clock is ever
  consulted during replay.
- Commit subject is `msg` when the directive gave one, else the kernel's derived
  subject — already resolved into `PlannedStep::msg`.
- Every commit carries `Book-Source`, `Book-Url`, `Bower-Step`, and
  `Generated-By` trailers.
- Every step gets an **annotated** tag `step-NNN-<id>`; every chapter boundary
  gets `<chapter-stem>-end`.
- A generated `STEPS.md` at the repo root lists every step with its book link.
- Repo-level boilerplate comes from the configured `template` directory, applied
  as **step 0**, "Initial commit — scaffolding".
- `bower plan` writes `bower.lock`; it is generated, reviewed, and never
  hand-edited.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Book (an mdBook on disk) | `BookLoader` → `BookSource` | ❌ absent |
| Book configuration | `BookConfig` from `bower.toml` | ❌ absent |
| Plan (pure value) | `BookPlan` `bower-core/src/plan.rs:15` | ✅ done |
| Step (a commit-to-be) | `PlannedStep` `bower-core/src/plan.rs:37` | ✅ done |
| Tree state after a step | `TreeState` `bower-core/src/tree.rs:24` | ✅ done |
| File content, text or bytes | `FileBody` `bower-core/src/tree.rs:15` | ✅ done |
| Tag name | `PlannedStep::tag()` `bower-core/src/plan.rs:67` | ✅ done |
| Lockfile text | `lock_text()` `bower-core/src/plan.rs:216` | ✅ done |
| Replay (plan → git) | `Replayer` | ❌ absent |
| Commit identity + epoch | `Identity`, `Epoch` | ❌ absent |
| Repo index back into the book | `steps_md()` | ❌ absent |

---

## Design

### Crate layout

`bower/` (new workspace member, added to `Cargo.toml:3`):

```
bower/
  Cargo.toml
  src/
    main.rs        # clap CLI: plan | build
    config.rs      # BookConfig, bower.toml
    loader.rs      # BookLoader, SUMMARY.md → BookSource
    replay.rs      # Replayer, BookPlan → git
    trailers.rs    # commit message assembly, STEPS.md
```

The domain-kernel split from spec § 8 holds: `bower-core` stays pure, `bower` owns
every side effect. `bower` depends on `bower-core`; nothing depends on `bower`.

**Git backend: `gix`.** Spec § 12 Q1 left this open. `gix` is pure Rust, so the
supply-chain posture this project preaches — one `bin/security-scan`, a tight
`deny.toml` licence allow-list — does not immediately acquire a C library.
Deterministic commits need direct control over author time, committer time, and
tree construction, which is `gix`'s strength rather than a corner of its API. If
annotated-tag creation or tree writing proves unworkable, the fallback is `git2`,
and the `Replayer` trait boundary below is what makes that swap cheap.

### `BookConfig`

`bower/src/config.rs` (new):

```rust
/// The parsed `bower.toml`. Every setting the replay layer needs, and
/// nothing the kernel does.
pub struct BookConfig {
    pub epoch: OffsetDateTime,
    pub site: Option<String>,
    pub identity: Identity,
    pub repos: BTreeMap<String, RepoConfig>,
}

pub struct Identity {
    pub name: String,
    pub email: String,
}

pub struct RepoConfig {
    pub github: Option<String>,
    pub template: Option<PathBuf>,
    pub verify: Option<String>,
    pub keep_region_markers: bool,
    pub links: LinkTemplates,
}

pub struct LinkTemplates {
    /// e.g. "https://…/blob/{tag}/{path}#L{start}-L{end}"
    pub blob: Option<String>,
}

impl BookConfig {
    pub fn load(book_root: &Path) -> Result<Self, ConfigError>;
    /// The catalog the kernel wants — the narrow projection of this config.
    pub fn catalog(&self) -> RepoCatalog;
}
```

`catalog()` is the whole point of the split. The kernel's `RepoCatalog`
(`bower-core/src/source.rs:109`) takes only what changes pure computation; every
other field stays on this side of the boundary. Adding a setting to `bower.toml`
must never require touching `bower-core`.

### `BookLoader`

`bower/src/loader.rs` (new):

```rust
pub struct BookLoader {
    root: PathBuf,
}

impl BookLoader {
    /// Read `src/SUMMARY.md`, resolve chapter links in reading order, and
    /// read each chapter. Chapter paths are stored book-root-relative
    /// (`src/ch01-a-repo-that-builds.md`), because that is what the
    /// `Book-Source` trailer must print.
    pub fn load(&self) -> Result<BookSource, LoadError>;
}
```

Reading order is `SUMMARY.md` order, which is what `BookSource` documents itself
as expecting (`bower-core/src/source.rs:7`) and what makes document order the
default step order.

**One convention must be settled here.** `fixtures::hello_playbook()`
(`bower-testkit/src/fixtures.rs:45`) names chapters bare — `ch01-a-repo-that-builds.md`
— while the loader will name them `src/ch01-a-repo-that-builds.md`. Those two
produce different `Location` values and therefore different `Book-Source`
trailers. The loader's form is correct, because a trailer must be a path someone
can open. Work item 2c updates the fixture to match.

### `Replayer`

`bower/src/replay.rs` (new):

```rust
pub struct Replayer<'a> {
    config: &'a BookConfig,
    repo_name: &'a str,
    out_dir: &'a Path,
}

pub struct ReplayReport {
    pub repo: String,
    pub head: ObjectId,
    pub commits: usize,
    pub tags: Vec<String>,
}

impl<'a> Replayer<'a> {
    /// Initialize an empty repository at `out_dir`, apply the template as
    /// step 0, then commit every step in order. Existing contents are
    /// removed first: replay is always from scratch (spec § 5.1).
    pub fn run(&self, plan: &RepoPlan) -> Result<ReplayReport, ReplayError>;
}
```

`run` takes a `&RepoPlan`, not a `&BookPlan`, so a multi-repo book replays as a
loop of independent, order-free operations.

### Identity and timestamps

```rust
/// Commit N's time. Deterministic by construction: no wall clock, no
/// timezone, no environment.
fn step_time(epoch: OffsetDateTime, seq: usize) -> OffsetDateTime {
    epoch + Duration::minutes(seq as i64)
}
```

Step 0 (scaffolding) takes `seq = 0`; the kernel's steps are 1-based
(`bower-core/src/plan.rs:39`), so they fall on minutes 1..=N with no collision.
Author time and committer time are the same value, and `gix` must be given both
explicitly — a default committer time from the clock is the single most likely
way to lose byte-identical SHAs.

### Trailers and `STEPS.md`

`bower/src/trailers.rs` (new):

```rust
/// The full commit message: subject, blank line, trailers.
pub fn commit_message(
    step: &PlannedStep,
    book_name: &str,
    site: Option<&str>,
    repo_name: &str,
) -> String;

/// The repo's own table of contents back into the book.
pub fn steps_md(plan: &RepoPlan, book_name: &str, site: Option<&str>) -> String;
```

`Book-Source` is built from `step.anchor.chapter` and the step id;
`Book-Url` needs `site` and is omitted when the book declares none, rather than
emitting a broken link. `Bower-Step` is `<repo>/<seq:03>`. `Generated-By` carries
this crate's version.

`STEPS.md` is written into the tree of the **final** step only. Writing it at
every step would make each commit's tree depend on every later step, which
destroys the property that a step's tree is a pure function of the steps before
it.

---

## Work Items

### Phase 0 — The crate exists

- [x] **0a.** Create `bower/Cargo.toml` and add `"bower"` to the members list at
  `Cargo.toml:3`. **Deviation:** only `bower-core` and `clap` were added.
  `toml`/`serde` land with Phase 1 and `gix`/`time` with Phase 3 — a dependency
  arrives in the phase that uses it, so an abandoned phase leaves no dead weight
  in the lockfile.
- [x] **0b.** `bower/src/main.rs` with a `clap` skeleton for `plan` and `build`.
  Both print the phase that will implement them and exit `ExitCode::FAILURE`, so
  no script can mistake the skeleton for a working tool.
- [x] **0c.** `cargo build --workspace` and `cargo clippy --workspace --all-targets`
  are green (0 warnings), `cargo test --workspace` is 80 passed, and
  `cargo tree -p bower-core -e normal` prints the crate and nothing else.
  Landed on branch `docs/epic-01`, 1 September 2026.

### Phase 1 — Configuration

- [ ] **1a.** `bower/src/config.rs`: `BookConfig`, `Identity`, `RepoConfig`,
  `LinkTemplates`, and `BookConfig::load`.
- [ ] **1b.** `BookConfig::catalog()` projecting to `RepoCatalog`
  (`bower-core/src/source.rs:109`), carrying `keep_region_markers` only.
- [ ] **1c.** Extend `books/hello-playbook/bower.toml` with the `[book]` epoch
  and site and the `[identity]` block it currently lacks.
- [ ] **1d.** Tests: `config__loads_the_sample_book`,
  `config__missing_epoch_is_an_error`,
  `config__catalog_carries_only_kernel_settings`.

### Phase 2 — Loading and `bower plan`

- [ ] **2a.** `bower/src/loader.rs`: parse `src/SUMMARY.md`, resolve chapter
  links in order, read each chapter, produce a `BookSource`.
- [ ] **2b.** `bower plan [--repo R]` — load, call `plan()`
  (`bower-core/src/plan.rs:82`), print the step table, write `bower.lock` from
  `lock_text()` (`bower-core/src/plan.rs:216`).
- [ ] **2c.** Change `fixtures::hello_playbook()`
  (`bower-testkit/src/fixtures.rs:45`) to name chapters `src/<file>.md`, matching
  the loader. Update the `Book-Source` expectations that depend on it.
- [ ] **2d.** Test: `loader__reads_hello_playbook_in_summary_order` asserts the
  loader and the fixture produce **equal** `BookSource` values. This is the test
  that stops the two from drifting.

### Phase 3 — Replay

- [ ] **3a.** `bower/src/replay.rs`: init an empty repo, write a `TreeState`
  (`bower-core/src/tree.rs:24`) to disk, honouring both `FileBody` variants
  (`bower-core/src/tree.rs:15`).
- [ ] **3b.** Apply the configured `template` directory as step 0,
  "Initial commit — scaffolding".
- [ ] **3c.** Commit each step with `step_time(epoch, seq)` for both author and
  committer time, and the fixed identity.
- [ ] **3d.** Create an annotated tag per step from `PlannedStep::tag()`
  (`bower-core/src/plan.rs:67`), plus a `<chapter-stem>-end` tag at each chapter
  boundary, derived from `step.anchor.chapter`.
- [ ] **3e.** `bower build [--repo R] [-o DIR]` wires it together.

### Phase 4 — The links back

- [ ] **4a.** `bower/src/trailers.rs`: `commit_message()` emitting all four
  trailers, omitting `Book-Url` when no site is configured.
- [ ] **4b.** `steps_md()`, written into the final step's tree only.
- [ ] **4c.** Tests: `trailers__name_the_chapter_and_line`,
  `trailers__book_url_is_omitted_without_a_site`.

### Phase 5 — The golden

- [ ] **5a.** `bower/tests/determinism.rs`: replay `hello-playbook` into two
  temporary directories and assert **identical HEAD SHAs**.
- [ ] **5b.** Assert the twenty tags match `EXPECTED_TAGS`
  (`bower-testkit/tests/sample_book.rs:13`), and that the working tree at
  `step-020-drop-scratch` matches `FINAL_PATHS`
  (`bower-testkit/tests/sample_book.rs:79`).
- [ ] **5c.** Assert replaying over a **dirty** output directory still yields the
  same SHAs — proving replay really does start from empty.

### Phase 6 — Documentation

- [ ] **6a.** Update `README.md` — the workspace is no longer Phase 1 only.
- [ ] **6b.** Record the `gix` decision against spec § 12 Q1 in `bower-spec.md`.
- [ ] **6c.** Flip this EPIC's Status rows and append the corrigendum.

---

## Test Plan

- `config__loads_the_sample_book` — `bower.toml` round-trips into `BookConfig`.
- `config__missing_epoch_is_an_error` — a book with no epoch cannot replay
  deterministically, so it must fail loudly at load, not silently at commit.
- `config__catalog_carries_only_kernel_settings` — pins the purity boundary: the
  projection drops `github`, `template`, `verify`, and `links`.
- `loader__reads_hello_playbook_in_summary_order` — the loader's `BookSource`
  equals the testkit fixture's. Pins the two representations together forever.
- `trailers__name_the_chapter_and_line` — `Book-Source` points at a real file.
- `trailers__book_url_is_omitted_without_a_site` — no broken links.
- `replay__hello_playbook_is_byte_identical_across_runs` — the headline
  requirement of spec § 5.1.
- `replay__starts_from_empty_even_over_a_dirty_directory` — the no-incremental
  rule, tested rather than assumed.
- `replay__tags_match_the_planned_tags` — tags are the book's only stable link
  target, so a drifted tag is a broken book.

## Key Files

| File | Role |
|---|---|
| `Cargo.toml:3` | add `bower` to `members` |
| `bower/src/main.rs` | CLI entry: `plan`, `build` |
| `bower/src/config.rs` | `bower.toml` → `BookConfig`, and the `RepoCatalog` projection |
| `bower/src/loader.rs` | `SUMMARY.md` + chapters → `BookSource` |
| `bower/src/replay.rs` | `RepoPlan` → git commits and tags |
| `bower/src/trailers.rs` | commit messages and `STEPS.md` |
| `books/hello-playbook/bower.toml` | gains `[book]` and `[identity]` |
| `bower-testkit/src/fixtures.rs:45` | chapter paths change to `src/…` |

## Reuse (do NOT recreate)

- `bower-core/src/plan.rs:82` — `plan()`. Ordering, grouping, and folding are
  solved. Do not re-derive step order in the CLI.
- `bower-core/src/plan.rs:67` — `PlannedStep::tag()`. Tag naming lives here;
  the replayer formats nothing.
- `bower-core/src/plan.rs:216` — `lock_text()`. `bower.lock`'s content is the
  kernel's business; the CLI only writes the bytes to disk.
- `bower-core/src/tree.rs:55` — `materialized()`. Already applied by `plan()`;
  `step.tree` is the tree to write, markers stripped.
- `bower-core/src/tree.rs:15` — `FileBody`. Binary assets are already modelled;
  do not add a second byte path.
- `bower-testkit/tests/sample_book.rs:13,79` — `EXPECTED_TAGS` and `FINAL_PATHS`.
  The golden test asserts against these, not against a fresh list.

## Compatibility

- **Preserves** every `bower-core` public signature and its zero-dependency,
  no-I/O contract (`bower-core/src/lib.rs:11`). Verified by
  `cargo tree -p bower-core -e normal` — plain `cargo tree` also prints
  dev-dependencies, which would mask a real regression.
- **Adds** a new binary crate and one changed chapter-path convention in
  `bower-testkit` (work item 2c), which is internal to the workspace.
- **Breaks** nothing published. Neither crate has been released.

## Dependencies

- **Blocks:** the verification EPIC (spec § 11 Phase 3 — the `expect` matrix),
  and the surfaces EPIC (spec § 11 Phase 4 — `mdbook-bower`, `push`, `status`).
  Both consume a real generated repository, which does not exist until this ships.
- **Built on:** the Phase 1 kernel, which predates this EPIC series and has no
  number of its own; and `books/hello-playbook/`, landed at `90fe086`.
- **Related:** `docs/superpowers/specs/2026-08-31-hello-playbook-book-design.md`,
  whose § 9 lists the two gaps this EPIC closes one of. Gap 9.2 — `expect` is
  recorded, not verified — remains open after this EPIC and is Phase 3's job.

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo tree -p bower-core -e normal  # must print the crate and nothing else
cargo run -p bower --  plan  --repo hello-playbook
cargo run -p bower --  build --repo hello-playbook -o /tmp/hp-1
cargo run -p bower --  build --repo hello-playbook -o /tmp/hp-2
diff <(git -C /tmp/hp-1 log --format=%H) <(git -C /tmp/hp-2 log --format=%H)
git -C /tmp/hp-1 tag | wc -l        # 20 step tags + 6 chapter-end tags
```

Exit criteria:

1. Two independent replays of an unchanged `hello-playbook` produce identical
   commit SHAs, and the `diff` above is empty.
2. The generated repository holds twenty commits plus a scaffolding commit, and
   the ten files listed at `bower-testkit/tests/sample_book.rs:79` at HEAD.
3. Every commit carries `Book-Source` and `Bower-Step` trailers pointing at a
   chapter file that exists.
4. `cargo tree -p bower-core -e normal` still lists no dependencies — the kernel did not
   acquire I/O to make replay convenient.
5. `bower-testkit` and the CLI loader produce equal `BookSource` values for the
   sample book.
