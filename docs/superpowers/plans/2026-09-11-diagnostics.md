# Captured Diagnostics (EPIC-11) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make what the compiler says at a step an executable claim: an `output="check"|"verify"` block holds the normalized output of that command, `bower verify` fails when it drifts, and `bower verify --record` writes it.

**Architecture:** The kernel (`bower-core`) learns one directive key, binds output blocks to steps with the play-cell rule, and gains a pure `capture` module — `normalize`, `drift` (with `[...]` elision), `error_codes`, `rewrite`. The CLI (`bower`) runs each command through one pipe, controls the toolchain, judges outputs of upheld steps, rewrites fences on `--record`, and renders a caption above each output fence. No output block touches a tree, a commit, or a SHA.

**Tech Stack:** Rust 2024 workspace (`rust-version = "1.98.1"`); `bower-core` (zero deps), `bower` CLI, `bower-testkit` (proptest, rstest). `std::io::pipe` (stable since 1.87) — no new dependency anywhere.

**Spec:** `docs/EPIC-11_Diagnostics.md` — read it first, **Decisions 1–13** and the settled open-questions table in particular. Every task cites its decisions.

## Global Constraints

- **Git is the user's.** Never run a state-changing git command (`~/.claude/CLAUDE.md`). Every "Commit" step means: stop, print the exact `git add … && git commit -m "…"` line for the user, and wait. Branch: `feat/diagnostics` (already created by the user).
- **Kernel purity.** `bower-core` gains no dependency and no I/O. `cargo tree -p bower-core -e normal` still prints one line (`make lint` runs `make purity`). The ANSI stripper and the error-code parser are hand-written; no `regex`.
- **Errors.** Every new `BowerError` variant carries a `Location`, is collected (never short-circuits), and has a `Display` arm. Names begin with `Output`; the corpus test matches on the Debug prefix.
- **Lints.** `#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]` in every crate; test modules `#[allow(non_snake_case, clippy::unwrap_used)]`. Test names use `subject__does_thing`.
- **Exact values** (copy verbatim):
  - Elision line: `[...]` (`bower_core::capture::ELISION`).
  - Serial environment for a step with output blocks: `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, `RUST_BACKTRACE=0`.
  - `run` always removes `RUSTUP_TOOLCHAIN`; the `toolchain` key sets it only when the tree has no root `rust-toolchain.toml` or `rust-toolchain`.
  - Derived error-code template: `https://doc.rust-lang.org/error_codes/{code}.html` (`RUSTC_ERROR_CODES`), when the repo's `check` command's first word is `cargo`.
  - Caption: `<span class="step-output"><sub>$ {command} · step {seq:03} of {repo}[ · {code link}]*</sub></span>`.
  - Lock lines: `    output {capture} at={chapter}:{line}` then `    > {line}` per recorded line.
  - Verify rows: step row unchanged (`  {mark} {seq:03} {id:<28} expect={expect}`); output row `       {mark} output={capture:<6} {loc}` with marks `ok  `, `drft`, `todo`, `skip`.
- **Run tests per crate while working** (`cargo test -p bower-core`), and `make ayce` once at the end.
- **Toolchains.** After Task 2, `hello-playbook` steps from `toolchain` on run on its own pin, Rust `1.95.0`. rustup installs it on first use; a CI runner downloads it once.

---

## File Structure

| File | Responsibility in this feature |
|---|---|
| `bower-core/src/directive.rs` | `Capture` (check/verify), `Directive::output`. |
| `bower-core/src/lib.rs` | Five `Output*` errors; `pub mod capture`; prelude. |
| `bower-core/src/block.rs` | `Block::output`; output blocks are inert and refuse other keys; `fence_width`/`fence_span` shared with `capture`. |
| `bower-core/src/plan.rs` | `CapturedOutput`, `PlannedStep::outputs`, `bind_outputs`, lock lines. |
| `bower-core/src/capture.rs` (new) | `tidy`, `Scrub`, `normalize`, `ELISION`, `Drift`, `drift`, `error_codes`, `rewrite`. Pure. |
| `bower-core/src/{display,tree,step}.rs` | One test-only `Block` literal each gains `output: None`. |
| `bower-testkit/src/fixtures.rs` | `captured_outputs()`, five broken fixtures. |
| `bower-testkit/src/coverage.rs` | `Mechanism::CapturedOutput`; the `capture × expect` axis. |
| `bower-testkit/src/textures.rs` (new) | Real raw outputs, as constants. |
| `bower-testkit/src/generators.rs` | `arb_raw_output`, `arb_plain_lines`. |
| `bower-testkit/tests/{corpus,textures,properties,sample_book}.rs` | Binding, textures, four properties, the sample book's outputs. |
| `bower/src/verify.rs` | `Outcome::output` (one pipe), `step_env`, `pins_toolchain`, `Scrub` from paths, `OutputVerdict`, `judge_output`. |
| `bower/src/record.rs` (new) | `recordings`, `apply`, `write_chapters`. |
| `bower/src/config.rs` | `RepoConfig::toolchain`; `LinkTemplates::error_code`; `RUSTC_ERROR_CODES`. |
| `bower/src/main.rs` | `--record`; output rows and reports; the toolchain warning; `write_lock`. |
| `bower/src/render.rs` | `outputs_by_line`, `output_caption`. |
| `bower/tests/{verification,determinism,preprocessor}.rs` | Goldens. |
| `books/*/theme/step-meta.css` | `.step-output`. |
| `books/hello-playbook/src/ch04-…md`, `books/rust4failures/{bower.toml,src/ch01-…md,src/ch03-rank.md}` | The recorded outputs. |
| `bower-spec.md`, `README.md`, `.okf/…`, `BACKLOG.md`, `docs/TECHNICAL_DEBT.md`, `docs/EPIC-11_Diagnostics.md` | The key, the feature, the closed debt. |

Task order: edge prerequisites (1–2), kernel (3–7), testkit (8–9), verify and record (10–11), render (12), books (13), docs (14).

---
### Task 1: Both streams through one pipe (EPIC Phase 0a, Decision 2)

**Files:**
- Modify: `bower/src/verify.rs` (`Outcome`, `Verdict::Broken`, `verdict_for`, `run`, tests)
- Modify: `bower/src/main.rs:359-378` (the `Broken` report)

**Interfaces:**
- Produces: `Outcome { command: String, success: bool, output: String }` — `output` is both streams, interleaved as written. `Verdict::Broken { happened: String, command: String, output: String }`. `run(command: &str, dir: &Path, target_dir: &Path) -> Result<Outcome, VerifyError>` (Task 2 adds a fourth parameter).

- [ ] **Step 1: Write the failing test**

In `bower/src/verify.rs`, inside `mod verify_tests`, add a helper and a test:

```rust
    /// A shell script in its own directory, run the way `run` runs any
    /// command: split on whitespace, no shell of ours in between.
    fn script(case: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bower-verify-run-{case}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("s.sh"), body).unwrap();
        dir
    }

    #[test]
    fn run__captures_both_streams_in_order() {
        // A reader's terminal interleaves the two streams in the order they
        // were written. Stderr-then-stdout would print cargo's closing
        // `error: test failed` above libtest's `running 1 test`.
        let dir = script("order", "echo one\necho two >&2\necho three\n");
        let out = run("sh s.sh", &dir, &dir.join("target")).unwrap();
        assert_eq!(out.output, "one\ntwo\nthree\n");
        assert!(out.success);
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p bower --lib run__captures_both_streams_in_order`
Expected: compile error, `no field `output` on type `Outcome``.

- [ ] **Step 3: Rename the field and read one pipe**

In `bower/src/verify.rs`:

Replace the imports

```rust
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
```

with

```rust
use std::fmt;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
```

Replace `struct Outcome`:

```rust
/// The result of running one command in one tree.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub command: String,
    pub success: bool,
    /// Both streams, interleaved in the order they were written: what a
    /// reader's terminal shows. The failure report prints its tail, and a
    /// recorded output is judged against it (EPIC-11 Decision 2).
    pub output: String,
}
```

In `enum Verdict`, the `Broken` variant becomes:

```rust
    /// It did not. Carries enough to go and fix the book: what happened, the
    /// command that showed it, and what that command printed.
    Broken {
        happened: String,
        command: String,
        output: String,
    },
```

In `verdict_for`, the `broken` closure becomes:

```rust
    let broken = |happened: &str, o: &Outcome| Verdict::Broken {
        happened: happened.to_string(),
        command: o.command.clone(),
        output: o.output.clone(),
    };
```

Replace the whole `fn run` with:

```rust
/// Run one command in `dir`, with a shared cargo target directory.
///
/// The command is split on whitespace, not handed to a shell. A book needing a
/// pipeline should put it in a script, the way `bin/security-scan` already does.
fn run(command: &str, dir: &Path, target_dir: &Path) -> Result<Outcome, VerifyError> {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return Err(VerifyError::EmptyCommand);
    };
    let io = |source| VerifyError::Io {
        path: PathBuf::from(program),
        source,
    };

    // One pipe for both streams, so the text keeps the order it was written
    // in: the order a reader's terminal shows (EPIC-11 Decision 2).
    let (mut reader, writer) = std::io::pipe().map_err(io)?;
    let mut cmd = Command::new(program);
    cmd.args(parts)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target_dir)
        .stdin(Stdio::null())
        .stdout(writer.try_clone().map_err(io)?)
        .stderr(writer);
    let mut child = cmd.spawn().map_err(io)?;
    // `cmd` still holds both write ends. Until it is gone the read below never
    // sees end-of-file, and waits for ever.
    drop(cmd);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(io)?;
    let status = child.wait().map_err(io)?;
    let output = String::from_utf8_lossy(&bytes).into_owned();

    if !status.success() && nested_workspace(&output) {
        return Err(VerifyError::NestedWorkspace {
            tree: dir.to_path_buf(),
        });
    }

    Ok(Outcome {
        command: command.to_string(),
        success: status.success(),
        output,
    })
}
```

`.stdin(Stdio::null())` keeps what `.output()` did before: a command that
reads stdin sees it closed at once.

- [ ] **Step 4: Update the existing tests**

In `mod verify_tests`, `ok` and `bad` build `output` instead of `stderr`:

```rust
    fn ok(cmd: &str) -> Outcome {
        Outcome {
            command: cmd.to_string(),
            success: true,
            output: String::new(),
        }
    }

    fn bad(cmd: &str) -> Outcome {
        Outcome {
            command: cmd.to_string(),
            success: false,
            output: "error[E0308]: mismatched types\n".to_string(),
        }
    }
```

Rename `matrix__broken_carries_the_stderr` to `matrix__broken_carries_the_output`
and change its pattern to `Verdict::Broken { output, command, .. }` with
`assert!(output.contains("E0308"));`.

- [ ] **Step 5: Update the report in `main.rs`**

In `run_verify`, the `Broken` destructuring becomes:

```rust
            let Verdict::Broken {
                happened,
                command,
                output,
            } = &v.verdict
            else {
                continue;
            };
            eprintln!(
                "\n{}: step `{}` claims `{}`, but {happened}.",
                v.anchor, v.id.0, v.expect
            );
            eprintln!("    (`{command}` in the step's tree)");
            let tail: Vec<&str> = output.lines().rev().take(8).collect();
```

(the rest of the loop is unchanged).

- [ ] **Step 6: Run the tests**

Run: `cargo test -p bower --lib verify_tests && cargo test -p bower --test verification`
Expected: PASS, including `run__captures_both_streams_in_order`,
`reports_a_wrong_expectation_as_broken`, and
`a_scratch_tree_inside_a_workspace_is_the_verifiers_fault_not_the_books`.

- [ ] **Step 7: Commit**

```bash
git add bower/src/verify.rs bower/src/main.rs
git commit -m "feat(verify): read both streams through one pipe, in written order"
```

---

### Task 2: The toolchain a step runs on (EPIC Phase 0b, Decisions 10 and 12)

**Files:**
- Modify: `bower/src/config.rs` (`RepoConfig`, `WireRepo`, `parse`, tests)
- Modify: `bower/src/verify.rs` (`step_env`, `pins_toolchain`, `run`, `Verifier::run`, tests)

**Interfaces:**
- Consumes: `run` from Task 1.
- Produces:
  - `RepoConfig::toolchain: Option<String>`.
  - `pub fn pins_toolchain(blobs: &Blobs) -> bool`.
  - `pub fn step_env(blobs: &Blobs, toolchain: Option<&str>, records_output: bool) -> Vec<(&'static str, String)>`.
  - `run(command: &str, dir: &Path, target_dir: &Path, env: &[(&str, String)]) -> Result<Outcome, VerifyError>`, which always removes `RUSTUP_TOOLCHAIN` before applying `env`.

- [ ] **Step 1: Write the failing config test**

In `bower/src/config.rs`, `mod config_tests`:

```rust
    #[test]
    fn config__toolchain_is_optional() {
        let bare = BookConfig::parse(&format!("{MINIMAL}[repos.r]\n")).unwrap();
        assert_eq!(bare.repos.get("r").unwrap().toolchain, None);

        let text = format!("{MINIMAL}[repos.r]\ntoolchain = \"1.98.1\"\n");
        let pinned = BookConfig::parse(&text).unwrap();
        assert_eq!(
            pinned.repos.get("r").unwrap().toolchain.as_deref(),
            Some("1.98.1")
        );
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p bower --lib config__toolchain_is_optional`
Expected: compile error, `no field `toolchain``.

- [ ] **Step 3: Add the key**

In `RepoConfig`, after `verify`:

```rust
    /// The rustup toolchain a step's commands run on when the step's tree
    /// pins none. A tree's own `rust-toolchain.toml` always wins: it is what a
    /// reader who checks the step out gets (EPIC-11 Decision 10).
    pub toolchain: Option<String>,
```

In `WireRepo`, after `verify: Option<String>,`:

```rust
    toolchain: Option<String>,
```

In `parse`, in the `RepoConfig { … }` literal, after `verify: r.verify,`:

```rust
                            toolchain: r.toolchain,
```

Run: `cargo test -p bower --lib config__toolchain_is_optional` — PASS.

- [ ] **Step 4: Write the failing verify tests**

In `bower/src/verify.rs`, `mod verify_tests`:

```rust
    fn blobs(paths: &[&str]) -> Blobs {
        paths
            .iter()
            .map(|p| ((*p).to_string(), (Vec::new(), false)))
            .collect()
    }

    #[test]
    fn toolchain__a_tree_pin_wins() {
        // hello-playbook teaches the pin. Overriding it would verify a
        // different book than the one a reader checks out.
        for pin in ["rust-toolchain.toml", "rust-toolchain"] {
            let env = step_env(&blobs(&["Cargo.toml", pin]), Some("1.98.1"), false);
            assert!(
                !env.iter().any(|(k, _)| *k == "RUSTUP_TOOLCHAIN"),
                "{pin}: {env:?}"
            );
        }
    }

    #[test]
    fn toolchain__the_key_fills_a_tree_without_one() {
        let env = step_env(&blobs(&["Cargo.toml"]), Some("1.98.1"), false);
        assert!(
            env.contains(&("RUSTUP_TOOLCHAIN", "1.98.1".to_string())),
            "{env:?}"
        );
        assert!(step_env(&blobs(&["Cargo.toml"]), None, false).is_empty());
    }

    #[test]
    fn step_env__a_recorded_output_runs_serially() {
        let env = step_env(&blobs(&["Cargo.toml"]), None, true);
        for (key, value) in [
            ("CARGO_BUILD_JOBS", "1"),
            ("RUST_TEST_THREADS", "1"),
            ("RUST_BACKTRACE", "0"),
        ] {
            assert!(env.contains(&(key, value.to_string())), "{env:?}");
        }
    }

    #[test]
    fn run__never_passes_the_inherited_toolchain_on() {
        // Under `cargo test`, rustup's proxy has already exported
        // RUSTUP_TOOLCHAIN into this process. The child must not see it: that
        // value is how `bower` was started, not what the book pins.
        let dir = script("toolchain", "echo \"${RUSTUP_TOOLCHAIN:-unset}\"\n");
        let out = run("sh s.sh", &dir, &dir.join("target"), &[]).unwrap();
        assert_eq!(out.output, "unset\n");

        let set = [("RUSTUP_TOOLCHAIN", "1.98.1".to_string())];
        let out = run("sh s.sh", &dir, &dir.join("target"), &set).unwrap();
        assert_eq!(out.output, "1.98.1\n");
    }
```

- [ ] **Step 5: Run them to see them fail**

Run: `cargo test -p bower --lib toolchain__`
Expected: compile error, `cannot find function `step_env``.

- [ ] **Step 6: Implement `pins_toolchain`, `step_env`, and the `run` parameter**

Add above `fn run`:

```rust
/// Does this tree carry its own rustup pin at the root?
#[must_use]
pub fn pins_toolchain(blobs: &Blobs) -> bool {
    blobs.contains_key("rust-toolchain.toml") || blobs.contains_key("rust-toolchain")
}

/// The environment a step's commands run with, beyond `CARGO_TARGET_DIR`.
///
/// The toolchain first (EPIC-11 Decision 10): a tree that pins one keeps its
/// pin, and only a tree that pins nothing gets the repo's `toolchain` key.
/// [`run`] removes the inherited `RUSTUP_TOOLCHAIN` either way, so whatever
/// launched `bower` never decides.
///
/// Then, for a step whose output the book records, the serial trio
/// (Decision 12): one build job and one test thread, so the order of what is
/// printed does not depend on scheduling, and no backtrace, so an author's
/// `RUST_BACKTRACE=1` does not end up in the book.
#[must_use]
pub fn step_env(
    blobs: &Blobs,
    toolchain: Option<&str>,
    records_output: bool,
) -> Vec<(&'static str, String)> {
    let mut env = Vec::new();
    if let Some(t) = toolchain
        && !pins_toolchain(blobs)
    {
        env.push(("RUSTUP_TOOLCHAIN", t.to_string()));
    }
    if records_output {
        env.push(("CARGO_BUILD_JOBS", "1".to_string()));
        env.push(("RUST_TEST_THREADS", "1".to_string()));
        env.push(("RUST_BACKTRACE", "0".to_string()));
    }
    env
}
```

Change `run`'s signature and command set-up:

```rust
fn run(
    command: &str,
    dir: &Path,
    target_dir: &Path,
    env: &[(&str, String)],
) -> Result<Outcome, VerifyError> {
```

and in the builder chain, after `.env("CARGO_TARGET_DIR", target_dir)`:

```rust
        // rustup's cargo proxy exports its own pin to every child, so a
        // `bower` started by `cargo run` would hand the workspace's toolchain
        // to the tree and override the tree's `rust-toolchain.toml`.
        .env_remove("RUSTUP_TOOLCHAIN")
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
```

- [ ] **Step 7: Pass the environment from `Verifier::run`**

After `let verify_cmd = …;` add:

```rust
        let toolchain = cfg.and_then(|c| c.toolchain.as_deref());
```

In the step loop, after `write_tree_to_disk(…)?;`, replace the two `run` calls:

```rust
                // `false` until output blocks exist (Task 10 passes
                // `!step.outputs.is_empty()`).
                let env = step_env(&blobs, toolchain, false);
                let check = run(check_cmd, &tree_dir, &target_dir, &env)?;
                let verify = if check.success && step.expect != Expect::CompileFail {
                    Some(run(verify_cmd, &tree_dir, &target_dir, &env)?)
                } else {
                    None
                };
```

Update the three older tests that call `run` to pass `&[]` as the fourth
argument (`run__reports_success_and_failure`, `run__empty_command_is_an_error`,
`run__captures_both_streams_in_order`).

- [ ] **Step 8: Run the tests**

Run: `cargo test -p bower --lib && cargo test -p bower --test verification`
Expected: PASS. `upholds_the_two_deliberate_failures` now runs
`hello-playbook`'s steps on its pinned `1.95.0` (rustup installs it on first
use if missing).

- [ ] **Step 9: Commit**

```bash
git add bower/src/config.rs bower/src/verify.rs
git commit -m "fix(verify): a step runs on its tree's toolchain pin, not the one that launched bower

rustup's cargo proxy exports RUSTUP_TOOLCHAIN to its children, so under
make the scratch tree's rust-toolchain.toml was silently ignored. Adds a
per-repo toolchain key for trees that pin nothing."
```

---
### Task 3: The `output` key, its five errors, and the inert block (EPIC Phase 1a–1b, Decisions 1 and 3)

**Files:**
- Modify: `bower-core/src/directive.rs` (`Capture`, `Directive::output`, `parse`, `overlaid_on`, tests)
- Modify: `bower-core/src/lib.rs` (five variants, `location`, `Display`, prelude)
- Modify: `bower-core/src/block.rs` (`Block::output`, `resolve`, tests)
- Modify: `bower-core/src/display.rs:280`, `bower-core/src/tree.rs:410`, `bower-core/src/step.rs:311` (test-only `Block` literals)

