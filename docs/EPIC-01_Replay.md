# EPIC-01: Replay — book plan to git history (RPL)

## Summary

- **Builds:** the `bower` CLI: `bower plan` and `bower build` turn a book into a
  deterministic git repository, one commit per step.
- **Why:** the kernel produced a `BookPlan` and nothing turned it into a repo.
- **Shape:** `bower-core` stays pure; a new `bower` crate owns config, loading,
  and a `Replayer` with fixed identity and timestamps, so SHAs never move.
- **Proves it:** `bower/tests/determinism.rs` replays the sample book twice and
  asserts identical SHAs, 26 tags, and the final tree.
- **Status:** Shipped, 1 September 2026; verification deferred to EPIC-02.

---

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
| `BookConfig` — `bower.toml` parser | **Complete** |
| `BookLoader` — disk → `BookSource` | **Complete** |
| `bower plan` + `bower.lock` on disk | **Complete** |
| `Replayer` — empty tree → commits | **Complete** |
| Deterministic identity and timestamps | **Complete** |
| Annotated step tags + chapter-end tags | **Complete** |
| Commit trailers | **Complete** |
| `STEPS.md` generation | **Complete** |
| Step-0 template scaffolding | **Complete** |
| Byte-identical-SHA golden test | **Complete** |

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
| Book (an mdBook on disk) | `BookLoader` → `BookSource` | ✅ done |
| Book configuration | `BookConfig` from `bower.toml` | ✅ done |
| Plan (pure value) | `BookPlan` `bower-core/src/plan.rs:15` | ✅ done |
| Step (a commit-to-be) | `PlannedStep` `bower-core/src/plan.rs:37` | ✅ done |
| Tree state after a step | `TreeState` `bower-core/src/tree.rs:24` | ✅ done |
| File content, text or bytes | `FileBody` `bower-core/src/tree.rs:15` | ✅ done |
| Tag name | `PlannedStep::tag()` `bower-core/src/plan.rs:67` | ✅ done |
| Lockfile text | `lock_text()` `bower-core/src/plan.rs:216` | ✅ done |
| Replay (plan → git) | `Replayer` | ✅ done |
| Commit identity + epoch | `Identity` + `Replayer::stamp` | ✅ done |
| Repo index back into the book | `steps_md()` | ✅ done |

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
    /// Parse configuration text. Split from `load` so the rules can be
    /// tested without touching a filesystem.
    pub fn parse(text: &str) -> Result<Self, ConfigError>;
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

- [x] **1a.** `bower/src/config.rs`: `BookConfig`, `Identity`, `RepoConfig`,
  `LinkTemplates`, `ConfigError`, and `BookConfig::load`. **Deviation:** added
  `BookConfig::parse(&str)` beside `load`, because the error rules cannot be
  tested through a path without inventing temporary files.
- [x] **1b.** `BookConfig::catalog()` projecting to `RepoCatalog`
  (`bower-core/src/source.rs:109`), carrying `keep_region_markers` only.
- [x] **1c.** Extended `books/hello-playbook/bower.toml` with `[book]` (epoch,
  site) and `[identity]`, and corrected its header comment, which still claimed
  nothing parsed the file.
- [x] **1d.** Four tests, not three — every wire struct carries
  `#[serde(deny_unknown_fields)]`, so a typo in `bower.toml` fails at load
  rather than being silently ignored, and that rule earned its own test.
  **Deviation:** `time` was scheduled for Phase 3 but landed here; an epoch that
  is not a valid instant has to fail at load, not at commit. Verified on branch
  `docs/epic-01`, 1 September 2026: 84 tests pass, clippy silent,
  `cargo tree -p bower-core -e normal` unchanged.

### Phase 2 — Loading and `bower plan`

- [x] **2a.** `bower/src/loader.rs`: `BookLoader`, `LoadError`, and a
  deliberately small link scanner — `SUMMARY.md` is a list of links by
  definition, so a full markdown parser would be a dependency bought for
  nothing. `.md` matching is case-insensitive via `Path::extension`.
  `BookSource.library` and `.assets` are left empty: no current book uses
  `include=` or `op="copy"`, and inventing a directory convention before a book
  needs one would be guesswork.
- [x] **2b.** `bower plan [--repo R]` loads, resolves through `plan()`
  (`bower-core/src/plan.rs:82`), prints the twenty-row step table, and writes
  `books/hello-playbook/bower.lock` from `lock_text()`
  (`bower-core/src/plan.rs:216`). It is the first subcommand that exits `0`.
