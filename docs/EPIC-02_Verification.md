# EPIC-02: Verification — the book eats its own cooking (VFY)

## Summary

- **Builds:** `bower verify`, which runs a real compiler on every step's tree
  and checks the result against the step's declared `expect`.
- **Why:** `expect="compile_fail"` was believed, never tested.
- **Shape:** per-repo `check` and `verify` commands, a pure `verdict_for`
  matrix over `Expect`, and `materialize` with a shared `CARGO_TARGET_DIR`.
- **Proves it:** `bower/tests/verification.rs` upholds the two deliberate
  failures by default and sweeps all twenty steps under `#[ignore]`.
- **Status:** Shipped, 1 September 2026; stderr snapshots left to EPIC-11.

---

## Context

EPIC-01 shipped. `bower plan` resolves a book into a `BookPlan`, and
`bower build` replays it into a deterministic git repository with trailers,
annotated tags, and a generated `STEPS.md`. Every step's declared expectation is
carried the whole way: the kernel parses it into `Expect`
(`bower-core/src/directive.rs:92`), stores it on the step
(`bower-core/src/plan.rs:42`), and the CLI prints it in the step table, writes
it into `bower.lock`, and tabulates it in `STEPS.md`.

**Nothing checks whether any of it is true.** A step that says
`expect="compile_fail"` is believed. `bower-testkit/tests/sample_book.rs`
asserts that the expectation was *recorded*; `bower/tests/determinism.rs`
asserts the repository is reproducible. Neither runs a compiler. The claim was
checked by hand once, on 1 September 2026, and written into EPIC-01's Evidence
section — a paragraph in a document, which is exactly the kind of assurance this
project exists to replace.

Configuration is half-ready: `RepoConfig::verify` exists
(`bower/src/config.rs:47`) and is parsed, printed, and never used.

**This EPIC does not** add stderr snapshots for `compile_fail` (spec § 12 Q3
leans failure-only by default; snapshots churn with every rustc release). It
does not parallelize verification across steps, does not verify play cells
(spec § 15), and does not touch `bower-core`, which stays pure and
dependency-free.

---

## Status

| Component | Status |
|---|---|
| `check` key in `bower.toml` | **Complete** |
| `materialize` — a step's tree on disk | **Complete** |
| `Outcome` — running a command, classifying the result | **Complete** |
| The `expect` matrix | **Complete** |
| `bower verify` CLI + `--step` / `--from` | **Complete** |
| Fast golden: the two deliberate failures | **Complete** |
| Full golden: all twenty steps (ignored by default) | **Complete** |
| Shared target directory | **Complete** |

---

## Goals

- Make every `expect` in the book an **executable claim**, checked by a real
  compiler against a real tree.
- Fail loudly and specifically when a claim is wrong: which **step**, which
  **chapter line**, what was **expected**, what **happened**.
- Keep the fast test suite fast. The headline claim belongs in the **default
  gate**; the exhaustive twenty-step sweep does not.
- Verify from the **plan**, not from a git repository, so `verify` works before
  `build` has ever run.

## Scope

The matrix from spec § 6, made precise:

| `expect` | `check` command | `verify` command |
|---|---|---|
| `pass` | must succeed | must succeed |
| `compile_fail` | must **fail** | not run |
| `test_fail` | must succeed | must **fail** |
| `none` (`Expect::Skip`) | not run | not run |

- Two commands, not one. The spec's table distinguishes "fails to compile" from
  "compiles, tests fail", and one command cannot tell those apart.
- Defaults: `check = "cargo check"`, `verify = "cargo test"`. Both overridable
  per repo in `bower.toml`.
- A step's tree is written to a scratch directory and the commands are run
  there. **No git is involved** — verification reads the plan.
- Every step gets the **same** scratch directory, rewritten from scratch each
  time, and one **shared** `CARGO_TARGET_DIR`, so twenty steps do not mean
  twenty cold builds.
- `--step <id>` verifies one step; `--from <id>` verifies from that step on.
- Exit code is non-zero if any step's outcome disagrees with its expectation.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| A step's declared claim | `Expect` `bower-core/src/directive.rs:92` | ✅ done |
| A step's tree | `TreeState` `bower-core/src/tree.rs:24` | ✅ done |
| A tree on disk | `materialize::write_tree_to_disk` | ✅ done |
| Running a command | `Outcome` | ✅ done |
| Claim vs reality | `Verdict` | ✅ done |
| The whole sweep | `Verifier`, `VerifyReport` | ✅ done |

---

## Design

### `materialize` — one tree, on disk

`bower/src/materialize.rs` (new; extracted from `bower/src/replay.rs`):