**Interfaces:**
- Produces:
  - `pub enum Capture { Check, Verify }` with `all() -> [Capture; 2]`, `runs_under(self, Expect) -> bool`, `Display` (via `f.pad`, so `{:<6}` pads), `FromStr<Err = String>`. Derives `Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd`.
  - `Directive::output: Option<Capture>`; `Block::output: Option<Capture>` (an output block has `op == Op::Prose`).
  - `BowerError::{OutputUnbound { loc }, OutputUnknownStep { loc, step: String }, OutputConflictingKeys { loc }, OutputDuplicate { loc, step: String, capture: Capture }, OutputNeverRuns { loc, step: String, capture: Capture, expect: Expect }}`.

- [ ] **Step 1: Write the failing directive tests**

In `bower-core/src/directive.rs`, `mod directive_tests`:

```rust
    #[test]
    fn parse__output_accepts_check_and_verify() {
        for (value, want) in [("check", Capture::Check), ("verify", Capture::Verify)] {
            let line = format!("<!-- bower repo=\"failers\" output=\"{value}\" -->");
            assert_eq!(Directive::parse(&line, &loc()).unwrap().output, Some(want));
        }
    }

    #[test]
    fn parse__output_rejects_any_other_value() {
        // The idea's bare `<!-- bower output -->` does not parse at all, and
        // a stream name is not a capture: captures name commands.
        let errs = Directive::parse("<!-- bower repo=\"r\" output=\"stderr\" -->", &loc())
            .unwrap_err();
        assert!(
            matches!(&errs.0[0], BowerError::BadValue { key, value, .. } if key == "output" && value == "stderr"),
            "{errs}"
        );
    }

    #[test]
    fn capture__runs_under_follows_the_matrix() {
        // `check` runs unless the step is skipped; `verify` only where the
        // check must pass (bower/src/verify.rs).
        assert!(Capture::Check.runs_under(Expect::Pass));
        assert!(Capture::Check.runs_under(Expect::CompileFail));
        assert!(Capture::Check.runs_under(Expect::TestFail));
        assert!(!Capture::Check.runs_under(Expect::Skip));
        assert!(Capture::Verify.runs_under(Expect::Pass));
        assert!(!Capture::Verify.runs_under(Expect::CompileFail));
        assert!(Capture::Verify.runs_under(Expect::TestFail));
        assert!(!Capture::Verify.runs_under(Expect::Skip));
    }
```

In `op__display_from_str_round_trips`, add:

```rust
        for c in Capture::all() {
            assert_eq!(c.to_string().parse::<Capture>().unwrap(), c);
        }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p bower-core directive_tests`
Expected: compile error, `cannot find type `Capture``.

- [ ] **Step 3: Add `Capture` and the key**

In `bower-core/src/directive.rs`, after the `impl std::str::FromStr for Expect` block:

```rust
/// Whose output an `output="…"` block records: the repo's `check` command or
/// its `verify` command, as `bower.toml` names them (EPIC-11 Decision 2).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Capture {
    Check,
    Verify,
}

impl Capture {
    /// Both captures, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [Capture; 2] {
        [Self::Check, Self::Verify]
    }

    /// Does the verifier run this capture's command at a step that claims
    /// `expect`? `check` runs unless the step is skipped; `verify` runs only
    /// when the check must pass. A fence the verifier could never fill is a
    /// typo, and `plan` says so (`BowerError::OutputNeverRuns`).
    #[must_use]
    pub fn runs_under(self, expect: Expect) -> bool {
        match self {
            Self::Check => expect != Expect::Skip,
            Self::Verify => matches!(expect, Expect::Pass | Expect::TestFail),
        }
    }
}

impl std::fmt::Display for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `pad`, not `write!`: the verify report aligns captures in a column.
        f.pad(match self {
            Self::Check => "check",
            Self::Verify => "verify",
        })
    }
}

impl std::str::FromStr for Capture {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "check" => Ok(Self::Check),
            "verify" => Ok(Self::Verify),
            other => Err(other.to_string()),
        }
    }
}
```

In `struct Directive`, after `exercise`:

```rust
    /// `output="check"|"verify"` — the fence holds what that command printed
    /// at the bound step, normalized. Never repo content (EPIC-11).
    pub output: Option<Capture>,
```

In `parse`, after the `"exercise" => …` arm:

```rust
                "output" => match value.parse::<Capture>() {
                    Ok(c) => d.output = Some(c),
                    Err(v) => errors.push(BowerError::BadValue {
                        loc: loc.clone(),
                        key: "output".to_string(),
                        value: v,
                    }),
                },
```

In `overlaid_on`, after the `exercise:` line:

```rust
            output: self.output.or(defaults.output),
```

- [ ] **Step 4: Add the five errors**

In `bower-core/src/lib.rs`, `enum BowerError`, after `ExerciseConflictingKeys`:

```rust
    /// An `output` block has no preceding step of its repo to attach to, and
    /// names none explicitly.
    OutputUnbound { loc: Location },
    /// An `output` block names a step that does not exist.
    OutputUnknownStep { loc: Location, step: String },
    /// An `output` block carries a key that belongs to another kind of block:
    /// a tree key, `notebook`, `exercise`, `expect`, or `include`.
    OutputConflictingKeys { loc: Location },
    /// A step already has an output block for this capture; reported at the
    /// second one.
    OutputDuplicate {
        loc: Location,
        step: String,
        capture: Capture,
    },
    /// The bound step never runs this capture's command: `verify` on a
    /// `compile_fail` step, or anything on a `none` step.
    OutputNeverRuns {
        loc: Location,
        step: String,
        capture: Capture,
        expect: Expect,
    },
```

Add `use crate::directive::{Capture, Expect};` beside `use crate::source::Location;`.

In `location()`, replace `| Self::ExerciseConflictingKeys { loc } => Some(loc),` with:

```rust
            | Self::ExerciseConflictingKeys { loc }
            | Self::OutputUnbound { loc }
            | Self::OutputUnknownStep { loc, .. }
            | Self::OutputConflictingKeys { loc }
            | Self::OutputDuplicate { loc, .. }
            | Self::OutputNeverRuns { loc, .. } => Some(loc),
```

In `Display`, after the `ExerciseConflictingKeys` arm:

```rust
            Self::OutputUnbound { loc } => {
                write!(f, "{loc}: output block has no preceding step to attach to")
            }
            Self::OutputUnknownStep { loc, step } => write!(
                f,
                "{loc}: output block names step `{step}`, which does not exist"
            ),
            Self::OutputConflictingKeys { loc } => write!(
                f,
                "{loc}: output block carries keys of another kind of block; it names only its repo, step, and capture"
            ),
            Self::OutputDuplicate { loc, step, capture } => write!(
                f,
                "{loc}: step `{step}` already has an output=\"{capture}\" block"
            ),
            Self::OutputNeverRuns {
                loc,
                step,
                capture,
                expect,
            } => write!(
                f,
                "{loc}: step `{step}` declares expect=\"{expect}\", so its `{capture}` command never runs"
            ),
```

In the prelude, change `pub use crate::directive::{Directive, Expect, Op};` to:

```rust
    pub use crate::directive::{Capture, Directive, Expect, Op};
```

- [ ] **Step 5: Run the directive tests**

Run: `cargo test -p bower-core directive_tests`
Expected: PASS.

- [ ] **Step 6: Write the failing block tests**

In `bower-core/src/block.rs`, `mod block_tests`:

```rust
    #[test]
    fn extract__output_block_is_inert() {
        let src = book(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"check\" -->\n```text\nerror: e\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        let b = &blocks[1];
        assert_eq!(b.output, Some(Capture::Check));
        assert_eq!(b.op, Op::Prose);
        assert_eq!(b.content.lines, vec!["error: e".to_string()]);
    }

    #[test]
    fn extract__output_needs_its_fence() {
        let src = book("<!-- bower repo=\"failers\" output=\"check\" -->\nprose instead\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(
            matches!(errors.0[0], BowerError::DirectiveWithoutBlock { .. }),
            "{errors}"
        );
    }

    #[test]
    fn extract__output_with_tree_keys_is_refused() {
        for keys in [
            "file=\"a.rs\"",
            "op=\"none\"",
            "expect=\"pass\"",
            "notebook=\"play\"",
            "exercise=\"Try\"",
        ] {
            let src = book(&format!(
                "<!-- bower repo=\"failers\" output=\"check\" {keys} -->\n```text\nx\n```\n"
            ));
            let (blocks, errors) = extract(&src, &catalog());
            assert!(blocks.is_empty(), "{keys}");
            assert!(
                errors
                    .0
                    .iter()
                    .any(|e| matches!(e, BowerError::OutputConflictingKeys { .. })),
                "{keys}: {errors}"
            );
        }
    }

    #[test]
    fn extract__output_with_include_is_refused() {
        // `--record` writes into the chapter, and a library entry is not one.
        let mut src = book("<!-- bower include=\"blocks/out.md\" output=\"check\" -->\n");
        src.library.insert(
            "blocks/out.md".to_string(),
            "<!-- bower repo=\"failers\" -->\n```text\nx\n```\n".to_string(),
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(
            matches!(errors.0[0], BowerError::OutputConflictingKeys { .. }),
            "{errors}"
        );
    }
```

`use super::*;` already brings `Capture` in once Step 8 imports it at the
top of `block.rs`.

- [ ] **Step 7: Run them to see them fail**

Run: `cargo test -p bower-core block_tests`
Expected: compile error, `no field `output` on type `Block``.

- [ ] **Step 8: Add `Block::output` and classify the block**

In `bower-core/src/block.rs`, change the import to
`use crate::directive::{Capture, Directive, Expect, Op};` and add to `struct Block`, after `exercise_block`:

```rust
    /// `output="…"`: the fence records what that command printed at the bound
    /// step. Like a play cell it never joins a step or touches a tree;
    /// [`crate::plan`] binds it (EPIC-11).
    pub output: Option<Capture>,
```

In `resolve`, add as the first statement of the body:

```rust
    // Read before the include is resolved: the overlay clears it.
    let included = directive.include.is_some();
```

Replace

```rust
    let play = directive.notebook.is_some();
    let tree_keys = carries_tree_keys(&directive);
    let exercise_block = directive.exercise.is_some() && !tree_keys && !play;
    let op = directive.op.unwrap_or_default();

    if play {
```

with

```rust
    let play = directive.notebook.is_some();
    let tree_keys = carries_tree_keys(&directive);
    let output = directive.output;
    let exercise_block =
        directive.exercise.is_some() && !tree_keys && !play && output.is_none();
    let op = directive.op.unwrap_or_default();

    if output.is_some() {
        // An output block quotes what a command printed at a step. It names
        // its step and nothing else: not a tree, a cell, an exercise, or a
        // claim — and not an include, because `--record` writes into the
        // chapter and a library entry is not one.
        if tree_keys
            || play
            || directive.exercise.is_some()
            || directive.expect.is_some()
            || included
        {
            errors.push(BowerError::OutputConflictingKeys { loc: loc.clone() });
        }
        if content.is_none() {
            errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
        }
    } else if play {
```

In the `Block { … }` literal at the end of `resolve`, the `op` field becomes:

```rust
        // Neither a play cell, a block-form exercise, nor an output applies
        // to a tree; Prose is the inert op.
        op: if play || exercise_block || output.is_some() {
            Op::Prose
        } else {
            op
        },
```

and add after `exercise_block,`:

```rust
        output,
```

- [ ] **Step 9: Fix the three test-only `Block` literals**

In `bower-core/src/display.rs` (near line 280), `bower-core/src/tree.rs`
(near line 410), and `bower-core/src/step.rs` (near line 311), add after
`exercise_block: false,`:

```rust
            output: None,
```

- [ ] **Step 10: Run the kernel tests**

Run: `cargo test -p bower-core`
Expected: PASS, including the four new block tests.

- [ ] **Step 11: Commit**

```bash
git add bower-core/src
git commit -m "feat(core): the output key, its five errors, and the inert output block"
```

---
### Task 4: Binding outputs to steps, and the lock (EPIC Phase 1c–1d, Decisions 3, 8, 9)

**Files:**
- Create: `bower-core/src/capture.rs` (only `tidy` for now)
- Modify: `bower-core/src/lib.rs` (`pub mod capture;`, prelude)
- Modify: `bower-core/src/plan.rs` (`CapturedOutput`, `PlannedStep::outputs`, partition, `bind_outputs`, `lock_text`, tests)

**Interfaces:**
- Consumes: `Capture`, `Block::output`, the five `Output*` errors (Task 3).
- Produces:
  - `pub fn capture::tidy(lines: &[String]) -> Vec<String>` — rule 8: trailing whitespace off every line, blank lines off both ends.
  - `pub struct CapturedOutput { pub loc: Location, pub capture: Capture, pub info: String, pub lines: Vec<String> }` (derives `Clone, Debug, Eq, PartialEq`). `lines` is the fence body after `tidy`; empty means "not recorded yet".
  - `PlannedStep::outputs: Vec<CapturedOutput>`, in document order.

- [ ] **Step 1: Write the failing `tidy` test**

Create `bower-core/src/capture.rs`:

```rust
//! What a command printed, made comparable (EPIC-11).
//!
//! Pure string functions: the edge runs the command and hands the text in.
//! Nothing here knows about cargo beyond the shape of its output lines.

/// Rule 8, shared by both sides of every comparison: trailing whitespace off
/// each line, blank lines off both ends. A recorded fence goes through this at
/// plan time, so an editor that strips trailing spaces changes nothing.
#[must_use]
pub fn tidy(lines: &[String]) -> Vec<String> {
    let _ = lines;
    Vec::new()
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod capture_tests {
    use super::*;

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn tidy__trims_line_ends_and_outer_blanks() {
        assert_eq!(
            tidy(&v(&["", "  ", "a  ", "", "b\t", ""])),
            v(&["a", "", "b"])
        );
        assert!(tidy(&v(&["", " "])).is_empty());
    }
}
```

In `bower-core/src/lib.rs`, add `pub mod capture;` above `pub mod directive;`.

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p bower-core tidy__`
Expected: FAIL (`left: []`).

- [ ] **Step 3: Implement `tidy`**

```rust
#[must_use]
pub fn tidy(lines: &[String]) -> Vec<String> {
    let trimmed: Vec<String> = lines.iter().map(|l| l.trim_end().to_string()).collect();
    let start = trimmed
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(trimmed.len());
    let end = trimmed
        .iter()
        .rposition(|l| !l.is_empty())
        .map_or(start, |i| i + 1);
    trimmed[start..end].to_vec()
}
```

Run: `cargo test -p bower-core tidy__` — PASS.

- [ ] **Step 4: Write the failing plan tests**

In `bower-core/src/plan.rs`, `mod plan_tests`, add (the line numbers in the
comments are what the assertions rely on):

```rust
    fn one(text: &str) -> BookSource {
        BookSource::from_chapters(vec![Chapter::new("ch.md", text)])
    }

    fn output_chapter() -> Chapter {
        Chapter::new(
            "ch03-outputs.md",
            concat!(
                "# Outputs\n\n",                                                                                   // 1-2
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"broken\" expect=\"compile_fail\" -->\n", // 3
                "```rust\nfn x() -> u32 { \"42\" }\n```\n\n",                                                  // 4-7
                "<!-- bower repo=\"failers\" output=\"check\" -->\n",                                            // 8
                "```text\nerror[E0308]: mismatched types   \n[...]\n\n```\n\n",                                  // 9-14
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"fixed\" -->\n",           // 15
                "```rust\nfn x() -> u32 { 42 }\n```\n\n",                                                      // 16-19
                "<!-- bower repo=\"failers\" output=\"check\" step=\"fixed\" -->\n",                             // 20
                "```text\n```\n",                                                                              // 21-22
            ),
        )
    }

    fn output_plan() -> BookPlan {
        plan(&BookSource::from_chapters(vec![output_chapter()]), &catalog()).unwrap()
    }

    #[test]
    fn plan__output_binds_to_the_nearest_preceding_step() {
        let p = output_plan();
        let o = &p.repo("failers").unwrap().steps[0].outputs[0];
        assert_eq!(o.capture, Capture::Check);
        assert_eq!(o.loc, Location::new("ch03-outputs.md", 8));
        assert_eq!(o.info, "text");
        // Tidied: the trailing spaces and the trailing blank line are gone.
        assert_eq!(
            o.lines,
            vec!["error[E0308]: mismatched types".to_string(), "[...]".to_string()]
        );
    }

    #[test]
    fn plan__output_binds_explicitly_by_step() {
        let p = output_plan();
        let fixed = &p.repo("failers").unwrap().steps[1];
        assert_eq!(fixed.outputs.len(), 1);
        assert_eq!(fixed.outputs[0].loc.line, 20);
        assert!(fixed.outputs[0].lines.is_empty(), "an empty fence is not recorded yet");
    }

    #[test]
    fn plan__output_never_touches_a_tree() {
        let p = output_plan();
        let steps = &p.repo("failers").unwrap().steps;
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].files, vec!["src/lib.rs".to_string()]);
        assert_eq!(
            steps[0].tree.text("src/lib.rs").unwrap(),
            "fn x() -> u32 { \"42\" }\n"
        );
    }

    #[test]
    fn plan__verify_output_on_compile_fail_never_runs() {
        let book = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" expect=\"compile_fail\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" output=\"verify\" -->\n```text\n```\n",
        ));
        let errs = plan(&book, &catalog()).unwrap_err();
        assert!(
            matches!(
                &errs.0[0],
                BowerError::OutputNeverRuns { loc, capture: Capture::Verify, expect: Expect::CompileFail, .. }
                    if loc.line == 5
            ),
            "{errs}"
        );
    }

    #[test]
    fn plan__output_on_a_none_step_never_runs() {
        let book = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" op=\"none\" step=\"talk\" expect=\"none\" msg=\"m\" -->\n",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
        ));
        let errs = plan(&book, &catalog()).unwrap_err();
        assert!(
            matches!(
                &errs.0[0],
                BowerError::OutputNeverRuns { step, capture: Capture::Check, expect: Expect::Skip, .. }
                    if step == "talk"
            ),
            "{errs}"
        );
    }

    #[test]
    fn plan__duplicate_capture_is_reported_at_the_second() {
        let book = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
        ));
        let errs = plan(&book, &catalog()).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs}");
        assert!(
            matches!(&errs.0[0], BowerError::OutputDuplicate { loc, capture: Capture::Check, .. } if loc.line == 8),
            "{errs}"
        );
    }

    #[test]
    fn plan__output_binding_errors_mirror_play_cells() {
        let unbound = one(concat!(
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ));
        let errs = plan(&unbound, &catalog()).unwrap_err();
        assert!(matches!(errs.0[0], BowerError::OutputUnbound { .. }), "{errs}");

        let ghost = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" output=\"check\" step=\"ghost\" -->\n```text\n```\n",
        ));
        let errs = plan(&ghost, &catalog()).unwrap_err();
        assert!(
            matches!(&errs.0[0], BowerError::OutputUnknownStep { step, .. } if step == "ghost"),
            "{errs}"
        );
    }

    #[test]
    fn lock_text__records_outputs_line_by_line() {
        let lock = lock_text(&output_plan());
        assert!(
            lock.contains(concat!(
                "    output check at=ch03-outputs.md:8\n",
                "    > error[E0308]: mismatched types\n",
                "    > [...]\n",
            )),
            "{lock}"
        );
        assert!(lock.contains("    output check at=ch03-outputs.md:20\n"), "{lock}");
    }
```

Add `use crate::directive::Capture;` to the test module's imports if
`use super::*;` does not already bring it (Step 6 imports it at the top of
`plan.rs`, which makes `super::*` enough).

- [ ] **Step 5: Run them to see them fail**

Run: `cargo test -p bower-core plan_tests`
Expected: compile error, `no field `outputs` on type `PlannedStep``.

- [ ] **Step 6: Add `CapturedOutput` and `PlannedStep::outputs`**

In `bower-core/src/plan.rs`, change the imports:

```rust
use crate::block::Block;
use crate::capture;
use crate::directive::{Capture, Expect, Op};
```

In `struct PlannedStep`, after `exercise`:

```rust
    /// What the compiler said at this step, as the book records it: every
    /// `output="…"` block bound here, in document order (EPIC-11).
    pub outputs: Vec<CapturedOutput>,