- [x] **2c.** `fixtures::hello_playbook()` (`bower-testkit/src/fixtures.rs:45`)
  now names chapters `src/<file>.md`. Nothing in
  `bower-testkit/tests/sample_book.rs` asserted on chapter paths, so no
  expectation changed; the effect is visible in `bower.lock`, whose anchors now
  read `src/ch01-a-repo-that-builds.md:6` rather than a path nobody could open.
- [x] **2d.** Five tests, not one. **Deviation:** `bower` gained
  `bower-testkit` as a **dev**-dependency so `loader__matches_the_testkit_fixture_exactly`
  can compare the two `BookSource` values directly — that is the anti-drift test
  the work item asked for, and it needed both crates in one place. Verified on
  branch `docs/epic-01`, 1 September 2026: 88 tests pass, clippy silent,
  `cargo tree -p bower-core -e normal` unchanged.

### Phase 3 — Replay

- [x] **3a.** `bower/src/replay.rs`: `Replayer`, `ReplayReport`, `ReplayError`.
  Trees are built directly in the object database with `gix`'s tree editor from
  `ObjectId::empty_tree`, not by staging files — nested paths like
  `.github/workflows/ci.yml` come out right and no index churn is involved.
  **Deviation:** the kernel does not model file modes (`FileBody` is text or
  bytes, `bower-core/src/tree.rs:15`), so the executable bit is inferred here
  from a `#!` shebang. That is what makes `bin/security-scan` land as `100755`,
  which the generated `Makefile` needs in order to invoke it.
- [x] **3b.** The configured `template` directory is read recursively and
  committed as step 0, "chore: initial commit — scaffolding", at `seq = 0`.
  Kernel steps are 1-based (`bower-core/src/plan.rs:39`), so there is no
  timestamp collision.
- [x] **3c.** Both author and committer are the same fixed signature, and both
  times are `epoch + seq` minutes formatted as `<unix> +0000`. **Deviation:**
  `gix_actor::SignatureRef::time` is a raw git time *string*, not a typed
  instant, so `time` is used only to add minutes and produce a unix timestamp.
  The offset is pinned at `+0000`; a local one would change every SHA.
- [x] **3d.** Annotated tags — a real tag object with a tagger and the step's
  own timestamp, so tags are deterministic too. Twenty `step-NNN-<id>` tags plus
  six `<chapter-stem>-end` tags.
- [x] **3e.** `bower build [--repo R] [-o DIR]`. **Deviation:** with one repo
  selected `DIR` is used as-is; with several, each gets a subdirectory, because
  two repositories cannot share one working tree. Two further additions the
  design did not specify: `HEAD` is rewritten to `refs/heads/main` because
  `gix::init` otherwise honours the machine's `init.defaultBranch`, and the
  final step is materialized into the working tree with a matching index, so
  the generated repository is one a reader can `cd` into and `git status`.

### Phase 4 — The links back

- [x] **4a.** `bower/src/trailers.rs`: `commit_message()` emitting
  `Book-Source`, `Book-Url`, `Bower-Step`, and `Generated-By`, with `Book-Url`
  omitted entirely when the book declares no site — a plausible-looking broken
  link is worse than no link. **Addition:** `scaffolding_message()`, because
  step 0 comes from the template directory rather than any chapter and a
  `Book-Source` on it would name a chapter that did not produce it.
- [x] **4b.** `steps_md()`, inserted into the final step's blobs only. The
  generated repository therefore holds eleven files at HEAD, not the kernel's
  ten — see the corrigendum.
- [x] **4c.** Five tests, including `html_name__matches_what_mdbook_emits`,
  which pins the `src/ch03-….md` → `ch03-….html` mapping the `Book-Url` depends
  on.

### Phase 5 — The golden

- [x] **5a.** `bower/tests/determinism.rs`. **Deviation:** the tests drive the
  **installed binary** via `CARGO_BIN_EXE_bower` rather than the crate's
  internals. The artifact a reader runs is then the artifact under test, and the
  CLI's own argument handling falls inside the guarantee. It also avoids
  splitting `bower` into a lib and a bin purely to satisfy a test.
- [x] **5b.** **Deviation:** `EXPECTED_TAGS` and `FINAL_PATHS` were consts
  inside `bower-testkit/tests/sample_book.rs`, unreachable from another crate.
  They are now `fixtures::HELLO_PLAYBOOK_TAGS` and
  `fixtures::HELLO_PLAYBOOK_FINAL_PATHS`, and both test suites assert against
  the one list. Tags are read straight from `.git/refs/tags` rather than through
  a git library, so the test cannot share a bug with the code it checks.