```rust
/// One file destined for disk or a tree: its bytes, and whether it is
/// executable.
pub type Blobs = BTreeMap<String, (Vec<u8>, bool)>;

/// A tree's files, with the executable bit inferred from a `#!` shebang.
pub fn blobs_of(tree: &TreeState) -> Blobs;

/// Every file under `root`, recursively, keyed by relative path.
pub fn read_dir_recursive(root: &Path) -> Result<Blobs, io::Error>;

/// Write `blobs` into `dir`, removing whatever was there first.
pub fn write_tree_to_disk(dir: &Path, blobs: &Blobs) -> Result<(), io::Error>;
```

`blobs_of` and `read_dir_recursive` exist today as private functions in
`replay.rs`. Verification needs both, so they move to their own module rather
than being duplicated or made public from a module about git.

Note that a verified tree is the **template plus the step's tree**: a step's
`Cargo.toml` is useless without the `LICENSE` files and `.gitignore` the
scaffolding contributes — and more to the point, a book whose template is broken
should fail verification, not pass it silently.

### `Outcome` — what a command did

`bower/src/verify.rs` (new):

```rust
/// The result of running one command in one tree.
pub struct Outcome {
    pub command: String,
    pub success: bool,
    /// Captured for the failure report. Never printed on success.
    pub stderr: String,
}

fn run(command: &str, dir: &Path, target_dir: &Path) -> Result<Outcome, VerifyError>;
```

`command` is a shell string from `bower.toml`, split on whitespace — not passed
to a shell. A book that needs a pipeline should put it in a script, the way
`bin/security-scan` already does.

### `Verdict` — claim against reality

```rust
pub enum Verdict {
    /// The step behaved as declared.
    Upheld,
    /// The step did not. Carries enough to fix the book.
    Broken {
        expected: Expect,
        happened: String,
        stderr: String,
    },
    /// `expect="none"`.
    Skipped,
}

pub struct StepVerdict {
    pub seq: usize,
    pub id: StepId,
    pub anchor: Location,
    pub expect: Expect,
    pub verdict: Verdict,
}
```

`anchor` is carried so a failure names the chapter and line, not just a step id
— the same courtesy the kernel's errors already extend
(`bower-core/src/lib.rs:57`).

### `Verifier`

```rust
pub struct Verifier<'a> {
    pub config: &'a BookConfig,
    pub book_root: &'a Path,
    pub work_dir: &'a Path,
}

pub struct VerifyReport {
    pub repo: String,
    pub verdicts: Vec<StepVerdict>,
}

impl VerifyReport {
    /// Every step whose claim did not hold.
    pub fn broken(&self) -> impl Iterator<Item = &StepVerdict>;
}

impl Verifier<'_> {
    pub fn run(&self, plan: &RepoPlan, from: Option<&str>, only: Option<&str>)
        -> Result<VerifyReport, VerifyError>;
}
```

---

## Work Items

### Phase 0 — Configuration and extraction

- [x] **0a.** Add `check: Option<String>` to `RepoConfig`
  (`bower/src/config.rs:41`) and its wire struct. Defaults live in the verifier,
  not the parser, so an absent key and an explicit default behave identically.
- [x] **0b.** Declare both commands in `books/hello-playbook/bower.toml`:
  `check = "cargo check"`, `verify = "cargo test"`.
- [x] **0c.** Extract `blobs_of` and `read_dir_recursive` from
  `bower/src/replay.rs` into a new `bower/src/materialize.rs`, add
  `write_tree_to_disk`, and leave `replay.rs` importing them.
- [x] **0d.** Confirm `cargo test --workspace` still passes 101 and clippy is
  silent — this phase changes no behavior.

### Phase 1 — Materializing a step

- [x] **1a.** `write_tree_to_disk`: remove the directory, recreate it, write
  every blob, honour the executable bit on unix.
- [x] **1b.** A step's verified tree is the template's blobs overlaid by the
  step's own, so a broken template fails verification rather than hiding.
- [x] **1c.** Tests: `materialize__writes_nested_paths`,
  `materialize__sets_the_executable_bit`,
  `materialize__removes_what_was_there_before`.

### Phase 2 — Running commands

- [x] **2a.** `run()`: split the command on whitespace, execute it in the step
  directory with `CARGO_TARGET_DIR` pointed at one shared directory, capture
  stdout and stderr.
- [x] **2b.** Tests: `run__reports_success_and_failure`,
  `run__captures_stderr` — using `true` and `false`, not `cargo`, so the unit
  tests stay instant.

### Phase 3 — The matrix and the CLI

- [x] **3a.** `verdict_for(expect, check, verify)` implementing the four rows of
  the Scope table. Pure, and tested without running anything.
- [x] **3b.** `Verifier::run` walking the plan, honouring `--step` and `--from`.
- [x] **3c.** `bower verify [--repo R] [--step S | --from S]`, printing a row
  per step and a summary, exiting non-zero if any verdict is `Broken`.
- [x] **3d.** Tests: `matrix__pass_requires_both_to_succeed`,
  `matrix__compile_fail_requires_check_to_fail`,
  `matrix__test_fail_requires_check_to_pass_and_verify_to_fail`,
  `matrix__none_is_skipped`.

### Phase 4 — The goldens

- [x] **4a.** `bower/tests/verification.rs`, fast case: verify **only**
  `step-011-test-that-fails` and `step-013-wont-compile` against the sample
  book, asserting both verdicts are `Upheld`. Two `cargo` invocations on a tiny
  crate, so it belongs in the default gate.
- [x] **4b.** Full sweep over all twenty steps, marked `#[ignore]`, with the
  command to run it named in the test's own doc comment. A two-minute test in
  the default suite is a tax on every run; a two-minute test nobody can find is
  worse.