```

After `struct PlayCell`:

```rust
/// One `output="…"` block, bound to a step. Never part of any tree, so
/// recording one moves no SHA (EPIC-11 Decision 9).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedOutput {
    /// The directive: where errors point and where `--record` writes.
    pub loc: Location,
    pub capture: Capture,
    /// The author's fence info string, kept on rewrite.
    pub info: String,
    /// The fence body, tidied (`capture::tidy`). Empty means "not recorded
    /// yet".
    pub lines: Vec<String>,
}
```

In `plan()`, replace the two `partition` lines with:

```rust
    let (play_blocks, rest): (Vec<_>, Vec<_>) = blocks.into_iter().partition(|b| b.play);
    let (output_blocks, rest): (Vec<_>, Vec<_>) =
        rest.into_iter().partition(|b| b.output.is_some());
    let (exercise_blocks, code_blocks): (Vec<_>, Vec<_>) =
        rest.into_iter().partition(|b| b.exercise_block);
```

In the `PlannedStep { … }` literal, after `exercise: None,`:

```rust
                outputs: Vec::new(),
```

After the `bind_exercises(…);` call:

```rust
        bind_outputs(&output_blocks, name, &index, &mut planned, &mut errors);
```

- [ ] **Step 7: Implement `bind_outputs`**

After `fn bind_exercises`:

```rust
/// Attach every output block of `repo` to its step, with the play-cell rule:
/// an explicit `step=`, else the nearest preceding code block. Blocks arrive in
/// document order, so a duplicate is reported at the second one. A capture the
/// verifier would never run at that step is refused here, at plan time: a
/// fence nothing could fill is a typo.
fn bind_outputs(
    output_blocks: &[Block],
    repo: &RepoName,
    index: &StepIndex,
    planned: &mut [PlannedStep],
    errors: &mut Errors,
) {
    for b in output_blocks.iter().filter(|b| &b.repo == repo) {
        let Some(capture) = b.output else { continue };
        let idx = match index.locate(b.step.as_deref(), b.seq_in_book) {
            Ok(idx) => idx,
            Err(Unbindable::UnknownStep(step)) => {
                errors.push(BowerError::OutputUnknownStep {
                    loc: b.loc.clone(),
                    step,
                });
                continue;
            }
            Err(Unbindable::NoPrecedingStep) => {
                errors.push(BowerError::OutputUnbound { loc: b.loc.clone() });
                continue;
            }
        };
        let step = &mut planned[idx];
        if !capture.runs_under(step.expect) {
            errors.push(BowerError::OutputNeverRuns {
                loc: b.loc.clone(),
                step: step.id.0.clone(),
                capture,
                expect: step.expect,
            });
            continue;
        }
        if step.outputs.iter().any(|o| o.capture == capture) {
            errors.push(BowerError::OutputDuplicate {
                loc: b.loc.clone(),
                step: step.id.0.clone(),
                capture,
            });
            continue;
        }
        step.outputs.push(CapturedOutput {
            loc: b.loc.clone(),
            capture,
            info: b.content.info.clone(),
            lines: capture::tidy(&b.content.lines),
        });
    }
}
```

- [ ] **Step 8: Print outputs in the lock**

In `lock_text`, after the `if let Some(x) = &s.exercise { … }` block:

```rust
            // Every recorded line, so a rewording shows up in review as a
            // diff of the lock as well as of the chapter.
            for o in &s.outputs {
                let _ = writeln!(out, "    output {} at={}", o.capture, o.loc);
                for line in &o.lines {
                    let _ = writeln!(out, "    > {line}");
                }
            }
```

- [ ] **Step 9: Export the new types**

In the prelude in `bower-core/src/lib.rs`, the plan line becomes:

```rust
    pub use crate::plan::{
        BookPlan, CapturedOutput, Exercise, ExerciseForm, PlannedStep, PlayCell, RepoPlan,
        lock_text, plan,
    };
```

- [ ] **Step 10: Run the tests**

Run: `cargo test -p bower-core && cargo test -p bower-testkit && cargo test -p bower`
Expected: PASS. No existing lock changes: `cargo run -q -p bower -- --book books/hello-playbook plan && git diff --stat books/`
prints nothing.

- [ ] **Step 11: Commit**

```bash
git add bower-core/src
git commit -m "feat(core): bind output blocks to steps and record them in the lock"
```

---
### Task 5: `normalize` — eight rules, one test each (EPIC Phase 2a, Decisions 4 and 13)

**Files:**
- Modify: `bower-core/src/capture.rs`
- Modify: `bower-core/src/lib.rs` (prelude)

**Interfaces:**
- Consumes: `tidy` (Task 4).
- Produces:
  - `#[derive(Clone, Debug, Default, Eq, PartialEq)] pub struct Scrub(pub Vec<(String, String)>);` — `(prefix, replacement)` pairs; applied longest prefix first; an empty prefix is ignored.
  - `pub fn normalize(raw: &str, scrub: &Scrub) -> Vec<String>` — pure, idempotent.

The rules run in the EPIC's table order. Each step below adds one failing test,
then the code that passes it. All tests go in `mod capture_tests`, and all use
this helper (add it once, beside `v`):

```rust
    fn norm(raw: &str) -> Vec<String> {
        normalize(raw, &Scrub::default())
    }
```

- [ ] **Step 1: The skeleton, and silence**

Add to `capture.rs`, above `tidy`:

```rust
/// Machine-specific path prefixes and what replaces each: the scratch tree,
/// the shared target directory, cargo's home. The edge knows them; the kernel
/// only replaces text (EPIC-11 Decision 4). Applied longest prefix first, so
/// `/w/tree/` is replaced before `/w/tree`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Scrub(pub Vec<(String, String)>);

/// Reduce what a command printed to the lines that depend only on what the
/// compiler said — not the machine, path, clock, terminal, or scheduling.
/// Pure and idempotent: normalizing normalized text changes nothing.
#[must_use]
pub fn normalize(raw: &str, scrub: &Scrub) -> Vec<String> {
    let _ = scrub;
    let lines: Vec<String> = raw.lines().map(str::to_string).collect();
    tidy(&lines)
}
```

Test:

```rust
    #[test]
    fn normalize__is_empty_for_silence() {
        assert!(norm("").is_empty());
        assert!(norm("\n\n").is_empty());
    }
```

Run: `cargo test -p bower-core normalize__` — PASS.

- [ ] **Step 2: Rule 1 — CRLF**

Test:

```rust
    #[test]
    fn normalize__folds_crlf() {
        // A Windows runner must record what a Mac records.
        assert_eq!(norm("a\r\nb\r\n"), v(&["a", "b"]));
    }
```

It already passes: `str::lines` strips a `\r` before `\n`, and `tidy` trims a
stray one. Make the rule explicit anyway, as the first line of `normalize`:

```rust
    let text = raw.replace("\r\n", "\n"); // rule 1
```

and split `text` rather than `raw`. Run — PASS.

- [ ] **Step 3: Rule 2 — ANSI escapes**

Test:

```rust
    #[test]
    fn normalize__strips_ansi() {
        // CARGO_TERM_COLOR=always in someone's shell. The second line is an
        // OSC 8 hyperlink, which cargo emits on terminals that support one.
        let raw = "\x1b[1m\x1b[91merror\x1b[0m: x\n\x1b]8;;http://x.invalid\x07link\x1b]8;;\x07\n";
        assert_eq!(norm(raw), v(&["error: x", "link"]));
    }
```

Run it: FAIL. Implement:

```rust
/// Rule 2. A CSI sequence (`ESC [ … final byte`) and an OSC sequence
/// (`ESC ] … BEL` or `ESC ] … ESC \`) go whole; any other escape character
/// goes alone. No escape character survives, so a second pass finds nothing.
/// Hand-written: the kernel takes no `regex`.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                // Parameters and intermediates, then one final byte in `@`..=`~`.
                for n in chars.by_ref() {
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                while let Some(n) = chars.next() {
                    if n == '\x07' {
                        break;
                    }
                    if n == '\x1b' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}
```

and in `normalize`: `let text = strip_ansi(&text); // rule 2`. Run — PASS.

- [ ] **Step 4: Rule 3 — scrub the machine's paths**

Tests:

```rust
    #[test]
    fn normalize__scrubs_the_tree_prefix() {
        // Given in the wrong order on purpose: longest first is the
        // function's job, not the caller's.
        let scrub = Scrub(vec![
            ("/w/tree".to_string(), ".".to_string()),
            ("/w/tree/".to_string(), String::new()),
        ]);
        assert_eq!(
            normalize("at `/w/tree/Cargo.toml` in /w/tree", &scrub),
            v(&["at `Cargo.toml` in ."])
        );
    }

    #[test]
    fn normalize__never_scrubs_an_empty_prefix() {
        // `str::replace("", …)` inserts between every character.
        let scrub = Scrub(vec![(String::new(), "X".to_string())]);
        assert_eq!(normalize("abc", &scrub), v(&["abc"]));
    }
```

Run: FAIL. Implement:

```rust
/// Rule 3. Longest prefix first, so a path that contains another is never cut
/// in half.
fn scrub_paths(text: &str, scrub: &Scrub) -> String {
    let mut pairs: Vec<&(String, String)> =
        scrub.0.iter().filter(|(from, _)| !from.is_empty()).collect();
    pairs.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));
    pairs
        .iter()
        .fold(text.to_string(), |t, (from, to)| t.replace(from.as_str(), to))
}
```

and `let text = scrub_paths(&text, scrub); // rule 3` (remove the `let _ = scrub;`). Run — PASS.

- [ ] **Step 5: Rule 4 — cargo's status lines**

Tests:

```rust
    #[test]
    fn normalize__drops_cargo_status_lines() {
        let raw = concat!(
            "   Compiling a v0.1.0 (/x)\n",
            "    Checking a v0.1.0 (/x)\n",
            "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.31s\n",
            "     Running unittests src/lib.rs (/x/deps/a-08506271f5924bd7)\n",
            "   Doc-tests a\n",
            "    Blocking waiting for file lock on build directory\n",
            "error: real\n",
        );
        assert_eq!(norm(raw), v(&["error: real"]));
    }

    #[test]
    fn normalize__keeps_a_verb_that_is_not_right_aligned() {
        // Only cargo indents its status verbs. A program's own line that
        // starts with `Running` is the program's.
        assert_eq!(norm("Running the numbers\n"), v(&["Running the numbers"]));
    }
```

Run: FAIL. Implement:

```rust
/// The verbs cargo right-aligns in its status column. All are shorter than the
/// column, so a status line always starts with a space.
const STATUS_VERBS: &[&str] = &[
    "Compiling",
    "Checking",
    "Finished",
    "Running",
    "Doc-tests",
    "Blocking",
    "Locking",
    "Updating",
    "Downloading",
    "Downloaded",
    "Adding",
    "Fresh",
];

/// Rule 4: build chatter that varies with the cache, and a `Running` line
/// that names a hashed binary.
fn is_cargo_status(line: &str) -> bool {
    let t = line.trim_start();
    t.len() < line.len()
        && STATUS_VERBS
            .iter()
            .any(|v| t.strip_prefix(v).is_some_and(|rest| rest.starts_with(' ')))
}
```

In `normalize`, the split becomes:

```rust
    let lines: Vec<String> = text
        .lines()
        .filter(|l| !is_cargo_status(l)) // rule 4
        .map(str::to_string)
        .collect();
```

Run — PASS.

- [ ] **Step 6: Cargo's summary lines stay**