- [x] **5c.** `starts_from_empty_even_over_a_dirty_directory` plants a stray
  file and corrupts `Cargo.toml` between two runs, then asserts the SHAs still
  match and the stray is gone.

### Phase 6 — Documentation

- [x] **6a.** `README.md` now says Phases 1 and 2, lists the `bower` crate,
  documents the purity gate, and shows how to build the sample book's repo.
- [x] **6b.** Spec § 12 Q1 struck through and answered: `gix`, with the
  reasoning that actually decided it once the code existed.
- [x] **6c.** Status rows flipped, domain map updated, corrigendum below.

---

## Evidence — the book's own claim, checked by hand

Phase 3 did not automate verification; **EPIC-02 did**. What follows is the
manual check made on 1 September 2026, kept as the record of what was true at
the time this EPIC shipped. It is now asserted by
`bower/tests/verification.rs::upholds_the_two_deliberate_failures`, which runs
in the default gate.

```
$ cargo test                       # at HEAD
test result: ok. 2 passed

$ git checkout step-011-test-that-fails && cargo test
tests::greet__ignores_stray_whitespace --- FAILED
test result: FAILED. 1 passed; 1 failed

$ git checkout step-013-wont-compile && cargo build
error[E0308]: mismatched types
error: could not compile `hello-playbook` (lib)
```

The two steps that *declare* failure genuinely fail, and every other step builds
and passes. Turning that paragraph into a test was exactly what spec § 11
Phase 3 — EPIC-02 — was for.

---

## Test Plan

- `config__loads_the_sample_book` — `bower.toml` round-trips into `BookConfig`.
- `config__missing_epoch_is_an_error` — a book with no epoch cannot replay
  deterministically, so it must fail loudly at load, not silently at commit.
- `config__unknown_key_is_an_error` — `deny_unknown_fields` holds; a misspelled
  setting is a loud failure, not a silent default.
- `config__catalog_carries_only_kernel_settings` — pins the purity boundary: the
  projection drops `github`, `template`, `verify`, and `links`.
- `loader__reads_hello_playbook_in_summary_order` — chapters arrive in
  `SUMMARY.md` order, named book-root-relative.
- `loader__matches_the_testkit_fixture_exactly` — the loader's `BookSource`
  equals the testkit fixture's. Pins the two representations together forever.
- `links__are_in_document_order_without_repeats` — a chapter listed twice is
  planned once; external links and non-markdown targets are skipped.
- `links__empty_summary_yields_nothing` — the precondition for
  `LoadError::NoChapters`.
- `loader__missing_book_is_an_error` — a bad `--book` path fails by name.
- `trailers__name_the_chapter_and_line` — `Book-Source` points at a real file.
- `trailers__book_url_is_omitted_without_a_site` — no broken links.
- `hello_playbook_is_byte_identical_across_runs` — the headline requirement of
  spec § 5.1.
- `starts_from_empty_even_over_a_dirty_directory` — the no-incremental rule,
  tested rather than assumed.
- `tags_match_the_planned_tags` — tags are the book's only stable link target,
  so a drifted tag is a broken book.
- `final_worktree_is_the_kernels_tree_plus_steps_md` — the generated repository
  is exactly the kernel's tree, plus the one file the replay layer adds.
- `steps_md_indexes_every_step_back_into_the_book` — no step is missing from
  the repository's own table of contents.

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
- `bower-testkit/src/fixtures.rs:37,63` — `HELLO_PLAYBOOK_TAGS` and
  `HELLO_PLAYBOOK_FINAL_PATHS`. Both test suites assert against these, not
  against a fresh list.

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
git -C /tmp/hp-1 tag | wc -l        # 26: 20 step tags + 6 chapter-end tags
git -C /tmp/hp-1 status --short     # empty: the worktree and index match HEAD
```

Exit criteria:

1. Two independent replays of an unchanged `hello-playbook` produce identical
   commit SHAs, and the `diff` above is empty.
2. The generated repository holds twenty commits plus a scaffolding commit, and
   at HEAD the ten files listed at `bower-testkit/src/fixtures.rs:63` **plus**
   the generated `STEPS.md` — eleven in all. See corrigendum item 6.
3. Every commit carries `Book-Source` and `Bower-Step` trailers pointing at a
   chapter file that exists.
4. `cargo tree -p bower-core -e normal` still lists no dependencies — the kernel did not
   acquire I/O to make replay convenient.
5. `bower-testkit` and the CLI loader produce equal `BookSource` values for the
   sample book.


---

## Implementation corrigendum

What the design assumed, and what building it actually showed. Recorded
1 September 2026, branch `docs/epic-01`.

### 1. `time` was needed a phase earlier than planned

The design put `time` in Phase 3 with `gix`. But `config__missing_epoch_is_an_error`
is a Phase 1 requirement, and an epoch cannot be validated without a type that
knows what an instant is. It landed in Phase 1. The general rule survived — a
dependency arrives in the phase that uses it — the schedule was simply wrong
about which phase that was.

### 2. The kernel does not model file modes, so the replayer had to

`FileBody` is text or bytes (`bower-core/src/tree.rs:15`); there is no
permission bit anywhere in the kernel. But `bin/security-scan` has to be
`100755` or the generated `Makefile` cannot invoke it. The rule chosen lives in
`blobs_of`: a text file beginning `#!` is executable, and nothing else is.
Verified by `blobs_of__marks_shebang_scripts_executable` and by the mode `git
ls-tree` reports. If a future book needs an executable without a shebang, the
honest fix is a `mode=` directive key in the kernel, not a longer heuristic here.