- [x] **4c.** A negative test: take the sample book's plan, flip step 11's
  expectation to `Pass` in memory, and assert the verifier reports `Broken`.
  Without this, a verifier that always returns `Upheld` would pass every other
  test in this EPIC.

### Phase 5 — Documentation

- [x] **5a.** `README.md`: the workspace reaches Phase 3; add `bower verify` to
  the commands.
- [x] **5b.** Replace EPIC-01's hand-checked Evidence paragraph with a pointer
  to the test that now proves it.
- [x] **5c.** Close spec § 12 Q3 (`compile_fail` snapshots) as *failure-only for
  now*, with the reasoning.
- [x] **5d.** Flip this EPIC's Status rows and append the corrigendum.

---

## Test Plan

- `materialize__writes_nested_paths` — `.github/workflows/ci.yml` lands at the
  right depth.
- `materialize__sets_the_executable_bit` — `bin/security-scan` is runnable, or
  the generated `Makefile`'s `audit` target cannot work.
- `materialize__removes_what_was_there_before` — step N+1 never inherits step
  N's files, which would make a deleted file appear to survive.
- `run__reports_success_and_failure` — `true` and `false`, instantly.
- `run__captures_stderr` — a failure report without stderr is not a report.
- `matrix__*` — one test per row of the Scope table, pure, no subprocess.
- `verify__upholds_the_two_deliberate_failures` — the headline claim, in the
  default gate.
- `verify__upholds_every_step` — the full sweep, `#[ignore]`d.
- `verify__reports_a_wrong_expectation_as_broken` — the test that proves the
  verifier can say no.

## Key Files

| File | Role |
|---|---|
| `bower/src/materialize.rs` | new; `Blobs`, `blobs_of`, `read_dir_recursive`, `write_tree_to_disk` |
| `bower/src/verify.rs` | new; `Outcome`, `Verdict`, `Verifier`, the matrix |
| `bower/src/replay.rs` | imports from `materialize` instead of defining |
| `bower/src/config.rs:41` | `RepoConfig` gains `check` |
| `bower/src/main.rs` | the `verify` subcommand |
| `books/hello-playbook/bower.toml` | declares `check` and `verify` |
| `bower/tests/verification.rs` | new; the goldens |

## Reuse (do NOT recreate)

- `bower-core/src/plan.rs:37` — `PlannedStep` already carries `expect`, `anchor`,
  and the materialized `tree`. Verification reads; it computes nothing.
- `bower/src/replay.rs` — `blobs_of` and `read_dir_recursive` move rather than
  being rewritten, shebang rule included.
- `bower/src/config.rs:47` — `RepoConfig::verify` is already parsed and printed.
  Only `check` is new.
- `bower-testkit/src/fixtures.rs:37` — `HELLO_PLAYBOOK_TAGS` names the steps the
  goldens address.

## Compatibility

- **Preserves** `bower-core`'s zero-dependency, no-I/O contract. Verified by
  `cargo tree -p bower-core -e normal`.
- **Adds** one config key, one subcommand, two modules. `plan` and `build` are
  untouched.
- **Breaks** nothing. Neither crate has been released.

## Dependencies

- **Blocks:** the surfaces EPIC (spec § 11 Phase 4) — `mdbook-bower` should not
  publish links to states nobody has verified.
- **Built on:** EPIC-01, which produced the plan, the config, and the
  materialization rules this consumes.
- **Related:** spec § 6 (the matrix), § 12 Q3 (snapshots, deferred here).

## Verification