Test (passes already — it pins open question 2's answer):

```rust
    #[test]
    fn normalize__keeps_cargo_summary_lines() {
        // A reader sees these, and they do not vary between runs.
        let raw = concat!(
            "error[E0308]: mismatched types\n",
            "error: could not compile `a` (lib) due to 1 previous error\n",
            "warning: `a` (lib) generated 1 warning\n",
            "error: test failed, to rerun pass `--lib`\n",
        );
        assert_eq!(norm(raw).len(), 4);
    }
```

- [ ] **Step 7: Rule 5 — the panicking thread's ID**

Test:

```rust
    #[test]
    fn normalize__removes_the_panicking_thread_id() {
        assert_eq!(
            norm("thread 'tests::a' (691424) panicked at src/lib.rs:31:9:"),
            v(&["thread 'tests::a' panicked at src/lib.rs:31:9:"])
        );
        // A thread with no ID (before Rust 1.91) is left alone.
        assert_eq!(
            norm("thread 'main' panicked at src/main.rs:1:1:"),
            v(&["thread 'main' panicked at src/main.rs:1:1:"])
        );
    }
```

Run: FAIL. Implement:

```rust
/// Rule 5: `thread 'name' (691424) panicked at` → `thread 'name' panicked at`.
/// The number is an OS thread ID, new every run (EPIC-11 Decision 13). Only a
/// parenthesized number directly before `panicked at` is removed, which keeps
/// the rule idempotent.
fn drop_thread_id(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("thread '")
        && let Some(close) = rest.find("' (")
    {
        let after = &rest[close + 3..];
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && after[digits..].starts_with(") panicked at") {
            return format!("thread '{}'{}", &rest[..close], &after[digits + 1..]);
        }
    }
    line.to_string()
}
```

In `normalize`, change `.map(str::to_string)` to `.map(drop_thread_id) // rule 5`. Run — PASS.

- [ ] **Step 8: Rule 6 — the clock**

Test:

```rust
    #[test]
    fn normalize__strips_test_timings() {
        assert_eq!(
            norm("test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s"),
            v(&["test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"])
        );
    }
```

Run: FAIL. Implement:

```rust
/// Rule 6: `; finished in 0.42s` off a libtest summary.
fn strip_test_timing(line: &str) -> String {
    if line.starts_with("test result: ")
        && let Some(at) = line.find("; finished in ")
    {
        return line[..at].to_string();
    }
    line.to_string()
}
```

In `normalize`, after the rule-5 `map`, add
`.map(|l| strip_test_timing(&l)) // rule 6`. Run — PASS.

- [ ] **Step 9: Rule 7 — parallel test lines**

Test:

```rust
    #[test]
    fn normalize__sorts_parallel_test_lines() {
        // libtest reports tests as they finish. `verify` runs a recorded step
        // on one thread; this rule serves output recorded anywhere else.
        let raw = "running 3 tests\ntest c ... ok\ntest a ... FAILED\ntest b ... ok\n\ntest result: x\n";
        assert_eq!(
            norm(raw),
            v(&["running 3 tests", "test a ... FAILED", "test b ... ok", "test c ... ok", "", "test result: x"])
        );
    }
```

Run: FAIL. Implement:

```rust
/// One libtest progress line: `test <name> ... <result>`.
fn is_test_line(line: &str) -> bool {
    line.starts_with("test ") && line.contains(" ... ")
}

/// Rule 7: sort each contiguous run of test lines.
fn sort_test_runs(lines: &mut [String]) {
    let mut i = 0;
    while i < lines.len() {
        if !is_test_line(&lines[i]) {
            i += 1;
            continue;
        }
        let end = (i..lines.len())
            .find(|&k| !is_test_line(&lines[k]))
            .unwrap_or(lines.len());
        lines[i..end].sort();
        i = end;
    }
}
```

The final `normalize`:

```rust
#[must_use]
pub fn normalize(raw: &str, scrub: &Scrub) -> Vec<String> {
    let text = raw.replace("\r\n", "\n"); // rule 1
    let text = strip_ansi(&text); // rule 2
    let text = scrub_paths(&text, scrub); // rule 3
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| !is_cargo_status(l)) // rule 4
        .map(drop_thread_id) // rule 5
        .map(|l| strip_test_timing(&l)) // rule 6
        // Trimmed before sorting, so trailing blanks cannot change an order.
        .map(|l| l.trim_end().to_string())
        .collect();
    sort_test_runs(&mut lines); // rule 7
    tidy(&lines) // rule 8
}
```

Run — PASS.

- [ ] **Step 10: Rule 8, stated**

Test (passes already):

```rust
    #[test]
    fn normalize__trims_line_ends_and_outer_blanks() {
        // A snapshot must survive an editor.
        assert_eq!(norm("\n\nerror: x   \n\n"), v(&["error: x"]));
    }
```

- [ ] **Step 11: Export, and check purity**

In the prelude:

```rust
    pub use crate::capture::{Scrub, normalize, tidy};
```

Run: `cargo test -p bower-core && make purity`
Expected: PASS; `purity: bower-core has no dependencies`.

- [ ] **Step 12: Commit**

```bash
git add bower-core/src
git commit -m "feat(core): normalize command output, one rule per test"
```

---
### Task 6: `drift` with `[...]`, and `error_codes` (EPIC Phase 2b–2c, Decisions 5 and 11)

**Files:**
- Modify: `bower-core/src/capture.rs`
- Modify: `bower-core/src/lib.rs` (prelude)

**Interfaces:**
- Produces:
  - `pub const ELISION: &str = "[...]";`
  - `#[derive(Clone, Debug, Eq, PartialEq)] pub struct Drift { pub line: usize, pub recorded: Option<String>, pub live: Option<String> }` — `line` is 1-based within the recorded (tidied) lines.
  - `pub fn drift(recorded: &[String], live: &[String]) -> Option<Drift>` — `None` means the recording holds.
  - `pub fn error_codes(lines: &[String]) -> Vec<String>` — `"E0004"`-style codes from `error[…]`/`warning[…]` headlines, first-seen order, deduplicated.

- [ ] **Step 1: Write the failing exact-match tests**

In `mod capture_tests`:

```rust
    #[test]
    fn drift__none_when_equal() {
        assert_eq!(drift(&v(&["a", "b"]), &v(&["a", "b"])), None);
    }

    #[test]
    fn drift__names_the_first_differing_line() {
        assert_eq!(
            drift(&v(&["a", "b", "c"]), &v(&["a", "x", "c"])),
            Some(Drift { line: 2, recorded: Some("b".into()), live: Some("x".into()) })
        );
    }

    #[test]
    fn drift__a_shorter_live_output_is_drift() {
        assert_eq!(
            drift(&v(&["a", "b"]), &v(&["a"])),
            Some(Drift { line: 2, recorded: Some("b".into()), live: None })
        );
    }

    #[test]
    fn drift__a_longer_live_output_is_drift() {
        // A new note at the end is a rewording too. Only `[...]` excuses it.
        assert_eq!(
            drift(&v(&["a"]), &v(&["a", "b"])),
            Some(Drift { line: 2, recorded: None, live: Some("b".into()) })
        );
    }
```

Run: `cargo test -p bower-core drift__` — compile error, `cannot find function `drift``.

- [ ] **Step 2: Implement exact matching**

Add to `capture.rs`:

```rust
/// A recorded line that is exactly this stands for zero or more live lines —
/// the editorial ellipsis of any quotation. Not `...`: rustc prints that in
/// the gutter of a long multi-line span (EPIC-11 Decision 5).
pub const ELISION: &str = "[...]";

/// The first place a recording and the live output disagree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Drift {
    /// 1-based, within the recorded lines.
    pub line: usize,
    /// The recorded line there; `None` when the recording has ended.
    pub recorded: Option<String>,
    /// The live line there; `None` when the output has ended, or when an
    /// unanchored piece appears nowhere after the piece before it.
    pub live: Option<String>,
}

/// Does the recording hold against the live output?
///
/// Without a `[...]` line the two must be equal. With one, the recording is
/// split into pieces at each `[...]`; every piece must appear as consecutive
/// live lines, in order. A piece with no `[...]` before it must start the
/// output, and one with none after it must end it. An empty recording is not
/// judged here: the caller reports it as not recorded yet.
#[must_use]
pub fn drift(recorded: &[String], live: &[String]) -> Option<Drift> {
    if !recorded.iter().any(|l| l == ELISION) {
        return compare_at(recorded, 0, live, 0, recorded.len().max(live.len()));
    }
    None
}

/// Compare `piece` (which starts at recorded index `start`) against `live`
/// from index `at`, for `len` lines — longer than the piece when the live
/// output must also end where it does.
fn compare_at(
    piece: &[String],
    start: usize,
    live: &[String],
    at: usize,
    len: usize,
) -> Option<Drift> {
    (0..len).find_map(|i| {
        let r = piece.get(i);
        let l = live.get(at + i);
        (r != l).then(|| Drift {
            line: start + i + 1,
            recorded: r.cloned(),
            live: l.cloned(),
        })
    })
}
```

Run: `cargo test -p bower-core drift__` — PASS.

- [ ] **Step 3: Write the failing elision tests**

```rust
    #[test]
    fn drift__elision_matches_a_middle_piece() {
        let live = v(&["w1", "w2", "thread panicked", "left: 1", "right: 2", "summary"]);
        let recorded = v(&[ELISION, "thread panicked", "left: 1", "right: 2", ELISION]);
        assert_eq!(drift(&recorded, &live), None);
    }

    #[test]
    fn drift__elision_at_neither_end_anchors_both() {
        let live = v(&["head", "middle", "tail"]);
        assert_eq!(drift(&v(&["head", ELISION, "tail"]), &live), None);
        assert_eq!(drift(&v(&["head", ELISION]), &live), None);
        assert_eq!(drift(&v(&[ELISION, "tail"]), &live), None);
        // Anchored: `middle` is neither the first line nor the last.
        assert_eq!(
            drift(&v(&["middle", ELISION]), &live),
            Some(Drift { line: 1, recorded: Some("middle".into()), live: Some("head".into()) })
        );
        assert_eq!(
            drift(&v(&[ELISION, "middle"]), &live),
            Some(Drift { line: 2, recorded: Some("middle".into()), live: Some("tail".into()) })
        );
    }

    #[test]
    fn drift__elided_pieces_must_keep_their_order() {
        let live = v(&["a", "b", "c"]);
        assert_eq!(drift(&v(&[ELISION, "a", ELISION, "c", ELISION]), &live), None);
        assert_eq!(
            drift(&v(&[ELISION, "c", ELISION, "a", ELISION]), &live),
            Some(Drift { line: 4, recorded: Some("a".into()), live: None })
        );
    }

    #[test]
    fn drift__an_elided_piece_that_is_gone_names_its_first_line() {
        assert_eq!(
            drift(&v(&[ELISION, "z", "y", ELISION]), &v(&["a", "b"])),
            Some(Drift { line: 2, recorded: Some("z".into()), live: None })
        );
    }

    #[test]
    fn drift__an_elision_may_stand_for_no_lines() {
        assert_eq!(drift(&v(&["a", ELISION, "b"]), &v(&["a", "b"])), None);
        assert_eq!(drift(&v(&[ELISION]), &v(&["x"])), None);
    }

    #[test]
    fn drift__anchored_ends_may_not_overlap() {
        // `a [...] a` against the single line `a`: both ends cannot claim it.
        assert_eq!(
            drift(&v(&["a", ELISION, "a"]), &v(&["a"])),
            Some(Drift { line: 3, recorded: Some("a".into()), live: None })
        );
    }
```

Run: FAIL (several return `None` where drift is expected).

- [ ] **Step 4: Implement piece matching**

Replace the body of `drift` with:

```rust
#[must_use]
pub fn drift(recorded: &[String], live: &[String]) -> Option<Drift> {
    if !recorded.iter().any(|l| l == ELISION) {
        return compare_at(recorded, 0, live, 0, recorded.len().max(live.len()));
    }

    // (index of the piece's first line in `recorded`, the piece)
    let mut pieces: Vec<(usize, &[String])> = Vec::new();
    let mut start = 0;
    for (i, line) in recorded.iter().enumerate() {
        if line == ELISION {
            if i > start {
                pieces.push((start, &recorded[start..i]));
            }
            start = i + 1;
        }
    }
    if start < recorded.len() {
        pieces.push((start, &recorded[start..]));
    }

    let anchored_start = recorded.first().is_some_and(|l| l != ELISION);
    let anchored_end = recorded.last().is_some_and(|l| l != ELISION);
    let last = pieces.len().saturating_sub(1);
    let mut pos = 0;

    for (k, &(start, piece)) in pieces.iter().enumerate() {
        if k == 0 && anchored_start {
            if let Some(d) = compare_at(piece, start, live, 0, piece.len()) {
                return Some(d);
            }
            pos = piece.len();
        } else if k == last && anchored_end {
            if live.len() < pos + piece.len() {
                return Some(Drift {
                    line: start + 1,
                    recorded: Some(piece[0].clone()),
                    live: None,
                });
            }
            let at = live.len() - piece.len();
            if let Some(d) = compare_at(piece, start, live, at, piece.len()) {
                return Some(d);
            }
            pos = live.len();
        } else {
            let found = (pos..=live.len().saturating_sub(piece.len()))
                .find(|&at| live.get(at..at + piece.len()) == Some(piece));
            let Some(at) = found else {
                return Some(Drift {
                    line: start + 1,
                    recorded: Some(piece[0].clone()),
                    live: None,
                });
            };
            pos = at + piece.len();
        }
    }
    None
}
```

Leftmost search is correct for "appears, in order": taking the earliest
occurrence of each piece leaves the most room for the pieces after it.

Run: `cargo test -p bower-core drift__` — PASS.

- [ ] **Step 5: Write the failing `error_codes` tests**

```rust
    #[test]
    fn error_codes__in_order_once_each() {
        let lines = v(&[
            "error[E0308]: mismatched types",
            "  = note: see E0004",
            "error[E0004]: non-exhaustive patterns",
            "error[E0308]: again",
            "warning: unused variable",
            "For more information about this error, try `rustc --explain E0308`.",
        ]);
        assert_eq!(error_codes(&lines), vec!["E0308", "E0004"]);
    }

    #[test]
    fn error_codes__only_headline_codes_count() {
        let lines = v(&["error[E12]: short", "error[X0001]: not rustc", "error: plain", "error[E0004"]);
        assert!(error_codes(&lines).is_empty());
    }
```

Run: compile error.

- [ ] **Step 6: Implement `error_codes`**

```rust
/// The rustc error codes an output names, from its `error[E0004]` and
/// `warning[E0004]` headlines: first-seen order, each once. The one parser
/// both the render caption and EPIC-12's failure index use.
#[must_use]
pub fn error_codes(lines: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("error[").or_else(|| t.strip_prefix("warning[")) else {
            continue;
        };
        let Some(end) = rest.find(']') else { continue };
        let code = &rest[..end];
        let is_code = code.len() == 5
            && code.starts_with('E')
            && code[1..].bytes().all(|b| b.is_ascii_digit());
        if is_code && !out.iter().any(|c| c == code) {
            out.push(code.to_string());
        }
    }
    out
}
```

Prelude: `pub use crate::capture::{Drift, ELISION, Scrub, drift, error_codes, normalize, tidy};`

Run: `cargo test -p bower-core` — PASS.

- [ ] **Step 7: Commit**

```bash
git add bower-core/src
git commit -m "feat(core): drift with [...] elision, and one error-code parser"
```

---

### Task 7: `rewrite`, on the one fence scanner (EPIC Phase 0c and 2c)

**Files:**
- Modify: `bower-core/src/block.rs` (`fence_width` to `pub(crate)`; new `fence_span`; `capture_block` uses it)
- Modify: `bower-core/src/capture.rs` (`rewrite`)
- Modify: `bower-core/src/lib.rs` (prelude)

**Interfaces:**
- Produces:
  - `pub(crate) fn block::fence_width(line: &str) -> Option<usize>` (unchanged body).
  - `pub(crate) fn block::fence_span(lines: &[&str], from: usize) -> Option<(usize, Option<usize>)>` — skips blank lines from `from`; `Some((opener, Some(closer)))`, `closer == None` when the fence never closes, `None` when no fence follows.
  - `pub fn capture::rewrite(chapter_text: &str, directive_line: usize, lines: &[String]) -> String`.

- [ ] **Step 1: Record today's test count**

Run: `cargo test -p bower-core 2>&1 | grep "test result"`
Write down the first `passed` number. The refactor below must not change it.

- [ ] **Step 2: Put `capture_block` on a shared scanner**

In `bower-core/src/block.rs`, make `fence_width` `pub(crate)` and add after it:

```rust
/// Where the fence after `from` opens and closes, blank lines between allowed:
/// `Some((opener, closer))`, with `closer == None` when it never closes, or
/// `None` when no fence follows. The one scanner behind `capture_block` and
/// `capture::rewrite`.
pub(crate) fn fence_span(lines: &[&str], from: usize) -> Option<(usize, Option<usize>)> {
    let mut j = from;
    while j < lines.len() && lines[j].trim().is_empty() {
        j += 1;
    }
    let width = fence_width(lines.get(j)?)?;
    let close = (j + 1..lines.len()).find(|&k| fence_width(lines[k]).is_some_and(|w| w >= width));
    Some((j, close))
}
```

Replace the body of `capture_block` with:

```rust
    let Some((open, close)) = fence_span(lines, from) else {
        return (None, from);
    };
    let info = lines[open]
        .trim_start()
        .trim_start_matches('`')
        .trim()
        .to_string();
    match close {
        Some(k) => (
            Some(BlockContent {
                info,
                lines: lines[open + 1..k].iter().map(ToString::to_string).collect(),
            }),
            k + 1,
        ),
        None => {
            errors.push(BowerError::UnclosedFence { loc: loc.clone() });
            (None, lines.len())
        }
    }
```

Run: `cargo test -p bower-core 2>&1 | grep "test result"` — the same count, all passing.

- [ ] **Step 3: Write the failing `rewrite` tests**

In `mod capture_tests`:

```rust
    const CH: &str = "# T\n\n<!-- bower repo=\"r\" output=\"check\" -->\n\n```text\nold one\nold two\n```\n\nAfter.\n";

    #[test]
    fn rewrite__replaces_only_the_fence_body() {
        assert_eq!(
            rewrite(CH, 3, &v(&["new"])),
            "# T\n\n<!-- bower repo=\"r\" output=\"check\" -->\n\n```text\nnew\n```\n\nAfter.\n"
        );
    }

    #[test]
    fn rewrite__fills_an_empty_fence() {
        let empty = "<!-- bower repo=\"r\" output=\"check\" -->\n```text\n```\n";
        assert_eq!(
            rewrite(empty, 1, &v(&["a", "b"])),
            "<!-- bower repo=\"r\" output=\"check\" -->\n```text\na\nb\n```\n"
        );
    }

    #[test]
    fn rewrite__empties_a_fence() {
        assert_eq!(
            rewrite(CH, 3, &[]),
            "# T\n\n<!-- bower repo=\"r\" output=\"check\" -->\n\n```text\n```\n\nAfter.\n"
        );
    }

    #[test]
    fn rewrite__widens_the_fence_when_output_holds_backticks() {
        let out = rewrite(CH, 3, &v(&["```rust", "x", "```"]));
        assert!(out.contains("\n````text\n```rust\nx\n```\n````\n"), "{out}");
    }

    #[test]
    fn rewrite__keeps_crlf_line_endings() {
        let crlf = "<!-- bower repo=\"r\" output=\"check\" -->\r\n```text\r\nold\r\n```\r\nAfter.\r\n";
        assert_eq!(
            rewrite(crlf, 1, &v(&["new"])),
            "<!-- bower repo=\"r\" output=\"check\" -->\r\n```text\r\nnew\r\n```\r\nAfter.\r\n"
        );
    }

    #[test]
    fn rewrite__keeps_a_missing_final_newline() {
        let bare = "<!-- bower repo=\"r\" output=\"check\" -->\n```text\nold\n```";
        assert_eq!(
            rewrite(bare, 1, &v(&["new"])),
            "<!-- bower repo=\"r\" output=\"check\" -->\n```text\nnew\n```"
        );
    }

    #[test]
    fn rewrite__without_a_fence_changes_nothing() {
        let text = "<!-- bower repo=\"r\" output=\"check\" -->\nprose\n";
        assert_eq!(rewrite(text, 1, &v(&["x"])), text);
        assert_eq!(rewrite(text, 0, &v(&["x"])), text);
    }
```

Run: compile error.

- [ ] **Step 4: Implement `rewrite`**

In `capture.rs`, add `use crate::block;` at the top, then:

```rust
/// Replace the body of the fence that follows the directive on
/// `directive_line` (1-based) with `lines`, and touch nothing else: every byte
/// outside the fence, line endings included, is copied back. The fence is
/// widened when a new line would close it. Without a fence after the
/// directive, the text comes back unchanged.
#[must_use]
pub fn rewrite(chapter_text: &str, directive_line: usize, lines: &[String]) -> String {
    // Pieces keep their own line endings; `bare` is what the scanner reads.
    let pieces: Vec<&str> = chapter_text.split_inclusive('\n').collect();
    let bare: Vec<&str> = pieces
        .iter()
        .map(|p| p.trim_end_matches(['\n', '\r']))
        .collect();
    if directive_line == 0 {
        return chapter_text.to_string();
    }
    let Some((open, Some(close))) = block::fence_span(&bare, directive_line) else {
        return chapter_text.to_string();
    };

    let eol = if pieces[open].ends_with("\r\n") { "\r\n" } else { "\n" };
    let width = block::fence_width(bare[open]).unwrap_or(3);
    let needed = lines
        .iter()
        .filter_map(|l| block::fence_width(l))
        .max()
        .map_or(width, |w| width.max(w + 1));
    let widen = |line: &str| {
        let indent = &line[..line.len() - line.trim_start().len()];
        let info = &line.trim_start()[width..];
        format!("{indent}{}{info}", "`".repeat(needed))
    };
    let (opener, closer) = if needed == width {
        (bare[open].to_string(), bare[close].to_string())
    } else {
        (widen(bare[open]), widen(bare[close]))
    };

    let mut out: String = pieces[..open].concat();
    out.push_str(&opener);
    out.push_str(eol);
    for line in lines {
        out.push_str(line);
        out.push_str(eol);
    }
    out.push_str(&closer);
    // The closer keeps whatever ended it: a newline, or nothing at the end of
    // the file.
    out.push_str(&pieces[close][bare[close].len()..]);
    out.push_str(&pieces[close + 1..].concat());
    out
}
```

Prelude: `pub use crate::capture::{Drift, ELISION, Scrub, drift, error_codes, normalize, rewrite, tidy};`

Run: `cargo test -p bower-core && make purity` — PASS.

- [ ] **Step 5: Commit**

```bash
git add bower-core/src
git commit -m "feat(core): rewrite an output fence in place, on the one fence scanner"
```

---
### Task 8: Fixtures, broken books, and the `capture × expect` axis (EPIC Phase 1e)

**Files:**
- Modify: `bower-testkit/src/fixtures.rs` (`captured_outputs`, `broken_outputs`, `valid`, `broken`)
- Modify: `bower-testkit/src/coverage.rs` (`Mechanism::CapturedOutput`, `capture_expects`)
- Modify: `bower-testkit/tests/corpus.rs`

**Interfaces:**
- Consumes: `Capture`, `CapturedOutput`, `PlannedStep::outputs`, `ELISION` (Tasks 3, 4, 6).
- Produces: `fixtures::captured_outputs() -> Fixture`; `CoverageReport::capture_expects: BTreeSet<String>` (entries like `"check×pass"`); `CoverageReport::missing_capture_expects() -> Vec<String>`.

- [ ] **Step 1: Write the failing corpus test**

In `bower-testkit/tests/corpus.rs`:

```rust
#[test]
fn captured_outputs_bind_every_legal_pair() {
    let f = fixtures::captured_outputs();
    let p = plan(&f.book, &f.catalog).unwrap();
    let steps = &p.repo("failers").unwrap().steps;
    let captures = |id: &str| {
        steps
            .iter()
            .find(|s| s.id.0 == id)
            .unwrap()
            .outputs
            .iter()
            .map(|o| o.capture)
            .collect::<Vec<_>>()
    };

    // `green`'s verify block sits last in the book and reaches back by name.
    assert_eq!(captures("green"), vec![Capture::Check, Capture::Verify]);
    assert_eq!(captures("broken"), vec![Capture::Check]);
    assert_eq!(captures("red"), vec![Capture::Check, Capture::Verify]);

    assert_eq!(
        steps[1].outputs[0].lines,
        vec!["error[E0308]: mismatched types".to_string(), ELISION.to_string()]
    );
    assert!(steps[0].outputs[0].lines.is_empty(), "an empty fence is not recorded yet");
    // Output blocks never reach a tree.
    assert_eq!(steps[2].tree.paths().collect::<Vec<_>>(), vec!["src/lib.rs"]);
}
```

In `corpus_state_coverage_is_complete`, after the exercise assertion:

```rust
    assert!(
        report.missing_capture_expects().is_empty(),
        "coverage gaps:\n{report}"
    );
```

Run: `cargo test -p bower-testkit --test corpus`
Expected: compile error, `cannot find function `captured_outputs``.

- [ ] **Step 2: Add the fixture**

In `bower-testkit/src/fixtures.rs`, after `exercise_forms`:

```rust
/// Every legal `capture × expect` pair, bound both ways: `check` on a green
/// step and its `verify` reaching back by `step=` from the end of the chapter,
/// `check` on a `compile_fail` step, and both on a `test_fail` step. Two fences
/// are empty: recorded and not-yet-recorded outputs plan alike.
#[must_use]
pub fn captured_outputs() -> Fixture {
    Fixture::new(
        "captured_outputs",
        BookSource::from_chapters(vec![chapter(
            "ch08-outputs.md",
            concat!(
                "# What the compiler said\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"green\" -->\n",
                "```rust\npub fn answer() -> u32 { 42 }\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"check\" -->\n",
                "```text\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"broken\" expect=\"compile_fail\" -->\n",
                "```rust\npub fn answer() -> u32 { \"42\" }\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"check\" -->\n",
                "```text\nerror[E0308]: mismatched types\n[...]\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"red\" expect=\"test_fail\" -->\n",
                "```rust\npub fn answer() -> u32 { 41 }\n#[test]\nfn answers() { assert_eq!(answer(), 42); }\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"check\" -->\n",
                "```text\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"verify\" -->\n",
                "```text\n[...]\ntest answers ... FAILED\n[...]\n```\n\n",
                "Back to the green step, by name:\n\n",
                "<!-- bower repo=\"failers\" output=\"verify\" step=\"green\" -->\n",
                "```text\n[...]\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n```\n",
            ),
        )]),
        failers(),
    )
}
```

Add `captured_outputs(),` to `valid()`, before `hello_playbook(),`.

After `broken_exercises`:

```rust
/// Errors raised by output blocks: binding, a key of another kind of block, a
/// second block for one capture, and a capture the verifier never runs.
fn broken_outputs() -> Vec<(&'static str, Fixture)> {
    vec![
        single(
            "OutputUnbound",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ),
        single(
            "OutputUnknownStep",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"check\" step=\"ghost\" -->\n```text\n```\n",
        ),
        single(
            "OutputConflictingKeys",
            "<!-- bower repo=\"failers\" output=\"check\" file=\"a.rs\" -->\n```text\n```\n",
        ),
        single(
            "OutputDuplicate",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
        ),
        single(
            "OutputNeverRuns",
            "<!-- bower repo=\"failers\" file=\"a.rs\" expect=\"compile_fail\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"verify\" -->\n```text\n```\n",
        ),
    ]
}
```

In `broken()`, after `cases.extend(broken_exercises());`:

```rust
    cases.extend(broken_outputs());
```

- [ ] **Step 3: Add the mechanism and the axis**

In `bower-testkit/src/coverage.rs`:

`enum Mechanism` gains, after `Exercise`:

```rust
    /// An `output="…"` block (EPIC-11).
    CapturedOutput,
```

`all()` returns `[Mechanism; 11]` with `Self::CapturedOutput` last; `Display`
maps it to `"captured-output"`.

`struct CoverageReport` gains, after `exercise_expects`:

```rust
    /// The legal `capture × expect` pairs the corpus binds, as `"check×pass"`.
    pub capture_expects: BTreeSet<String>,
```

In `absorb`, replace

```rust
            if b.play || b.exercise_block {
```

with

```rust
            if b.output.is_some() {
                self.mechanisms.insert(Mechanism::CapturedOutput.to_string());
            }
            if b.play || b.exercise_block || b.output.is_some() {
```

and in the plan-time loop, after the exercise check:

```rust
                for o in &s.outputs {
                    self.capture_expects
                        .insert(format!("{}×{}", o.capture, s.expect));
                }
```

Add after `missing_exercise_expects`:

```rust
    /// Legal `capture × expect` pairs — those the verifier runs
    /// (`Capture::runs_under`) — that no output in the corpus sits on.
    #[must_use]
    pub fn missing_capture_expects(&self) -> Vec<String> {
        Capture::all()
            .into_iter()
            .flat_map(|c| {
                Expect::all()
                    .into_iter()
                    .filter(move |e| c.runs_under(*e))
                    .map(move |e| format!("{c}×{e}"))
            })
            .filter(|pair| !self.capture_expects.contains(pair))
            .collect()
    }
```

`is_complete` gains `&& self.missing_capture_expects().is_empty()`. In
`Display`, after the `exercise × expect` line, change its trailing `)` to `)?;`
and add:

```rust
        writeln!(
            f,
            "{}",
            line(
                "capture × expect",
                &self.capture_expects,
                &self.missing_capture_expects()
            )
        )
```

- [ ] **Step 4: Run the testkit**

Run: `cargo test -p bower-testkit`
Expected: PASS — `every_broken_fixture_produces_its_named_error` covers the
five new names, and coverage is complete.

- [ ] **Step 5: Commit**

```bash
git add bower-testkit
git commit -m "test(testkit): output fixtures, five broken books, and the capture x expect axis"
```

---

### Task 9: Real textures and the four properties (EPIC Phase 2a and 2d)

**Files:**
- Create: `bower-testkit/src/textures.rs`
- Create: `bower-testkit/tests/textures.rs`
- Modify: `bower-testkit/src/lib.rs`, `bower-testkit/src/generators.rs`, `bower-testkit/tests/properties.rs`

**Interfaces:**
- Consumes: `normalize`, `Scrub`, `rewrite`, `plan` (Tasks 4–7).
- Produces: `textures::{SCRATCH, E0004, E0308, TEST_PANIC, WARNING_ONLY, MANIFEST_ERROR, TIMED_PASS, TIMED_PASS_SHUFFLED, E0004_ANSI, e0004_crlf, scrub}`; `generators::{arb_raw_output, arb_plain_lines}`.

- [ ] **Step 1: Add the textures**

Every constant is real output, captured 11 September 2026, with only the
scratch directory replaced by `SCRATCH`. `E0004*` and `TIMED_PASS` came from
small crates on rustc 1.98.1; `E0308` and `TEST_PANIC` from `hello-playbook`
steps `wont-compile` and `test-that-fails` on its pinned 1.95.0;
`WARNING_ONLY` from `rust4failures` step `kata-compiles`. `TIMED_PASS_SHUFFLED`
is the one exception: `TIMED_PASS` reordered by hand, and its doc comment says so.

Create `bower-testkit/src/textures.rs`:

```rust
//! Raw command output, as a machine printed it: the data textures of
//! `bower_core::capture::normalize`. Each is real text, captured on 11
//! September 2026, with only the scratch directory replaced by [`SCRATCH`] —
//! except [`TIMED_PASS_SHUFFLED`], which says how it was made.

use bower_core::prelude::Scrub;

/// Where these textures' scratch tree and target directory lived.
pub const SCRATCH: &str = "/private/var/folders/zz/T/bower-verify/book";

/// The scrub `bower verify` would build for [`SCRATCH`].
#[must_use]
pub fn scrub() -> Scrub {
    Scrub(vec![
        (format!("{SCRATCH}/tree/"), String::new()),
        (format!("{SCRATCH}/tree"), ".".to_string()),
        (format!("{SCRATCH}/target"), "target".to_string()),
    ])
}

/// `cargo check` on a non-exhaustive `match` over `char`: a note, a help, and
/// spans — the shape `rust4failures` chapter 3 teaches.
pub const E0004: &str = r#"    Checking rank v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
error[E0004]: non-exhaustive patterns: `'\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered
  --> src/lib.rs:8:15
   |
 8 |         match c {
   |               ^ patterns `'\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered
   |
   = note: the matched value is of type `char`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern as shown, or multiple match arms
   |
10 ~             'K' | 'k' => Rank::KING,
11 ~             _ => todo!(),
   |

For more information about this error, try `rustc --explain E0004`.
error: could not compile `rank` (lib) due to 1 previous error
"#;

/// [`E0004`] with `CARGO_TERM_COLOR=always`.
pub const E0004_ANSI: &str = "\x1b[1m\x1b[92m    Checking\x1b[0m rank v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)\n\x1b[1m\x1b[91merror[E0004]\x1b[0m\x1b[1m: non-exhaustive patterns: `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered\x1b[0m\n  \x1b[1m\x1b[94m--> \x1b[0msrc/lib.rs:8:15\n   \x1b[1m\x1b[94m|\x1b[0m\n\x1b[1m\x1b[94m 8\x1b[0m \x1b[1m\x1b[94m|\x1b[0m         match c {\n   \x1b[1m\x1b[94m|\x1b[0m               \x1b[1m\x1b[91m^\x1b[0m \x1b[1m\x1b[91mpatterns `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered\x1b[0m\n   \x1b[1m\x1b[94m|\x1b[0m\n   \x1b[1m\x1b[94m= \x1b[0m\x1b[1mnote\x1b[0m: the matched value is of type `char`\n\x1b[1m\x1b[96mhelp\x1b[0m: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern as shown, or multiple match arms\n   \x1b[1m\x1b[94m|\x1b[0m\n\x1b[1m\x1b[94m10\x1b[0m \x1b[92m~ \x1b[0m            'K' | 'k' => Rank::KING\x1b[92m,\x1b[0m\n\x1b[1m\x1b[94m11\x1b[0m \x1b[92m~             _ => todo!()\x1b[0m,\n   \x1b[1m\x1b[94m|\x1b[0m\n\n\x1b[1mFor more information about this error, try `rustc --explain E0004`.\x1b[0m\n\x1b[1m\x1b[91merror\x1b[0m: could not compile `rank` (lib) due to 1 previous error\n";

/// [`E0004`] as a Windows runner prints it.
#[must_use]
pub fn e0004_crlf() -> String {
    E0004.replace('\n', "\r\n")
}

/// `hello-playbook` step `wont-compile`: a string literal where a `u32` goes.
pub const E0308: &str = r#"    Checking hello-playbook v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
error[E0308]: mismatched types
 --> src/scratch.rs:4:18
  |
4 |     let n: u32 = "42";
  |            ---   ^^^^ expected `u32`, found `&str`
  |            |
  |            expected due to this

For more information about this error, try `rustc --explain E0308`.
error: could not compile `hello-playbook` (lib) due to 1 previous error
"#;

/// `hello-playbook` step `test-that-fails`, both streams through one pipe:
/// cargo's status lines, libtest's report with the panicking thread's ID and
/// the clock, then cargo's closing line.
pub const TEST_PANIC: &str = r#"   Compiling hello-playbook v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.64s
     Running unittests src/lib.rs (/private/var/folders/zz/T/bower-verify/book/target/debug/deps/hello_playbook-75e8099ccb6a0c05)

running 2 tests
test tests::greet_ignores_stray_whitespace ... FAILED
test tests::greet_uses_the_name ... ok

failures:

---- tests::greet_ignores_stray_whitespace stdout ----

thread 'tests::greet_ignores_stray_whitespace' (714955) panicked at src/lib.rs:20:9:
assertion `left == right` failed
  left: "Hello,   world  !"
 right: "Hello, world!"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    tests::greet_ignores_stray_whitespace

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--lib`
"#;

/// `rust4failures` step `kata-compiles`: `cargo check` succeeds with one
/// warning.
pub const WARNING_ONLY: &str = r"    Checking rust4failures v0.0.1 (/private/var/folders/zz/T/bower-verify/book/tree)
warning: method `hello__world` should have a snake case name
  --> src/lib.rs:20:13
   |
20 |     pub fn  hello__world() -> String {
   |             ^^^^^^^^^^^^ help: convert the identifier to snake case: `hello_world`
   |
   = note: `#[warn(non_snake_case)]` (part of `#[warn(nonstandard_style)]`) on by default

warning: `rust4failures` (lib) generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.31s
";

/// A manifest with no targets: cargo names the scratch path itself — the
/// shape of `rust4failures` step `kata`, a lone `Cargo.toml`.
pub const MANIFEST_ERROR: &str = r"error: failed to parse manifest at `/private/var/folders/zz/T/bower-verify/book/tree/Cargo.toml`

Caused by:
  no targets specified in the manifest
  either src/lib.rs, src/main.rs, a [lib] section, or [[bin]] section must be present
";

/// Three passing tests and the doc-test run, timed.
pub const TIMED_PASS: &str = r"   Compiling three v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.92s
     Running unittests src/lib.rs (/private/var/folders/zz/T/bower-verify/book/target/debug/deps/three-b48f404885b51d48)

running 3 tests
test tests::alpha ... ok
test tests::beta ... ok
test tests::gamma ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests three

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

";

/// [`TIMED_PASS`] edited by hand into what another multi-threaded run prints:
/// another order, another clock, another hash. The one texture not captured
/// as-is — one thread per step (EPIC-11 Decision 12) means `verify` never
/// sees this, but a reader's own run does.
pub const TIMED_PASS_SHUFFLED: &str = r"   Compiling three v0.1.0 (/private/var/folders/zz/T/bower-verify/book/tree)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.07s
     Running unittests src/lib.rs (/private/var/folders/zz/T/bower-verify/book/target/debug/deps/three-0c1d2e3f4a5b6c7d)

running 3 tests
test tests::gamma ... ok
test tests::alpha ... ok
test tests::beta ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests three

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

";
```

In `bower-testkit/src/lib.rs`, add `pub mod textures;` after `pub mod generators;`.

- [ ] **Step 2: Write the texture tests**

Create `bower-testkit/tests/textures.rs`:

```rust
//! `normalize` against real output: every texture reduces to the lines a
//! reader of the book should see, and nothing that varies by machine or run.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bower_testkit::prelude::*;
use bower_testkit::textures::{self, scrub};

fn v(lines: &[&str]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

#[test]
fn texture__e0004_keeps_the_spans_and_notes() {
    assert_eq!(
        normalize(textures::E0004, &scrub()),
        v(&[
            "error[E0004]: non-exhaustive patterns: `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered",
            "  --> src/lib.rs:8:15",
            "   |",
            " 8 |         match c {",
            "   |               ^ patterns `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered",
            "   |",
            "   = note: the matched value is of type `char`",
            "help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern as shown, or multiple match arms",
            "   |",
            "10 ~             'K' | 'k' => Rank::KING,",
            "11 ~             _ => todo!(),",
            "   |",
            "",
            "For more information about this error, try `rustc --explain E0004`.",
            "error: could not compile `rank` (lib) due to 1 previous error",
        ])
    );
}

#[test]
fn texture__colour_and_crlf_change_nothing() {
    let plain = normalize(textures::E0004, &scrub());
    assert_eq!(normalize(textures::E0004_ANSI, &scrub()), plain);
    assert_eq!(normalize(&textures::e0004_crlf(), &scrub()), plain);
}

#[test]
fn texture__the_test_panic_loses_its_thread_id_clock_and_paths() {
    let lines = normalize(textures::TEST_PANIC, &scrub());
    assert_eq!(lines[0], "running 2 tests");
    assert!(
        lines.contains(&"thread 'tests::greet_ignores_stray_whitespace' panicked at src/lib.rs:20:9:".to_string()),
        "{lines:#?}"
    );
    assert!(
        lines.contains(&"test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out".to_string()),
        "{lines:#?}"
    );
    assert_eq!(lines.last().unwrap(), "error: test failed, to rerun pass `--lib`");
    assert!(!lines.iter().any(|l| l.contains(textures::SCRATCH)), "{lines:#?}");
    assert!(!lines.iter().any(|l| l.contains("Compiling") || l.contains("Running")), "{lines:#?}");
}

#[test]
fn texture__a_warning_only_check_keeps_the_warning() {
    let lines = normalize(textures::WARNING_ONLY, &scrub());
    assert_eq!(lines[0], "warning: method `hello__world` should have a snake case name");
    assert_eq!(lines.last().unwrap(), "warning: `rust4failures` (lib) generated 1 warning");
}

#[test]
fn texture__the_manifest_error_names_a_relative_path() {
    assert_eq!(
        normalize(textures::MANIFEST_ERROR, &scrub())[0],
        "error: failed to parse manifest at `Cargo.toml`"
    );
}

#[test]
fn texture__two_runs_of_one_suite_normalize_alike() {
    assert_eq!(
        normalize(textures::TIMED_PASS_SHUFFLED, &scrub()),
        normalize(textures::TIMED_PASS, &scrub())
    );
}

#[test]
fn texture__error_codes_come_from_the_headline() {
    assert_eq!(error_codes(&normalize(textures::E0004, &scrub())), vec!["E0004"]);
    assert_eq!(error_codes(&normalize(textures::E0308, &scrub())), vec!["E0308"]);
    assert!(error_codes(&normalize(textures::TEST_PANIC, &scrub())).is_empty());
}
```

Run: `cargo test -p bower-testkit --test textures`
Expected: PASS. If `texture__colour_and_crlf_change_nothing` fails, print both
vectors and fix the rule that differs — never the texture.

- [ ] **Step 3: Add the generators**

In `bower-testkit/src/generators.rs`, at the end:

```rust
/// Arbitrary command output: printable ASCII lines — backticks and brackets
/// included — some in colour, joined by LF or CRLF. The input space of
/// `normalize`, which is not a compiler's grammar, and that is the point.
pub fn arb_raw_output() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(("[ -~]{0,24}", any::<bool>()), 0..=8),
        any::<bool>(),
    )
        .prop_map(|(lines, crlf)| {
            let eol = if crlf { "\r\n" } else { "\n" };
            lines
                .into_iter()
                .map(|(l, coloured)| {
                    if coloured {
                        format!("\x1b[1m\x1b[91m{l}\x1b[0m")
                    } else {
                        l
                    }
                })
                .collect::<Vec<_>>()
                .join(eol)
        })
}