### 3. `gix::init` inherits the machine's default branch

Left alone, one developer's replay produces `master` and another's `main`. That
is not a SHA difference, but it is a reproducibility difference, and it would
have surfaced as a confusing diff rather than a failing test. `HEAD` is now
written explicitly as `ref: refs/heads/main`.

### 4. `SignatureRef::time` is a string, and that turned out to be a feature

`gix_actor::SignatureRef::time` is `&str` holding git's own `<unix> <offset>`
format, not a typed instant. It reads like a wart. In practice it is exactly
what determinism wants: the offset is written as `+0000` by hand, so no local
timezone can reach a SHA. `time` is used only to add minutes and produce a unix
timestamp.

### 5. A repository needs a working tree, and the design never said so

The design described commits and tags and stopped there. A reader who runs
`bower build` and then `ls` should see files. `write_worktree` materializes the
final step and writes a matching index from the HEAD tree, so `git status` on a
freshly generated repository is clean. Asserted by
`final_worktree_is_the_kernels_tree_plus_steps_md`.

### 6. `STEPS.md` makes HEAD eleven files, not ten

Exit criterion 2 originally said the generated repository holds the ten files
the kernel's tree holds. It holds eleven: the replay layer adds `STEPS.md`, and
that file exists precisely because it is *not* book content. The criterion has
been corrected rather than quietly satisfied. `STEPS.md` is written into the
final step's tree only — it lists every step, so putting it in each commit would
make step 3's tree depend on step 20.

### 7. The golden tests drive the binary, not the crate

An integration test in `bower/tests/` cannot reach a binary crate's modules.
The two ways out are splitting `bower` into a lib plus a bin purely to satisfy a
test, or driving the built binary through `CARGO_BIN_EXE_bower`. The second was
chosen: the artifact a reader runs is the artifact under test, and the CLI's own
argument handling falls inside the guarantee. It also forced
`HELLO_PLAYBOOK_TAGS` and `HELLO_PLAYBOOK_FINAL_PATHS` out of a test file and
into `bower-testkit`'s public surface, which is where shared expectations
belonged anyway.

### 8. Chapter paths changed shape, and one test now guards it

Work item 2c was predicted in the design and landed as written: the fixture's
bare `ch01-….md` became `src/ch01-….md` to match the loader. The visible effect
is in `bower.lock` and every `Book-Source` trailer, which now name a path a
reader can open. `loader__matches_the_testkit_fixture_exactly` is what stops the
two representations drifting again.

### Phase status summary

| Phase | Status | Notes |
|---|---|---|
| 0 (the crate exists) | Shipped | dependencies deferred to their own phases |
| 1 (configuration) | Shipped | `time` pulled forward, see item 1 |
| 2 (loading and `plan`) | Shipped | chapter paths changed shape, see item 8 |
| 3 (replay) | Shipped | items 2–5 |
| 4 (the links back) | Shipped | item 6 |
| 5 (the golden) | Shipped | item 7 |
| 6 (documentation) | Shipped | spec § 12 Q1 closed in favour of `gix` |

### Still open after this EPIC

- ~~**Verification.**~~ Closed by EPIC-02.
- **`push`, the mdBook preprocessor, and `status`.** Spec § 11 Phase 4.
- **Multi-repo books.** The code loops over repos and gives each its own
  directory, but no book exercises it, so it is untested rather than proven.