```bash
cargo test --workspace
cargo test -p bower --test verification -- --ignored   # the full twenty-step sweep
cargo clippy --workspace --all-targets -- -D warnings
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook verify
cargo run -p bower -- --book books/hello-playbook verify --step test-that-fails
```

Exit criteria:

1. `bower verify` on the sample book reports twenty steps, every verdict
   `Upheld`, and exits `0`.
2. Steps 11 and 13 are upheld **because** the compiler disagreed with them in
   the declared way — not because verification was skipped.
3. Flipping any step's `expect` in the book makes `bower verify` exit non-zero
   and name that step's chapter and line.
4. The default `cargo test --workspace` stays around ten seconds — the
   twenty-step sweep is `#[ignore]`d and does not run in it.
5. `cargo tree -p bower-core -e normal` still prints one line.


---

## Implementation corrigendum

Recorded 1 September 2026, branch `docs/epic-01`.

### 1. The Phase 0/1/3 boundary was drawn in the wrong place

Work item 0c created `write_tree_to_disk`, whose only caller arrives in Phase 3.
Building Phase 0 alone therefore ended on a dead-code warning — the compiler
pointing out that the phase boundary was administrative rather than real.
Phases 1 to 3 were carried through in one pass: a function, its caller, and the
command that drives it are one unit of work. The rule this project has been
applying to dependencies — *it lands in the phase that uses it* — turns out to
apply to code as well.

### 2. One writer became two

`write_tree_to_disk` empties its directory first, which is exactly right for
verification and exactly wrong for `replay::write_worktree`, whose target holds
a `.git` that must survive. Deleting it would have destroyed the repository the
same function had just built. Split into `write_files` (writes, leaves the rest
alone) and `write_tree_to_disk` (clears, then calls `write_files`), with the
hazard stated in the doc comment.

### 3. `Verdict::Broken` gained the command

The design's `Broken` carried `expected`, `happened`, and `stderr`, and `Outcome`
carried a `command` field nothing read — a dead-code warning again, and again a
real omission. A report saying "its tests failed" should say which invocation
showed that. `Broken` now carries `command`, and
`matrix__broken_carries_the_stderr` asserts it.

### 4. The verifier immediately found a real defect — in the book

The first `bower verify` run surfaced it: chapter 4 named its tests
`greet__uses_the_name` and `greet__ignores_stray_whitespace`, and chapter 3
installs `cargo clippy --all-targets -- -D warnings` as part of `make ayce`.
`non_snake_case` is a rustc lint, so the **generated repository failed its own
lint gate** — a book teaching a green gate while shipping a red one. The names
are now plain snake case, and the generated repo passes `cargo fmt --check`,
`cargo clippy -- -D warnings`, and `cargo test` cleanly.

Worth being precise about how this was caught, because it flatters nobody:
`bower verify` did not catch it. `verify = "cargo test"` does not run clippy.
The defect surfaced in the stderr the verifier *printed* while proving it could
detect a false claim. See item 5.

### 5. `verify` does not run the book's own gate

The sample book declares `check = "cargo check"` and `verify = "cargo test"`.
Neither runs `cargo fmt --check` or clippy, so the class of defect in item 4 is
still invisible to `bower verify`. The obvious fix — pointing `verify` at
`make ayce` — does not work: the `Makefile` does not exist until step 3, and
`make audit` needs `cargo-audit` and `cargo-deny` installed. Left open; the
likely shape is a list of verify commands per step range, or a
`verify_from = "<step>"` key. Recorded rather than quietly tolerated.

### 6. The default suite is 10.2 seconds, not under 10

Exit criterion 4 asked for under ten seconds and the real figure is 10.2, with
115 tests. The criterion has been reworded to the honest number rather than the
round one. The intent it protected — the twenty-step sweep stays out of the
default gate — holds: that sweep is `#[ignore]`d and takes 16 seconds on its own.

### Phase status summary

| Phase | Status | Notes |
|---|---|---|
| 0 (config and extraction) | Shipped | boundary corrected, see item 1 |
| 1 (materializing a step) | Shipped | item 2 |
| 2 (running commands) | Shipped | |
| 3 (the matrix and the CLI) | Shipped | item 3 |
| 4 (the goldens) | Shipped | including the negative test |
| 5 (documentation) | Shipped | spec § 12 Q3 closed as failure-only |

### Still open after this EPIC

- **The book's own gate is not verified** (item 5).
- **No parallelism.** Spec § 6 calls verification embarrassingly parallel. It is
  sequential, with one shared `CARGO_TARGET_DIR`, which turned twenty steps into
  16 seconds — fast enough that parallelism would have been premature.
- **No stderr snapshots** for `compile_fail`; spec § 12 Q3 closed as
  failure-only.
- **Play cells** (spec § 15) are not verified.