/// Plain printable lines: what [`arb_raw_output`] draws before colour and
/// line endings.
pub fn arb_plain_lines() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[ -~]{0,24}", 0..=8)
}

/// Lowercase words, one per line: prose that can never be a fence or a
/// directive.
pub fn arb_words() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(line(), 0..=4)
}
```

In the prelude in `bower-testkit/src/lib.rs`:

```rust
    pub use crate::generators::{arb_book, arb_plain_lines, arb_raw_output, arb_words};
```

- [ ] **Step 4: Write the four properties**

In `bower-testkit/tests/properties.rs`, extend the module doc list with:

```rust
//! 5. **Normalization is idempotent** and blind to colour and line endings.
//! 6. **Recording round-trips** — a rewritten fence plans back to exactly the
//!    lines written, and nothing outside the fence moves.
```

Inside the existing `proptest! { … }` block, add:

```rust
    #[test]
    fn normalize_is_idempotent(raw in arb_raw_output()) {
        let once = normalize(&raw, &Scrub::default());
        let twice = normalize(&once.join("\n"), &Scrub::default());
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn normalize_ignores_ansi_and_crlf(lines in arb_plain_lines()) {
        let plain = lines.join("\n");
        let dressed = lines
            .iter()
            .map(|l| format!("\x1b[1m{l}\x1b[0m"))
            .collect::<Vec<_>>()
            .join("\r\n");
        prop_assert_eq!(
            normalize(&dressed, &Scrub::default()),
            normalize(&plain, &Scrub::default())
        );
    }

    #[test]
    fn rewrite_round_trips_through_plan(raw in arb_raw_output()) {
        let live = normalize(&raw, &Scrub::default());
        // Line 8 is the output directive.
        let chapter = concat!(
            "# Out\n\n",
            "<!-- bower repo=\"gen\" file=\"a.rs\" -->\n```rust\nx\n```\n\n",
            "<!-- bower repo=\"gen\" output=\"check\" -->\n```text\n```\n\n",
            "The end.\n",
        );
        let text = rewrite(chapter, 8, &live);
        let book = BookSource::from_chapters(vec![Chapter::new("out.md", &text)]);
        let p = plan(&book, &RepoCatalog::from_names(&["gen"])).unwrap();
        prop_assert_eq!(&p.repos[0].steps[0].outputs[0].lines, &live);
    }

    #[test]
    fn rewrite_touches_nothing_outside_the_fence(
        live in arb_plain_lines(),
        before in arb_words(),
        after in arb_words(),
    ) {
        let head: String = before.iter().map(|l| format!("{l}\n")).collect::<String>()
            + "<!-- bower repo=\"gen\" output=\"check\" -->\n";
        let tail: String = "\n".to_string()
            + &after.iter().map(|l| format!("{l}\n")).collect::<String>();
        let text = format!("{head}```text\nold\n```{tail}");
        let out = rewrite(&text, before.len() + 1, &live);
        prop_assert!(out.starts_with(&head), "{out}");
        prop_assert!(out.ends_with(&tail), "{out}");
    }
```

- [ ] **Step 5: Run the properties**

Run: `cargo test -p bower-testkit --test properties`
Expected: PASS. A shrunk counter-example is a real bug in a rule: fix the rule
and add the counter-example as a unit test in `bower-core/src/capture.rs`.

- [ ] **Step 6: Commit**

```bash
git add bower-testkit
git commit -m "test(testkit): real output textures and four capture properties"
```

---
### Task 10: Judging outputs in `bower verify` (EPIC Phase 3a, 3b, 3d; Decisions 4, 6, 7, 10, 12)

**Files:**
- Modify: `bower/src/verify.rs` (`OutputResult`, `OutputVerdict`, `judge_output`, `scrub_for`, `cargo_home`, `StepVerdict::outputs`, `VerifyReport::{unpinned_step, outputs}`, `Verifier::run`, tests)
- Modify: `bower/src/main.rs` (`print_rows`, `print_broken`, `warn_unpinned`, `print_output_problems`, `run_verify`)

**Interfaces:**
- Consumes: `CapturedOutput`, `Capture`, `Drift`, `Scrub`, `drift`, `normalize` (kernel); `step_env`, `pins_toolchain`, `run` (Task 2).
- Produces:
  - `pub enum OutputResult { Matches, NotRecorded, Drifted(Drift), Unjudged }` (derives `Clone, Debug, Eq, PartialEq`).
  - `pub struct OutputVerdict { pub loc: Location, pub capture: Capture, pub live: Vec<String>, pub result: OutputResult }` (derives `Clone, Debug`).
  - `pub fn judge_output(recorded: &CapturedOutput, verdict: &Verdict, ran: Option<&Outcome>, scrub: &Scrub) -> OutputVerdict`.
  - `pub fn scrub_for(tree_dir: &Path, target_dir: &Path, cargo_home: Option<&Path>) -> Scrub`.
  - `StepVerdict::outputs: Vec<OutputVerdict>`; `VerifyReport::unpinned_step: Option<StepId>`; `VerifyReport::outputs(&self) -> impl Iterator<Item = (&StepVerdict, &OutputVerdict)>`.

- [ ] **Step 1: Write the failing judging tests**

In `bower/src/verify.rs`, `mod verify_tests`:

```rust
    fn recorded(lines: &[&str]) -> CapturedOutput {
        CapturedOutput {
            loc: Location::new("src/ch04.md", 12),
            capture: Capture::Check,
            info: "text".to_string(),
            lines: lines.iter().map(ToString::to_string).collect(),
        }
    }

    fn printed(text: &str) -> Outcome {
        Outcome {
            command: "cargo check".to_string(),
            success: false,
            output: text.to_string(),
        }
    }

    const E0308: &str =
        "    Checking hp v0.1.0 (/scratch/tree)\nerror[E0308]: mismatched types\n --> src/scratch.rs:4:18\n";

    #[test]
    fn output__a_match_upholds_the_step() {
        let v = judge_output(
            &recorded(&["error[E0308]: mismatched types", " --> src/scratch.rs:4:18"]),
            &Verdict::Upheld,
            Some(&printed(E0308)),
            &Scrub::default(),
        );
        assert_eq!(v.result, OutputResult::Matches);
    }

    #[test]
    fn output__drift_fails_verify_and_names_the_line() {
        let v = judge_output(
            &recorded(&["error[E0308]: mismatched types", " --> src/scratch.rs:5:18"]),
            &Verdict::Upheld,
            Some(&printed(E0308)),
            &Scrub::default(),
        );
        assert_eq!(
            v.result,
            OutputResult::Drifted(Drift {
                line: 2,
                recorded: Some(" --> src/scratch.rs:5:18".to_string()),
                live: Some(" --> src/scratch.rs:4:18".to_string()),
            })
        );
    }

    #[test]
    fn output__an_empty_fence_is_not_recorded_and_fails() {
        let v = judge_output(&recorded(&[]), &Verdict::Upheld, Some(&printed(E0308)), &Scrub::default());
        assert_eq!(v.result, OutputResult::NotRecorded);
        assert_eq!(v.live.len(), 2, "the live text is what --record would write");
    }

    #[test]
    fn output__a_broken_step_is_unjudged_and_never_recorded() {
        // The broken claim is the headline. A snapshot of the wrong failure
        // is worse than none (EPIC-11 Decision 6).
        let broken = Verdict::Broken {
            happened: "it compiled".to_string(),
            command: "cargo check".to_string(),
            output: String::new(),
        };
        let v = judge_output(&recorded(&[]), &broken, Some(&printed(E0308)), &Scrub::default());
        assert_eq!(v.result, OutputResult::Unjudged);
        assert!(v.live.is_empty());

        // A command that never ran is unjudged too.
        let v = judge_output(&recorded(&[]), &Verdict::Upheld, None, &Scrub::default());
        assert_eq!(v.result, OutputResult::Unjudged);
    }

    #[test]
    fn scrub_for__names_the_tree_the_target_and_cargo_home() {
        let s = scrub_for(
            Path::new("/w/tree"),
            Path::new("/w/target"),
            Some(Path::new("/home/me/.cargo")),
        );
        assert_eq!(
            normalize(
                "error: failed to parse manifest at `/w/tree/Cargo.toml`\nin /w/tree, /w/target/debug, /home/me/.cargo/registry\n",
                &s
            ),
            vec![
                "error: failed to parse manifest at `Cargo.toml`".to_string(),
                "in ., target/debug, $CARGO_HOME/registry".to_string(),
            ]
        );
    }

    #[test]
    fn scrub_for__covers_the_canonical_spelling_too() {
        // macOS: the temp directory is `/var/folders/…`, and cargo prints
        // `/private/var/folders/…`. Both must go.
        let dir = std::env::temp_dir().join("bower-verify-scrub");
        std::fs::create_dir_all(&dir).unwrap();
        let canonical = dir.canonicalize().unwrap().display().to_string();
        let s = scrub_for(&dir, &dir.join("target"), None);
        assert!(s.0.iter().any(|(from, to)| from == &canonical && to == "."), "{s:?}");
    }
```

Extend the file's import line to
`use bower_core::prelude::{Capture, CapturedOutput, Drift, Expect, Location, PlannedStep, RepoPlan, Scrub, StepId, drift, normalize};`
The test module's `use super::*;` then brings every name the tests use.

Run: `cargo test -p bower --lib output__`
Expected: compile error, `cannot find function `judge_output``.

- [ ] **Step 2: Implement the verdict types and `judge_output`**

In `bower/src/verify.rs`, after `enum Verdict`:

```rust
/// What became of one recorded output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputResult {
    /// The fence holds what the command printed.
    Matches,
    /// The fence is empty: the directive is written and `--record` has not
    /// run yet. Fails `verify` (EPIC-11 Decision 7).
    NotRecorded,
    /// The fence and the live output disagree, first at this line.
    Drifted(Drift),
    /// The step's claim did not hold, or the command never ran, so the output
    /// was not compared (Decision 6).
    Unjudged,
}

/// One output block, judged.
#[derive(Clone, Debug)]
pub struct OutputVerdict {
    /// The block's directive: where the report points and `--record` writes.
    pub loc: Location,
    pub capture: Capture,
    /// What the command printed, normalized. Empty when unjudged.
    pub live: Vec<String>,
    pub result: OutputResult,
}

/// Judge one recorded output against what its command printed. Pure, so the
/// whole table is tested without a compiler.
#[must_use]
pub fn judge_output(
    recorded: &CapturedOutput,
    verdict: &Verdict,
    ran: Option<&Outcome>,
    scrub: &Scrub,
) -> OutputVerdict {
    let (Verdict::Upheld, Some(outcome)) = (verdict, ran) else {
        return OutputVerdict {
            loc: recorded.loc.clone(),
            capture: recorded.capture,
            live: Vec::new(),
            result: OutputResult::Unjudged,
        };
    };
    let live = normalize(&outcome.output, scrub);
    let result = if recorded.lines.is_empty() {
        OutputResult::NotRecorded
    } else {
        drift(&recorded.lines, &live).map_or(OutputResult::Matches, OutputResult::Drifted)
    };
    OutputVerdict {
        loc: recorded.loc.clone(),
        capture: recorded.capture,
        live,
        result,
    }
}

/// The machine's paths a capture must not keep: the scratch tree, the target
/// directory, and cargo's home — each as given and as canonicalized, since
/// cargo prints the canonical spelling (EPIC-11 Decision 4).
#[must_use]
pub fn scrub_for(tree_dir: &Path, target_dir: &Path, cargo_home: Option<&Path>) -> Scrub {
    fn spellings(dir: &Path) -> Vec<String> {
        let mut out = vec![dir.display().to_string()];
        if let Ok(c) = dir.canonicalize() {
            let c = c.display().to_string();
            if !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }
    let mut pairs = Vec::new();
    for tree in spellings(tree_dir) {
        pairs.push((format!("{tree}/"), String::new()));
        pairs.push((tree, ".".to_string()));
    }
    for target in spellings(target_dir) {
        pairs.push((target, "target".to_string()));
    }
    if let Some(home) = cargo_home {
        for h in spellings(home) {
            pairs.push((h, "$CARGO_HOME".to_string()));
        }
    }
    Scrub(pairs)
}

/// Where cargo keeps registry sources: `$CARGO_HOME`, else `~/.cargo`. A
/// diagnostic inside a dependency names a path under it.
fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))
}
```

`StepVerdict` gains, after `verdict`:

```rust
    /// This step's output blocks, judged, in document order.
    pub outputs: Vec<OutputVerdict>,
```

`VerifyReport` gains, after `verdicts`:

```rust
    /// The first step that records output while nothing pins its toolchain:
    /// its tree has no `rust-toolchain.toml` and the repo sets no `toolchain`.
    pub unpinned_step: Option<StepId>,
```

and `impl VerifyReport` gains:

```rust
    /// Every judged output, with the step it belongs to.
    pub fn outputs(&self) -> impl Iterator<Item = (&StepVerdict, &OutputVerdict)> {
        self.verdicts
            .iter()
            .flat_map(|v| v.outputs.iter().map(move |o| (v, o)))
    }
```

The crate does not build again until Step 3 fills the new fields; the tests
run at the end of Step 3.

- [ ] **Step 3: Run each step through one helper**

`Verifier::run` would pass clippy's length limit with this change, so the
per-step work moves into a helper. Add above `impl Verifier<'_>`:

```rust
/// What stays the same from one step to the next.
struct Bench<'a> {
    check_cmd: &'a str,
    verify_cmd: &'a str,
    toolchain: Option<&'a str>,
    scaffolding: &'a Blobs,
    tree_dir: PathBuf,
    target_dir: PathBuf,
    cargo_home: Option<PathBuf>,
}

/// One step, verified: its claim, its outputs, and whether it records output
/// with nothing pinning its toolchain.
struct StepRun {
    verdict: Verdict,
    outputs: Vec<OutputVerdict>,
    unpinned: bool,
}

/// Write one step's tree, run its commands, and judge its claim and its
/// outputs. A step that records output runs serially (Decision 12).
fn run_step(bench: &Bench, step: &PlannedStep) -> Result<StepRun, VerifyError> {
    if step.expect == Expect::Skip {
        return Ok(StepRun {
            verdict: Verdict::Skipped,
            outputs: Vec::new(),
            unpinned: false,
        });
    }
    let mut blobs = bench.scaffolding.clone();
    blobs.extend(blobs_of(&step.tree));
    write_tree_to_disk(&bench.tree_dir, &blobs).map_err(|source| VerifyError::Io {
        path: bench.tree_dir.clone(),
        source,
    })?;

    let records = !step.outputs.is_empty();
    let env = step_env(&blobs, bench.toolchain, records);
    let check = run(bench.check_cmd, &bench.tree_dir, &bench.target_dir, &env)?;
    let verify = if check.success && step.expect != Expect::CompileFail {
        Some(run(bench.verify_cmd, &bench.tree_dir, &bench.target_dir, &env)?)
    } else {
        None
    };
    let verdict = verdict_for(step.expect, &check, verify.as_ref());

    let scrub = scrub_for(&bench.tree_dir, &bench.target_dir, bench.cargo_home.as_deref());
    let outputs = step
        .outputs
        .iter()
        .map(|o| {
            let ran = match o.capture {
                Capture::Check => Some(&check),
                Capture::Verify => verify.as_ref(),
            };
            judge_output(o, &verdict, ran, &scrub)
        })
        .collect();

    Ok(StepRun {
        verdict,
        outputs,
        unpinned: records && bench.toolchain.is_none() && !pins_toolchain(&blobs),
    })
}
```

In `Verifier::run`, replace
everything from `// One target directory across every step` to the end of the
`for` loop with:

```rust
        // One target directory across every step, so twenty steps are not
        // twenty cold builds.
        let bench = Bench {
            check_cmd,
            verify_cmd,
            toolchain: cfg.and_then(|c| c.toolchain.as_deref()),
            scaffolding: &scaffolding,
            tree_dir: self.work_dir.join("tree"),
            target_dir: self.work_dir.join("target"),
            cargo_home: cargo_home(),
        };

        let mut verdicts = Vec::new();
        let mut unpinned_step = None;
        for step in plan.steps.iter().skip(start) {
            if only.is_some_and(|want| want != step.id.0) {
                continue;
            }
            let StepRun {
                verdict,
                outputs,
                unpinned,
            } = run_step(&bench, step)?;
            if unpinned && unpinned_step.is_none() {
                unpinned_step = Some(step.id.clone());
            }
            verdicts.push(StepVerdict {
                seq: step.seq,
                id: step.id.clone(),
                anchor: step.anchor.clone(),
                expect: step.expect,
                verdict,
                outputs,
            });
        }
```

and return `Ok(VerifyReport { repo: repo_name, verdicts, unpinned_step })`.
Remove the Task 2 `toolchain` local and its comment; `run_step` now passes
`records`.

Run: `cargo test -p bower --lib verify_tests` — PASS, including the six new
tests from Step 1.

- [ ] **Step 4: Report output rows, problems, and the warning**

In `bower/src/main.rs`, change the import to
`use bower::verify::{OutputResult, Verdict, Verifier, VerifyReport};` and add
these helpers after `run_verify`:

```rust
/// One row per step, and under it one row per output block.
fn print_rows(report: &VerifyReport) {
    println!("{} — {} step(s)", report.repo, report.verdicts.len());
    for v in &report.verdicts {
        let mark = match &v.verdict {
            Verdict::Upheld => "ok  ",
            Verdict::Skipped => "skip",
            Verdict::Broken { .. } => "FAIL",
        };
        println!("  {mark} {:03} {:<28} expect={}", v.seq, v.id.0, v.expect);
        for o in &v.outputs {
            let mark = match &o.result {
                OutputResult::Matches => "ok  ",
                OutputResult::Drifted(_) => "drft",
                OutputResult::NotRecorded => "todo",
                OutputResult::Unjudged => "skip",
            };
            println!("       {mark} output={:<6} {}", o.capture, o.loc);
        }
    }
}

/// The report for every step whose claim did not hold; returns how many.
fn print_broken(report: &VerifyReport) -> usize {
    let mut count = 0;
    for v in report.broken() {
        count += 1;
        let Verdict::Broken {
            happened,
            command,
            output,
        } = &v.verdict
        else {
            continue;
        };
        eprintln!(
            "\n{}: step `{}` claims `{}`, but {happened}.",
            v.anchor, v.id.0, v.expect
        );
        eprintln!("    (`{command}` in the step's tree)");
        let tail: Vec<&str> = output.lines().rev().take(8).collect();
        for line in tail.iter().rev() {
            eprintln!("    {line}");
        }
    }
    count
}

/// Recorded output that depends on this machine's default compiler is a
/// snapshot nobody else can reproduce (EPIC-11 Decision 10).
fn warn_unpinned(report: &VerifyReport) {
    if let Some(id) = &report.unpinned_step {
        eprintln!(
            "\nwarning: step `{id}` of `{repo}` records compiler output, but its tree pins no \
             toolchain and bower.toml sets no `toolchain` for it, so what it records depends \
             on this machine's default compiler. Pin one under [repos.{repo}].",
            repo = report.repo
        );
    }
}

/// The report for every output that is not what the book holds. Returns
/// (drifted, not recorded).
fn print_output_problems(reports: &[VerifyReport]) -> (usize, usize) {
    let (mut drifted, mut unrecorded) = (0, 0);
    for (v, o) in reports.iter().flat_map(|r| r.outputs()) {
        let record = format!("bower verify --record --step {}", v.id);
        match &o.result {
            OutputResult::Drifted(d) => {
                drifted += 1;
                eprintln!(
                    "\n{}: the `{}` output of step `{}` has drifted, at line {} of the recording.",
                    o.loc, o.capture, v.id, d.line
                );
                eprintln!(
                    "    recorded: {}",
                    d.recorded.as_deref().unwrap_or("(nothing: the recording ends here)")
                );
                eprintln!(
                    "    live:     {}",
                    d.live.as_deref().unwrap_or("(nothing: not in the live output)")
                );
                eprintln!("    if the compiler is right, run: {record}");
            }
            OutputResult::NotRecorded => {
                unrecorded += 1;
                eprintln!(
                    "\n{}: the `{}` output of step `{}` is not recorded yet.",
                    o.loc, o.capture, v.id
                );
                eprintln!("    run: {record}");
            }
            OutputResult::Matches | OutputResult::Unjudged => {}
        }
    }
    (drifted, unrecorded)
}
```

In `run_verify`, replace the body of the `for repo in selected` loop after
`let report = match verifier.run(…) { … };` — the `println!` of the header,
the row loop, and the `for v in report.broken()` loop — with:

```rust
        print_rows(&report);
        broken_total += print_broken(&report);
        warn_unpinned(&report);
        reports.push(report);
```

declare `let mut reports = Vec::new();` beside `let mut broken_total`, and
replace the final `if broken_total == 0 { … } else { … }` with:

```rust
    let (drifted, unrecorded) = print_output_problems(&reports);
    if broken_total + drifted + unrecorded == 0 {
        println!("\nevery claim holds");
        return ExitCode::SUCCESS;
    }
    if broken_total > 0 {
        eprintln!("\n{broken_total} claim(s) in the book are not true");
    }
    if drifted > 0 {
        eprintln!("{drifted} recorded output(s) no longer match what the compiler says");
    }
    if unrecorded > 0 {
        eprintln!("{unrecorded} output(s) are not recorded yet");
    }
    ExitCode::FAILURE
```

If `run_verify` still carries `#[allow(clippy::too_many_lines)]` and no longer
needs it, remove the attribute.

- [ ] **Step 5: Fix the sweep's count**

Output rows also print `ok  `. In `bower/tests/verification.rs`,
`upholds_every_step`, replace
`assert_eq!(stdout.matches("ok  ").count(), 20, "{stdout}");` with:

```rust
    // Step rows start two spaces in; output rows start seven.
    let steps_ok = stdout.lines().filter(|l| l.starts_with("  ok ")).count();
    assert_eq!(steps_ok, 20, "{stdout}");
```

- [ ] **Step 6: Run the crate**

Run: `cargo test -p bower && cargo clippy -p bower --all-targets --all-features -- -D warnings`
Expected: PASS, no warnings. No book has an output block yet, so every golden
prints what it printed before.

- [ ] **Step 7: Commit**

```bash
git add bower/src/verify.rs bower/src/main.rs bower/tests/verification.rs
git commit -m "feat(verify): judge recorded outputs of upheld steps; drift and missing recordings fail"
```

---

### Task 11: `bower verify --record` (EPIC Phase 3c, Decisions 5, 6, 8)

**Files:**
- Create: `bower/src/record.rs`
- Modify: `bower/src/lib.rs` (`pub mod record;`)
- Modify: `bower/src/main.rs` (`--record`, `record_outputs`, `write_lock`, `run_plan`)

**Interfaces:**
- Consumes: `VerifyReport::outputs`, `OutputResult` (Task 10); `rewrite` (Task 7); `check_path`.
- Produces:
  - `pub struct Recording { pub chapter: String, pub line: usize, pub lines: Vec<String> }` (derives `Clone, Debug, Eq, PartialEq`).
  - `pub fn recordings(reports: &[VerifyReport]) -> Vec<Recording>`.
  - `pub fn apply(texts: &BTreeMap<String, String>, recs: &[Recording]) -> BTreeMap<String, String>` — only the chapters that changed.
  - `pub fn write_chapters(book_root: &Path, changed: &BTreeMap<String, String>) -> std::io::Result<()>`.

- [ ] **Step 1: Write the failing tests**

Create `bower/src/record.rs` with the module doc and tests first:

```rust
//! `bower verify --record`: writing what the compiler said back into the book.
//!
//! The only code path that edits a chapter. It rewrites fence bodies and
//! nothing else (`bower_core::prelude::rewrite`), and only the fences whose
//! output is missing or has drifted: a fence that still matches is left alone,
//! so an author's `[...]` trim survives every re-record while it holds
//! (EPIC-11 Decision 5). A broken step's output is never written (Decision 6).

use std::collections::BTreeMap;
use std::path::Path;

use bower_core::prelude::rewrite;

use crate::materialize::check_path;
use crate::verify::{OutputResult, VerifyReport};

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod record_tests {
    use super::*;
    use crate::verify::{OutputVerdict, StepVerdict, Verdict};
    use bower_core::prelude::{Capture, Drift, Expect, Location, StepId};

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    fn report(outputs: Vec<(usize, OutputResult, &[&str])>) -> VerifyReport {
        VerifyReport {
            repo: "r".to_string(),
            verdicts: vec![StepVerdict {
                seq: 1,
                id: StepId("s".to_string()),
                anchor: Location::new("src/ch.md", 1),
                expect: Expect::CompileFail,
                verdict: Verdict::Upheld,
                outputs: outputs
                    .into_iter()
                    .map(|(line, result, live)| OutputVerdict {
                        loc: Location::new("src/ch.md", line),
                        capture: Capture::Check,
                        live: v(live),
                        result,
                    })
                    .collect(),
            }],
            unpinned_step: None,
        }
    }

    const CH: &str = concat!(
        "<!-- bower repo=\"r\" output=\"check\" -->\n", // 1
        "```text\n[...]\nerror: kept\n[...]\n```\n",     // 2-6
        "<!-- bower repo=\"r\" output=\"check\" -->\n", // 7
        "```text\nerror: old\n```\n",                    // 8-10
    );

    fn texts() -> BTreeMap<String, String> {
        BTreeMap::from([("src/ch.md".to_string(), CH.to_string())])
    }

    #[test]
    fn record__keeps_a_trim_that_still_matches() {
        let recs = recordings(&[report(vec![(1, OutputResult::Matches, &["a", "error: kept", "b"])])]);
        assert!(recs.is_empty());
        assert!(apply(&texts(), &recs).is_empty(), "nothing changed, nothing written");
    }

    #[test]
    fn record__replaces_a_drifted_trim_with_the_full_output() {
        let drifted = OutputResult::Drifted(Drift {
            line: 2,
            recorded: Some("error: kept".to_string()),
            live: None,
        });
        let recs = recordings(&[report(vec![(1, drifted, &["a", "error: new", "b"])])]);
        let changed = apply(&texts(), &recs);
        assert!(
            changed["src/ch.md"].starts_with(
                "<!-- bower repo=\"r\" output=\"check\" -->\n```text\na\nerror: new\nb\n```\n"
            ),
            "{changed:?}"
        );
    }

    #[test]
    fn record__never_writes_an_unjudged_output() {
        assert!(recordings(&[report(vec![(7, OutputResult::Unjudged, &[])])]).is_empty());
    }

    #[test]
    fn record__rewrites_bottom_up_so_both_fences_land() {
        let recs = recordings(&[report(vec![
            (1, OutputResult::NotRecorded, &["one", "two", "three"]),
            (7, OutputResult::NotRecorded, &["four"]),
        ])]);
        let changed = apply(&texts(), &recs);
        assert_eq!(
            changed["src/ch.md"],
            concat!(
                "<!-- bower repo=\"r\" output=\"check\" -->\n```text\none\ntwo\nthree\n```\n",
                "<!-- bower repo=\"r\" output=\"check\" -->\n```text\nfour\n```\n",
            )
        );
    }

    #[test]
    fn record__refuses_a_chapter_path_that_climbs_out() {
        let dir = std::env::temp_dir().join("bower-record-escape").join("book");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        let changed = BTreeMap::from([("../ESCAPED.md".to_string(), "no".to_string())]);
        let err = write_chapters(&dir, &changed).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(!dir.parent().unwrap().join("ESCAPED.md").exists());
    }
}
```

Add `pub mod record;` to `bower/src/lib.rs` after `pub mod publish;`.

Run: `cargo test -p bower --lib record__`
Expected: compile error, `cannot find function `recordings``.

- [ ] **Step 2: Implement**

Between the imports and the test module of `bower/src/record.rs`:

```rust
/// One fence to rewrite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recording {
    /// Book-root-relative, as the loader names chapters: `src/ch04-….md`.
    pub chapter: String,
    /// The output directive's 1-based line.
    pub line: usize,
    /// The normalized live output to write.
    pub lines: Vec<String>,
}

/// Which fences `--record` rewrites: every output of an upheld step that is
/// not recorded yet or has drifted. Nothing else.
#[must_use]
pub fn recordings(reports: &[VerifyReport]) -> Vec<Recording> {
    reports
        .iter()
        .flat_map(VerifyReport::outputs)
        .filter(|(_, o)| {
            matches!(
                o.result,
                OutputResult::NotRecorded | OutputResult::Drifted(_)
            )
        })
        .map(|(_, o)| Recording {
            chapter: o.loc.chapter.clone(),
            line: o.loc.line,
            lines: o.live.clone(),
        })
        .collect()
}

/// Fold the recordings into the chapter texts. Bottom-up within a chapter: a
/// fence that grows moves every line below it, and none above. Returns only
/// the chapters whose text changed.
#[must_use]
pub fn apply(texts: &BTreeMap<String, String>, recs: &[Recording]) -> BTreeMap<String, String> {
    let mut by_chapter: BTreeMap<&str, Vec<&Recording>> = BTreeMap::new();
    for r in recs {
        by_chapter.entry(r.chapter.as_str()).or_default().push(r);
    }
    let mut changed = BTreeMap::new();
    for (chapter, mut list) in by_chapter {
        let Some(original) = texts.get(chapter) else {
            continue;
        };
        list.sort_by(|a, b| b.line.cmp(&a.line));
        let text = list
            .iter()
            .fold(original.clone(), |t, r| rewrite(&t, r.line, &r.lines));
        if &text != original {
            changed.insert(chapter.to_string(), text);
        }
    }
    changed
}

/// Write each changed chapter under `book_root`. Every path is checked before
/// anything is written: chapter paths come from the book, which is not trusted
/// to name only its own files (`docs/DEFECT_Path_Traversal.md`).
///
/// # Errors
///
/// An unsafe path (`InvalidInput`), or a failed write.
pub fn write_chapters(book_root: &Path, changed: &BTreeMap<String, String>) -> std::io::Result<()> {
    for path in changed.keys() {
        check_path(path)?;
    }
    for (path, text) in changed {
        std::fs::write(book_root.join(path), text)?;
    }
    Ok(())
}
```

If `flat_map(VerifyReport::outputs)` does not infer, use
`.flat_map(|r| r.outputs())`.

Run: `cargo test -p bower --lib record__` — PASS.

- [ ] **Step 3: Wire the flag**

In `bower/src/main.rs`, in `Command::Verify`, after `work`:

```rust
        /// Write what each command printed into the output fences that are
        /// empty or no longer match, then rewrite `bower.lock`. A fence that
        /// still matches is left alone, so a `[...]` trim survives.
        #[arg(long)]
        record: bool,
```

Pass it through `main`'s match (`Command::Verify { repo, step, from, work, record } => run_verify(…, work.as_deref(), record)`) and add `record: bool` as `run_verify`'s last parameter.

Pull the lock write out of `run_plan` so both commands share it:

```rust
/// Write `bower.lock` for `resolved`. One function, so `plan` and
/// `verify --record` cannot write two different formats.
fn write_lock(book_root: &Path, resolved: &BookPlan) -> Result<PathBuf, String> {
    let lock_path = book_root.join("bower.lock");
    std::fs::write(&lock_path, lock_text(resolved))
        .map_err(|e| format!("cannot write {}: {e}", lock_path.display()))?;
    Ok(lock_path)
}
```

and in `run_plan` replace the lock-writing block with:

```rust
    match write_lock(book_root, &resolved) {
        Ok(path) => {
            println!("\nwrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bower: {e}");
            ExitCode::FAILURE
        }
    }
```

Add:

```rust
/// `--record`: rewrite every fence whose output is missing or has drifted,
/// then re-plan and rewrite the lock — a fence's new length moves every later
/// anchor in its chapter (EPIC-11 Decision 8). Returns how many fences were
/// written.
fn record_outputs(
    book_root: &Path,
    cfg: &BookConfig,
    reports: &[VerifyReport],
) -> Result<usize, String> {
    let recs = bower::record::recordings(reports);
    if recs.is_empty() {
        return Ok(0);
    }
    let book = BookLoader::new(book_root).load().map_err(|e| e.to_string())?;
    let texts: std::collections::BTreeMap<String, String> = book
        .chapters
        .iter()
        .map(|c| (c.path.clone(), c.text.clone()))
        .collect();
    let changed = bower::record::apply(&texts, &recs);
    bower::record::write_chapters(book_root, &changed).map_err(|e| e.to_string())?;
    let resolved = resolve(book_root, cfg).ok_or("the book no longer resolves after recording")?;
    write_lock(book_root, &resolved)?;
    Ok(recs.len())
}
```

In `run_verify`, before the `let (drifted, unrecorded) = …` line from Task 10,
insert:

```rust
    if record {
        match record_outputs(book_root, cfg, &reports) {
            Ok(0) => println!("\nnothing to record"),
            Ok(n) => println!("\nrecorded {n} output(s) and rewrote bower.lock"),
            Err(e) => {
                eprintln!("bower: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
```

and make the problems line honour it:

```rust
    let (drifted, unrecorded) = if record {
        (0, 0)
    } else {
        print_output_problems(&reports)
    };
```

- [ ] **Step 4: Run and lint**

Run: `cargo test -p bower && cargo clippy -p bower --all-targets --all-features -- -D warnings`
Expected: PASS. `cargo run -q -p bower -- verify --help` lists `--record`.

- [ ] **Step 5: Commit**

```bash
git add bower/src/record.rs bower/src/lib.rs bower/src/main.rs
git commit -m "feat(verify): --record writes drifted and missing outputs, then the lock"
```

---
### Task 12: The caption, on all three targets (EPIC Phase 4a–4c, open question 4)

**Files:**
- Modify: `bower/src/config.rs` (`RUSTC_ERROR_CODES`, `LinkTemplates::error_code`, `WireLinks::error_code`, tests)
- Modify: `bower/src/render.rs` (`outputs_by_line`, `output_caption`, `chapter`, tests)
- Modify: `books/hello-playbook/theme/step-meta.css`, `books/rust4failures/theme/step-meta.css`

**Interfaces:**
- Consumes: `PlannedStep::outputs`, `CapturedOutput`, `error_codes` (kernel).
- Produces: `pub const RUSTC_ERROR_CODES: &str`; `LinkTemplates::error_code: Option<String>` (a `{code}` template); `pub fn render::output_caption(step: &PlannedStep, output: &CapturedOutput, repo: &str, links: Option<&LinkTemplates>) -> String`.

- [ ] **Step 1: Write the failing config tests**

In `bower/src/config.rs`, `mod config_tests`:

```rust
    #[test]
    fn config__a_cargo_repo_links_error_codes_to_rustc() {
        let cfg = BookConfig::parse(&format!("{MINIMAL}[repos.r]\n")).unwrap();
        assert_eq!(
            cfg.repos.get("r").unwrap().links.error_code.as_deref(),
            Some(RUSTC_ERROR_CODES)
        );
    }

    #[test]
    fn config__a_declared_error_code_link_wins() {
        let text = format!(
            "{MINIMAL}[repos.r]\n[repos.r.links]\nerror_code = \"https://codes.invalid/{{code}}\"\n"
        );
        let cfg = BookConfig::parse(&text).unwrap();
        assert_eq!(
            cfg.repos.get("r").unwrap().links.error_code.as_deref(),
            Some("https://codes.invalid/{code}")
        );
    }

    #[test]
    fn config__a_repo_that_is_not_cargo_gets_no_error_code_link() {
        // rustc's codes mean nothing to `make check`. No link beats a wrong one.
        let text = format!("{MINIMAL}[repos.r]\ncheck = \"make check\"\n");
        let cfg = BookConfig::parse(&text).unwrap();
        assert_eq!(cfg.repos.get("r").unwrap().links.error_code, None);
    }
```

Run: `cargo test -p bower --lib config__` — compile error.

- [ ] **Step 2: Add the template**

After `DEFAULT_VERIFY`:

```rust
/// Where rustc explains an error code. The `links.error_code` a repo gets when
/// its `check` command is cargo and `[links]` declares none — derived the way
/// `fork` is derived from `github`.
pub const RUSTC_ERROR_CODES: &str = "https://doc.rust-lang.org/error_codes/{code}.html";
```

`LinkTemplates` gains, after `verify`:

```rust
    /// Where an error code in a recorded output links:
    /// `https://…/{code}.html`. Declarable in `[links]`; derived from a cargo
    /// `check` command when absent (EPIC-11).
    pub error_code: Option<String>,
```

`WireLinks` gains `error_code: Option<String>,`. In `parse`, after the
`let verify = …;` statement:

```rust
                    let error_code = r.links.error_code.clone().or_else(|| {
                        check
                            .as_deref()
                            .is_some_and(|c| c.split_whitespace().next() == Some("cargo"))
                            .then(|| RUSTC_ERROR_CODES.to_string())
                    });
```

and in the `LinkTemplates { … }` literal, after `verify,`:

```rust
                                error_code,
```

Run: `cargo test -p bower --lib config__` — PASS.

- [ ] **Step 3: Write the failing render tests**

In `bower/src/render.rs`, `mod render_tests`:

```rust
    const OUTPUT_CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"broken\" file=\"src/lib.rs\" expect=\"compile_fail\" -->\n\n",
        "```rust\n",
        "fn x() -> u32 { \"42\" }\n",
        "```\n\n",
        "<!-- bower repo=\"r\" output=\"check\" -->\n\n",
        "```text\n",
        "error[E0308]: mismatched types\n",
        "[...]\n",
        "```\n\n",
        "After.\n",
    );

    fn output_links() -> BTreeMap<String, LinkTemplates> {
        let mut m = github_links();
        let l = m.get_mut("r").unwrap();
        l.check = Some("cargo check".to_string());
        l.error_code = Some("https://doc.rust-lang.org/error_codes/{code}.html".to_string());
        m
    }

    fn render_output(target: Target, links: &BTreeMap<String, LinkTemplates>) -> String {
        chapter(OUTPUT_CH, "src/ch01.md", &tiny_plan(OUTPUT_CH), links, target)
    }

    #[test]
    fn render__output_block_keeps_its_fence_and_gains_a_caption() {
        let out = render_output(Target::Html, &output_links());
        assert!(
            out.contains(concat!(
                "<span class=\"step-output\"><sub>$ cargo check · step 001 of r · ",
                "[E0308](https://doc.rust-lang.org/error_codes/E0308.html)</sub></span>\n\n",
                "```text\nerror[E0308]: mismatched types\n[...]\n```\n",
            )),
            "{out}"
        );
        assert!(!out.contains("<!-- bower"), "{out}");
        assert!(out.contains("After."), "{out}");
    }

    #[test]
    fn render__error_code_links_through_the_template() {
        let mut links = output_links();
        links.get_mut("r").unwrap().error_code = Some("https://codes.invalid/{code}".to_string());
        let out = render_output(Target::Html, &links);
        assert!(out.contains("[E0308](https://codes.invalid/E0308)"), "{out}");
    }

    #[test]
    fn render__no_template_no_link() {
        // No repo links at all: the verifier's default command, and the code
        // as plain text.
        let out = render_output(Target::Html, &no_links());
        assert!(
            out.contains("<sub>$ cargo check · step 001 of r · E0308</sub>"),
            "{out}"
        );
        assert!(!out.contains("](http"), "{out}");
    }

    #[test]
    fn render__epub_caption_matches_html() {
        let caption = |out: &str| {
            out.lines()
                .find(|l| l.contains("step-output"))
                .map(str::to_string)
        };
        let html = caption(&render_output(Target::Html, &output_links()));
        assert!(html.is_some());
        assert_eq!(html, caption(&render_output(Target::Epub, &output_links())));
        assert_eq!(html, caption(&render_output(Target::Pdf, &output_links())));
    }
```

Run: `cargo test -p bower --lib render__output` — FAIL (no caption).

- [ ] **Step 4: Implement the caption**

In `bower/src/render.rs`, the imports become:

```rust
use bower_core::prelude::{
    BlockDisplay, BookPlan, Capture, CapturedOutput, Directive, Exercise, ExerciseForm, Expect,
    LineRange, PlannedStep, ShowMark, StepId, error_codes, show_marker,
};

use crate::config::{DEFAULT_CHECK, DEFAULT_VERIFY, LinkTemplates};
```

In `chapter`, after `let folds = folds_by_line(plan, chapter_path);`:

```rust
    let outputs = outputs_by_line(plan, chapter_path);
```

Change the header block from `if let Some((repo, step, display)) = blocks.get(&directive_line) { … }`
to add an `else if` arm:

```rust
        if let Some((repo, step, display)) = blocks.get(&directive_line) {
            // … unchanged …
        } else if let Some((repo, step, output)) = outputs.get(&directive_line) {
            // A recorded output: the command that printed it, and a link for
            // each error code. In a caption, because markdown does not render
            // inside a fence. The fence itself passes through untouched.
            out.push(output_caption(step, output, repo, forge.get(repo)));
            // The blank lines between directive and fence are copied next;
            // add a separator only when the book left none.
            if j == i {
                out.push(String::new());
            }
        }
```

(`i` is the line after the directive and `j` the fence, both already computed
at this point in `chapter`.)

After `checkout_line`:

```rust
/// The line above a recorded output: the command that printed it, the step,
/// and each rustc error code it names — linked when the repo has an
/// `error_code` template, plain text when it has none.
///
/// One string for every target, as `step-meta` is: the class is the tool's
/// whole opinion about looks.
#[must_use]
pub fn output_caption(
    step: &PlannedStep,
    output: &CapturedOutput,
    repo: &str,
    links: Option<&LinkTemplates>,
) -> String {
    let command = match output.capture {
        Capture::Check => links
            .and_then(|l| l.check.as_deref())
            .unwrap_or(DEFAULT_CHECK),
        Capture::Verify => links
            .and_then(|l| l.verify.as_deref())
            .unwrap_or(DEFAULT_VERIFY),
    };
    let mut parts = vec![format!("$ {command}"), format!("step {:03} of {repo}", step.seq)];
    for code in error_codes(&output.lines) {
        parts.push(match links.and_then(|l| l.error_code.as_deref()) {
            Some(template) => format!("[{code}]({})", template.replace("{code}", &code)),
            None => code,
        });
    }
    format!(
        "<span class=\"step-output\"><sub>{}</sub></span>",
        parts.join(" · ")
    )
}
```

After `blocks_by_line`:

```rust
/// Line number → the output block that directive introduces, with its step
/// and repo.
fn outputs_by_line<'a>(
    plan: &'a BookPlan,
    chapter_path: &str,
) -> BTreeMap<usize, (String, &'a PlannedStep, &'a CapturedOutput)> {
    let mut out = BTreeMap::new();
    for repo in &plan.repos {
        for step in &repo.steps {
            for o in &step.outputs {
                if o.loc.chapter == chapter_path {
                    out.insert(o.loc.line, (repo.repo.0.clone(), step, o));
                }
            }
        }
    }
    out
}
```

Run: `cargo test -p bower --lib render_` — PASS, old render tests included.

- [ ] **Step 5: Style the caption**

Append to both `books/hello-playbook/theme/step-meta.css` and
`books/rust4failures/theme/step-meta.css` (the two files are identical today):

```css
/* The caption bower prints above a recorded output: the command that printed
   it, the step, and a link for each error code. Same box as .step-meta, so a
   caption reads the same whether its fence holds code or what the compiler
   said about it. */
.step-output {
    margin: 0.5em 0 0.5em 1.5em;
    padding: 0.4em 0.8em;
    display: inline-block;
    border: 1px solid var(--table-border-color, #ccc);
    border-radius: 4px;
    background: var(--table-alternate-bg, rgba(127, 127, 127, 0.08));
}
```

- [ ] **Step 6: Run and lint**

Run: `cargo test -p bower && cargo clippy -p bower --all-targets --all-features -- -D warnings`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add bower/src/config.rs bower/src/render.rs books/hello-playbook/theme/step-meta.css books/rust4failures/theme/step-meta.css
git commit -m "feat(render): caption a recorded output with its command, step, and error-code links"
```

---
### Task 13: The books record their outputs; the goldens prove it (EPIC Phase 5a–5b; exit criteria 1–5)

**Files:**
- Modify: `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md`, `books/hello-playbook/bower.lock`
- Modify: `books/rust4failures/bower.toml`, `books/rust4failures/src/ch01-local_development.md`, `books/rust4failures/src/ch03-rank.md`, `books/rust4failures/bower.lock`
- Modify: `bower/tests/verification.rs`, `bower/tests/determinism.rs`, `bower/tests/preprocessor.rs`, `bower-testkit/tests/sample_book.rs`

**Interfaces:**
- Consumes: everything above.
- Produces: two recorded outputs in `hello-playbook` (steps `test-that-fails`, `wont-compile`), one trimmed output in `rust4failures` (step `kata-compiles`), one unplanned trimmed output in `ch03-rank.md`.

- [ ] **Step 1: Add the two directives to `hello-playbook`, empty**

In `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md`, after the
paragraph ending "The next section is one answer, and it stays folded until you
open it." and before `## The fix`, insert:

````markdown

Here is what `cargo test` prints at this step:

<!-- bower repo="hello-playbook" output="verify" -->

```text
```

That is not a paste. The `output="verify"` directive above binds the block to
this step, and `bower verify` runs the tests and fails the day what they print
changes. `bower verify --record` is what filled it in.
````

After the paragraph ending "it belongs to the region, not to the file around
it." and before `## The repair`, insert:

````markdown

And here is what the compiler says about it, checked the same way:

<!-- bower repo="hello-playbook" output="check" -->

```text
```
````

- [ ] **Step 2: Watch verify refuse the empty fences**

Run: `cargo run -q -p bower -- --book books/hello-playbook verify --step wont-compile`
Expected: exit 1; stdout has `       todo output=check  src/ch04-tests-and-failing-on-purpose.md:…`;
stderr says `is not recorded yet.` and `run: bower verify --record --step wont-compile`.

- [ ] **Step 3: Record them**

Run:

```bash
cargo run -q -p bower -- --book books/hello-playbook verify --record --step test-that-fails
cargo run -q -p bower -- --book books/hello-playbook verify --record --step wont-compile
```

Expected: each prints `recorded 1 output(s) and rewrote bower.lock`, then
`every claim holds`. The `verify` fence now holds exactly:

```text
running 2 tests
test tests::greet_ignores_stray_whitespace ... FAILED
test tests::greet_uses_the_name ... ok

failures:

---- tests::greet_ignores_stray_whitespace stdout ----

thread 'tests::greet_ignores_stray_whitespace' panicked at src/lib.rs:20:9:
assertion `left == right` failed
  left: "Hello,   world  !"
 right: "Hello, world!"
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    tests::greet_ignores_stray_whitespace

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

error: test failed, to rerun pass `--lib`
```

and the `check` fence:

```text
error[E0308]: mismatched types
 --> src/scratch.rs:4:18
  |
4 |     let n: u32 = "42";
  |            ---   ^^^^ expected `u32`, found `&str`
  |            |
  |            expected due to this

For more information about this error, try `rustc --explain E0308`.
error: could not compile `hello-playbook` (lib) due to 1 previous error
```

If either differs, stop: find which normalization rule let a machine detail
through, fix it with a unit test in `bower-core/src/capture.rs`, and record
again. Do not hand-edit a fence to make it match.

- [ ] **Step 4: Prove it is stable**

Run:

```bash
cargo run -q -p bower -- --book books/hello-playbook verify --record --step wont-compile
cargo run -q -p bower -- --book books/hello-playbook verify --record --step test-that-fails
git diff --stat books/
```

Expected: both print `nothing to record`; the diff shows only
`ch04-tests-and-failing-on-purpose.md` and `bower.lock` (exit criterion 3).

- [ ] **Step 5: Extend and add the verification goldens**

In `bower/tests/verification.rs`, in `upholds_the_two_deliberate_failures`,
after `assert!(stdout.contains("ok  "), "{stdout}");`:

```rust
        // Both deliberate failures quote what the compiler said, and both
        // quotes still hold.
        assert!(stdout.contains("       ok   output="), "{stdout}");
        assert!(!stdout.contains("drft") && !stdout.contains("todo"), "{stdout}");
```

Add helpers and three tests:

```rust
fn chapter_of(book: &Path) -> PathBuf {
    book.join("src").join("ch04-tests-and-failing-on-purpose.md")
}

/// The 1-based line of the first directive in `text` containing `needle`.
fn directive_line(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains("<!-- bower") && l.contains(needle))
        .map(|i| i + 1)
        .expect("the directive is in the chapter")
}

/// The test that proves the output check can say no (EPIC-11 exit
/// criterion 2), as `reports_a_wrong_expectation_as_broken` does for `expect`.
#[test]
fn reports_a_drifted_diagnostic_as_broken() {
    let book = scratch("drift-book");
    copy_dir(&book_root(), &book);
    let chapter = chapter_of(&book);
    let text = std::fs::read_to_string(&chapter).unwrap();
    let edited = text.replacen(
        "error[E0308]: mismatched types",
        "error[E0308]: mismatched typos",
        1,
    );
    assert_ne!(edited, text, "the fixture must actually change");
    std::fs::write(&chapter, edited).unwrap();

    let out = verify(&book, &scratch("drift-work"), &["--step", "wont-compile"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(!out.status.success(), "a drifted output must fail:\n{stdout}");
    assert!(stdout.contains("drft output=check"), "{stdout}");
    assert!(
        stderr.contains("ch04-tests-and-failing-on-purpose.md"),
        "the report must name the chapter:\n{stderr}"
    );
    assert!(stderr.contains("at line 1 of the recording"), "{stderr}");
    assert!(
        stderr.contains("mismatched typos") && stderr.contains("mismatched types"),
        "the report must show both sides:\n{stderr}"
    );
    assert!(
        stderr.contains("bower verify --record --step wont-compile"),
        "{stderr}"
    );
}

#[test]
fn record__rewrites_the_chapter_and_the_lock() {
    let book = scratch("record-book");
    copy_dir(&book_root(), &book);
    let chapter = chapter_of(&book);
    let original = std::fs::read_to_string(&chapter).unwrap();
    let line = directive_line(&original, "output=\"check\"");
    std::fs::write(&chapter, bower_core::prelude::rewrite(&original, line, &[])).unwrap();
    std::fs::write(book.join("bower.lock"), "stale\n").unwrap();

    let out = verify(
        &book,
        &scratch("record-work"),
        &["--step", "wont-compile", "--record"],
    );
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Recording on this machine restores exactly what the book holds, which
    // was recorded on another run (exit criterion 4, on one machine).
    assert_eq!(std::fs::read_to_string(&chapter).unwrap(), original);
    assert_eq!(
        std::fs::read_to_string(book.join("bower.lock")).unwrap(),
        std::fs::read_to_string(book_root().join("bower.lock")).unwrap(),
        "the lock must be rewritten, and consistent with the chapter"
    );
}

#[test]
fn record__is_a_no_op_when_nothing_changed() {
    let book = scratch("noop-book");
    copy_dir(&book_root(), &book);
    let chapter = chapter_of(&book);
    let before = std::fs::read_to_string(&chapter).unwrap();
    std::fs::write(book.join("bower.lock"), "untouched\n").unwrap();

    let out = verify(
        &book,
        &scratch("noop-work"),
        &["--step", "wont-compile", "--record"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("nothing to record"), "{stdout}");
    assert_eq!(std::fs::read_to_string(&chapter).unwrap(), before);
    assert_eq!(
        std::fs::read_to_string(book.join("bower.lock")).unwrap(),
        "untouched\n",
        "nothing recorded, so the lock is not touched"
    );
}
```

In `upholds_every_step` (the `#[ignore]`d sweep), after the `steps_ok`
assertion from Task 10:

```rust
    assert_eq!(stdout.matches("       ok   output=").count(), 2, "{stdout}");
```

Run: `cargo test -p bower --test verification`
Expected: PASS.

- [ ] **Step 6: The SHA golden**

In `bower/tests/determinism.rs`, split `build` so a copied book can be built:

```rust
/// Run `bower build` and return the HEAD id it reports.
fn build(out: &Path) -> String {
    build_from(&book_root(), out)
}

/// `build`, for any book.
fn build_from(book: &Path, out: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book)
        // … the rest of the old `build` body, unchanged …
```

Add a `copy_dir` helper (the same function as in `bower/tests/verification.rs`,
skipping a `book` directory) and:

```rust
/// Recording an output rewrites a fence, which moves every later line of its
/// chapter. Commits name steps by id, never by line, so no SHA may move
/// (EPIC-11 Decision 9, exit criterion 5). No compiler needed: any new fence
/// body proves it.
#[test]
fn replay__recording_an_output_moves_no_sha() {
    let book = scratch("sha-book");
    copy_dir(&book_root(), &book);
    let before = build_from(&book, &scratch("sha-before"));

    let chapter = book.join("src").join("ch04-tests-and-failing-on-purpose.md");
    let text = std::fs::read_to_string(&chapter).unwrap();
    let line = text
        .lines()
        .position(|l| l.contains("output=\"check\""))
        .unwrap()
        + 1;
    let longer: Vec<String> = ["a", "longer", "recording", "moves", "every later line"]
        .iter()
        .map(ToString::to_string)
        .collect();
    std::fs::write(&chapter, bower_core::prelude::rewrite(&text, line, &longer)).unwrap();

    let after = build_from(&book, &scratch("sha-after"));
    assert_eq!(before, after, "a fence body is not part of any commit");
}
```

If `bower-core` is not yet a dev-dependency of `bower`, it does not need to
be: it is a normal dependency, and integration tests see normal dependencies.

Run: `cargo test -p bower --test determinism` — PASS.

- [ ] **Step 7: The preprocessor and the testkit see the outputs**

In `bower/tests/preprocessor.rs`:

```rust
#[test]
fn sample_book_captions_the_recorded_outputs() {
    let ch04 = &rendered_chapters()[3];
    assert!(
        ch04.contains(
            "<span class=\"step-output\"><sub>$ cargo test · step 011 of hello-playbook</sub></span>"
        ),
        "{ch04}"
    );
    assert!(
        ch04.contains(concat!(
            "<span class=\"step-output\"><sub>$ cargo check · step 013 of hello-playbook · ",
            "[E0308](https://doc.rust-lang.org/error_codes/E0308.html)</sub></span>"
        )),
        "{ch04}"
    );
    assert!(ch04.contains("error[E0308]: mismatched types"), "{ch04}");
}
```

In `bower-testkit/tests/sample_book.rs`:

```rust
#[test]
fn records_what_the_compiler_says_at_both_failures() {
    let p = book_plan();
    let repo = p.repo(REPO).expect("repo present");
    let outputs_of = |id: &str| {
        repo.steps
            .iter()
            .find(|s| s.id.0 == id)
            .unwrap_or_else(|| panic!("no step `{id}`"))
            .outputs
            .clone()
    };

    let red = outputs_of("test-that-fails");
    assert_eq!(red.len(), 1);
    assert_eq!(red[0].capture, Capture::Verify);
    assert!(
        red[0].lines.iter().any(|l| {
            l == "thread 'tests::greet_ignores_stray_whitespace' panicked at src/lib.rs:20:9:"
        }),
        "{:#?}",
        red[0].lines
    );

    let broken = outputs_of("wont-compile");
    assert_eq!(broken[0].capture, Capture::Check);
    assert_eq!(broken[0].lines[0], "error[E0308]: mismatched types");
    assert_eq!(error_codes(&broken[0].lines), vec!["E0308"]);
}
```

Run: `cargo test -p bower --test preprocessor && cargo test -p bower-testkit --test sample_book`
Expected: PASS.

- [ ] **Step 8: `rust4failures` — pin the toolchain, trim the quotes**

In `books/rust4failures/bower.toml`, after `verify = "cargo test"`:

```toml
# The compiler for every step whose tree has no rust-toolchain.toml of its
# own. The book records what the compiler says, so which compiler says it is
# part of the book. Matches the workspace pin in the repository root.
toolchain = "1.98.1"
```

In `books/rust4failures/src/ch01-local_development.md`, replace the fence

````markdown
```console
$ cargo test
thread 'tests::hello_world' panicked at src/lib.rs:31:9:
assertion `left == right` failed
  left: "Hello, world!"
 right: "Hello,  wirld!!"
```
````

with

````markdown
<!-- bower repo="rust4failures" output="verify" -->

```text
[...]
thread 'tests::hello_world' panicked at src/lib.rs:31:9:
assertion `left == right` failed
  left: "Hello, world!"
 right: "Hello,  wirld!!"
[...]
```
````

The caption prints `$ cargo test`, which is why the fence no longer does.

In `books/rust4failures/src/ch03-rank.md`, replace

````markdown
```console
$ cargo check
error[E0004]: non-exhaustive patterns: `'\0'..='/'`, `'1'`, `':'..='@'` and 9 more not covered
```
````

with

````markdown
<!-- bower repo="rust4failures" output="check" -->

```text
[...]
error[E0004]: non-exhaustive patterns: `'\0'..='/'`, `'1'`, `':'..='@'` and 9 more not covered
[...]
```
````

`ch03-rank.md` is commented out of `SUMMARY.md`, so this block is not planned
until the chapter is listed; the first `verify` after that checks it.

- [ ] **Step 9: Verify `rust4failures`**

Run:

```bash
cargo run -q -p bower -- --book books/rust4failures plan
cargo run -q -p bower -- --book books/rust4failures verify
```

Expected: `ok   002 kata-compiles` with `       ok   output=verify src/ch01-local_development.md:…`,
no toolchain warning, and `every claim holds`. If the row says `drft`, read
the report: the trim must still hold on 1.98.1. Do not run `--record` here
without reading it — `--record` on a drifted trim writes the full output,
which is correct, but the author should choose the new trim.

- [ ] **Step 10: The whole suite and the book targets**

Run:

```bash
make plan
make test
make slow
make book && make epub
make failures
```

Expected: all green; `git diff --stat books/` lists only the chapters,
`bower.toml`, the two locks, and the two CSS files.

- [ ] **Step 11: Commit**

```bash
git add books bower/tests bower-testkit/tests
git commit -m "feat(books): record what the compiler says at every deliberate failure"
```

---
### Task 14: Spec, README, OKF, backlog, debt, and the EPIC (EPIC Phase 5c–5e)

**Files:**
- Modify: `bower-spec.md` (§ 3.2 table, § 6 table, § 12 Q3)
- Modify: `README.md` (a "Recorded output" section after "Exercises")
- Modify: `.okf/model/directive.md`, `.okf/model/index.md`, `.okf/model/errors.md`, `.okf/commands/verify.md`, `.okf/config/bower-toml.md`, `.okf/log.md`
- Create: `.okf/model/captured-output.md`
- Modify: `docs/TECHNICAL_DEBT.md`, `BACKLOG.md`, `docs/EPIC-11_Diagnostics.md`

- [ ] **Step 1: The spec**

`bower-spec.md` § 3.2 key table, after the `exercise` row:

```markdown
| `output` | no | — | `check` \| `verify`: the fence holds what that command printed at the bound step, normalized. `bower verify` compares; `bower verify --record` writes. A `[...]` line elides. See `docs/EPIC-11_Diagnostics.md` |
```

§ 6 table — replace the `compile_fail` and `test_fail` rows:

```markdown
| `compile_fail` | `cargo check` **fails**. An `output="check"` block, if the book has one, must match what it printed (EPIC-11) |
| `test_fail` | Build succeeds, tests **fail**. An `output="verify"` block, if present, records which test and how (EPIC-11) |
```

§ 12 Q3 — replace the item with:

```markdown
3. ~~**Diagnostic snapshots for `compile_fail`**~~ **Decided: failure-only by
   default; visible opt-in snapshots** (EPIC-02, 1 September 2026; amended by
   EPIC-11, 11 September 2026). `expect` still asserts only *that* a step
   fails. A book that quotes *what* the compiler said puts an `output="check"`
   or `output="verify"` block after the step; `bower verify` compares its
   normalized text and `--record` writes it. The snapshot is book content,
   rendered and diffed in review — never a hidden fixture.
```

- [ ] **Step 2: The README**

In `README.md`, after the "Exercises" section's last paragraph and before
`## License`:

`````markdown
### Recorded output

A step that fails on purpose can show what the compiler said, and prove it.
Put an `output` directive after the step, with an empty fence:

````markdown
<!-- bower repo="hello-playbook" output="check" -->
```text
```
````

`bower verify --record` fills the fence with what the repo's `check` (or
`verify`) command printed at that step, with paths, colour, timings, build
chatter and thread IDs taken out. From then on `bower verify` fails the day the
compiler says something else. A line that is exactly `[...]` stands for lines
left out, so a fence can quote only what matters; `--record` keeps that trim
while it still matches. The rendered page gets a caption with the command and a
link for each error code.

Which compiler says it matters, so a step runs on its tree's own
`rust-toolchain.toml`, or else on the repo's `toolchain` key in `bower.toml`.
Design: `docs/EPIC-11_Diagnostics.md`.
`````

- [ ] **Step 3: The OKF bundle**

`.okf/model/directive.md` key table, after the `exercise` row:

```markdown
| `output` | no | — | `check` \| `verify`: what that command printed at the bound step. See [captured output](/model/captured-output.md) |
```

`.okf/model/index.md`, under "What the reader sees", after the Exercise entry:

```markdown
* [Captured output](captured-output.md) - what the compiler said at a step, recorded in the book and checked on every verify.
```

Create `.okf/model/captured-output.md`:

````markdown
---
type: Domain Concept
title: Captured output
description: What a step's check or verify command printed, recorded as a fence in the chapter and checked by bower verify.
tags: [model, verification, rendering]
timestamp: '2026-09-11T00:00:00Z'
---

# What it is

An `output="check"` or `output="verify"` directive, followed by a fence, binds
to a [step](/model/step.md) the way a [play cell](/model/play-cell.md) does: an
explicit `step=`, else the nearest preceding code block of the repo. It never
touches a tree, so recording one moves no SHA.

The fence holds the **normalized** output of that command at that step. An
empty fence means "not recorded yet".

# Normalization

Eight rules, in order, all pure kernel code (`bower-core/src/capture.rs`):
CRLF to LF; ANSI escapes removed; the scratch tree, target directory, and
`$CARGO_HOME` replaced; cargo's right-aligned status lines dropped; the
panicking thread's ID removed; `; finished in N.NNs` removed; runs of
`test … ok` lines sorted; trailing whitespace and outer blank lines trimmed.
Cargo's summary lines (`error: could not compile …`) stay.

# Matching

A fence without a `[...]` line must equal the live output. A `[...]` line
stands for zero or more lines; the other lines must appear in order, and a
piece at either end is anchored there.

# The verifier

Only an **upheld** step's outputs are judged. A step with an output block runs
with one build job, one test thread, and `RUST_BACKTRACE=0`. Rows are
`ok` / `drft` / `todo` / `skip`; a drift or a missing recording fails
[`bower verify`](/commands/verify.md). `--record` rewrites only drifted and
missing fences, then the [lock](/config/bower-lock.md).

# Errors

| Error | Rule |
|---|---|
| `OutputUnbound` | No preceding step to attach to. |
| `OutputUnknownStep` | Names a step that does not exist. |
| `OutputConflictingKeys` | Carries a tree key, `notebook`, `exercise`, `expect`, or `include`. |
| `OutputDuplicate` | A second block for one step and capture. |
| `OutputNeverRuns` | `verify` on a `compile_fail` step, or anything on a `none` step. |

# Citations

[1] `bower-core/src/capture.rs`, `bower-core/src/plan.rs`, `bower/src/verify.rs`, `bower/src/record.rs`
[2] `docs/EPIC-11_Diagnostics.md`
````

`.okf/model/errors.md` and `.okf/model/index.md`: the variant count goes from
32 to 37 (`grep -n "32" .okf/model/errors.md .okf/model/index.md` finds each
mention), and `errors.md` gains the five rows from the table above, in the
same shape as its exercise rows.

`.okf/commands/verify.md`:
- Usage line becomes `bower --book <DIR> verify [--repo <NAME>] [--step <ID> | --from <ID>] [--work <DIR>] [--record]`.
- "The verdicts": `Broken { happened, command, output }`, where `output` is both streams in written order; add "Each output block gets a row under its step: `ok  ` / `drft` / `todo` / `skip`. A drift or a missing recording fails the run."
- New section `# Which toolchain`: the inherited `RUSTUP_TOOLCHAIN` is always removed; a tree's own pin wins; else the repo's `toolchain` key; a warning when an output-bearing step has neither.
- New section `# --record`: rewrites drifted and missing fences only, bottom-up, through `check_path`, then rewrites `bower.lock`; a second run prints `nothing to record`.
- Citations gain `bower/src/record.rs` and `docs/EPIC-11_Diagnostics.md`.

`.okf/config/bower-toml.md`:
- `[repos.<name>]` table, after the `check`, `verify` row: `| | \`toolchain\` | Optional. The rustup toolchain for a step whose tree pins none. A tree's own \`rust-toolchain.toml\` wins. |`
- The `[repos.<name>.links]` row lists `blob`, `tree`, `commit`, `fork`, `error_code`.
- "Derived, not declared" gains: `- \`links.error_code\` = the declared value, else \`https://doc.rust-lang.org/error_codes/{code}.html\` when \`check\`'s first word is \`cargo\``.

`.okf/log.md`, a new dated section at the top of the log:

```markdown
## 2026-09-11

* **Creation**: [Captured output](/model/captured-output.md) — EPIC-11's `output="check"|"verify"` blocks, their normalization, `[...]` matching, and five errors.
* **Update**: [bower verify](/commands/verify.md) gains `--record`, output rows, and the toolchain rule; [bower.toml](/config/bower-toml.md) gains `toolchain` and `links.error_code`.
```

Run the OKF checker: invoke the `okf:validate` skill on `.okf/`.
Expected: conformant.

- [ ] **Step 4: Debt and backlog**

`docs/TECHNICAL_DEBT.md` — replace the `- [ ] **No stderr snapshots for
\`compile_fail\`.**` item with:

```markdown
- [x] ~~**No stderr snapshots for `compile_fail`.**~~ Closed 11 September 2026
  by EPIC-11. A book records what the compiler said in an `output="check"` or
  `output="verify"` fence; `bower verify` compares its normalized text and
  `--record` writes it (`bower-core/src/capture.rs`, `bower/src/record.rs`).
  Grounding it found that `bower verify` under `make` ignored every tree's
  toolchain pin, because rustup's cargo proxy exports `RUSTUP_TOOLCHAIN` to its
  children; `run` now removes it.
```

`BACKLOG.md`:
- Remove the **Captured diagnostics** row from "Next up — not started".
- In "Ideas — not yet EPICs", the rank-1 row's EPIC cell becomes `[EPIC-11](docs/EPIC-11_Diagnostics.md) — shipped`.
- In "Known gaps", remove the `No stderr snapshots for compile_fail` line.
- In "Recently completed", add at the top:

```markdown
- **[EPIC-11 — captured diagnostics](docs/EPIC-11_Diagnostics.md)** (11 September 2026) — 9/9 components. `output="check"|"verify"` records what the compiler said as book content; `verify` fails on drift, `--record` writes it, `[...]` trims it. Found and fixed: `verify` under `make` ignored every tree's toolchain pin.
```

- [ ] **Step 5: Close the EPIC**

In `docs/EPIC-11_Diagnostics.md`:
- Every Status row becomes **Done**.
- Every Work Items checkbox becomes `- [x]`.
- Append after the settled open-questions table and before the closing italic
  note:

```markdown
---

## Corrigendum — as built, 11 September 2026

Phase status:

| Phase | Status | Notes |
|---|---|---|
| 0 (edge prerequisites) | Shipped | the inherited-toolchain defect fixed in 0b |
| 1 (key, block, binding) | Shipped | |
| 2 (`capture.rs`) | Shipped | eight rules; `[...]` matching in `drift` |
| 3 (verify, `--record`) | Shipped | |
| 4 (render) | Shipped | caption on all three targets |
| 5 (books, docs) | Shipped | `ch03-rank.md`'s block waits for the chapter to be listed |

Record here anything that differed from the design while building it: a rule
that had to change, a test that needed a different shape, a count that moved.
```

Then fill in that last paragraph with what actually happened, or delete it
if nothing did.

- [ ] **Step 6: The full gate**

Run:

```bash
make fmt-check
make ayce
make slow
cargo run -p bower -- --book books/hello-playbook verify
cargo run -p bower -- --book books/hello-playbook status -o target/hello-playbook
make book && make epub
make failures
cargo tree -p bower-core -e normal
```

Expected: every command green; `verify` ends `every claim holds` with two
`ok   output=` rows; `cargo tree` prints one line.

- [ ] **Step 7: Commit**

```bash
git add bower-spec.md README.md .okf BACKLOG.md docs/TECHNICAL_DEBT.md docs/EPIC-11_Diagnostics.md
git commit -m "docs: captured diagnostics in the spec, README, OKF, backlog, and debt; close EPIC-11"
```

---

## Exit criteria (from the EPIC), and the task that proves each

| # | Criterion | Proof |
|---|---|---|
| 1 | `hello-playbook` verifies with both outputs matched | Task 13 Step 5, `upholds_the_two_deliberate_failures` |
| 2 | One changed character fails verify, naming chapter, block, line | Task 13 Step 5, `reports_a_drifted_diagnostic_as_broken` |
| 3 | A second `--record` writes nothing; the lock is in sync | Task 13 Steps 4–5, `record__is_a_no_op_when_nothing_changed` |
| 4 | Different paths, terminals, scheduling record the same text | Tasks 5, 9, 10 (rules, textures, `scrub_for`, serial env); `record__rewrites_the_chapter_and_the_lock` |
| 5 | No SHA moves on `--record` | Task 13 Step 6, `replay__recording_an_output_moves_no_sha` |
| 6 | `make failures` fails when the pinned toolchain rewords a quote | Task 13 Steps 8–9: `rust4failures` pins 1.98.1 and records a trimmed `kata-compiles` quote |
| 7 | `bower-core` still has no dependencies | `make purity`, in every kernel task |
