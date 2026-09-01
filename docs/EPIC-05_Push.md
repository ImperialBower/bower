# EPIC-05: `bower push` — publishing, with a guard rail (PUB)

## Context

Four EPICs shipped. `bower build` replays a book into a deterministic git
repository; `bower verify` checks every declared `expect` against a real
compiler; `bower status` reports drift; `mdbook-bower` renders the book with
links into the code. Everything the reader would follow exists — except the
repository those links point at, which lives only on somebody's disk.

`RepoConfig::github` (`bower/src/config.rs:43`) is parsed, printed by
`bower status`, and used by nothing. Spec § 5.5 describes what it is for:
force-push with lease to `owner/name`, tags included, creating the repository
through the GitHub API on first push.

**This is the first command in the project that can destroy something
irreplaceable.** Every other command is safe by construction — `build` writes to
a directory the caller named, `verify` works in a scratch directory, `status`
only reads. `push` rewrites remote history by design, because a generated repo
*is* a build artifact: edit chapter 3 and every commit after it takes a new SHA,
so the remote must be replaced rather than appended to. `--force-with-lease`
protects against a remote that moved; it does nothing whatever about a remote
that was never yours. One typo — `github = "ImperialBower/pkcore"` — and the
next push replaces years of human history with a book.

That risk sets this EPIC's central requirement, below.

**This EPIC does not** implement `bower publish` (spec § 13, the epub/PDF
targets), does not manage branch protection or repository settings beyond what
creation needs, does not push the *book's* own repository, and does not touch
`bower-core`.

---

## Status

| Component | Status |
|---|---|
| `Forge` trait + `FakeForge` | Planned |
| **The generated-repo marker gate** | Planned |
| Preconditions: built, and in sync | Planned |
| `GitHubForge` — `git` and `gh` | Planned |
| `bower push` CLI, dry-run by default | Planned |
| Goldens against the fake forge | Planned |

---

## Goals

- Publish a generated repository to its configured remote, **tags included**.
- Make it **impossible to force-push over a repository Bower did not generate**,
  without an override flag, without a confirmation prompt, without a way to talk
  the tool into it.
- Refuse to publish a build that is **out of date** — pushing a stale repo
  publishes a lie, and `bower status` already knows.
- Keep the network at arm's length behind a **trait**, so the logic is tested
  and only the last inch is not.
- Default to **saying what it would do**, not doing it.

## Scope

### The marker gate — the hard requirement

Before any push to an existing remote, `bower push` fetches that remote's
`STEPS.md` and reads its first paragraph. `trailers::steps_md`
(`bower/src/trailers.rs:50`) writes it as:

```text
This repository is generated from the book *hello-playbook*. Do not open
```

The rules, in order:

| Remote state | Verdict |
|---|---|
| Does not exist | **Safe.** Create it and push; there is nothing to destroy. |
| Exists and is empty | **Safe.** Nothing to destroy. |
| Has `STEPS.md` naming **this** book | **Safe.** This is our repository. |
| Has `STEPS.md` naming a **different** book | **Refuse.** Two books are fighting over one repo. |
| Has content but no `STEPS.md` | **Refuse.** Not a generated repository. |
| Cannot be read at all | **Refuse.** An unknown remote is not a safe one. |

**There is no override flag.** Not `--force`, not `--yes-really`, not an
environment variable. A guard with a documented bypass is a guard that gets
bypassed at 2am, and the thing on the other side of it is somebody's years of
work. The escape hatch, for the rare case of adopting an existing repository, is
to push a `STEPS.md` to it by hand first — which is deliberate, visible, and
hard to do by accident.

Note that this also catches the *subtler* mistake the typo story misses: two
books configured to publish to the same repository, each quietly overwriting the
other.

### The rest

- `bower push [--repo R] [-o DIR] [--execute]`.
- **Dry run is the default.** Without `--execute` the command reports exactly
  what it would do — the remote, the branch, the tag count, the gate's verdict —
  and changes nothing. Spec § 5.5 says push is always explicit; making the
  destructive half the opt-in is the strongest reading of that.
- Refuses when the repository has never been built, or when `bower status`
  reports drift. A stale build must not be published.
- Pushes the branch `refs/heads/main` (`bower/src/replay.rs:25`) and every tag
  from `replay::expected_tags` (`bower/src/replay.rs:288`), force-with-lease.
- No `github` key means no push, and saying so is not an error.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| The remote's name | `RepoConfig::github` `bower/src/config.rs:43` | 🟡 parsed, unused |
| The repo's own marker | `steps_md` `bower/src/trailers.rs:50` | ✅ done |
| What must be pushed | `expected_tags` `bower/src/replay.rs:288` | ✅ done |
| Is the build current? | `repo_drift` `bower/src/status.rs:132` | ✅ done |
| A place to publish to | `Forge` | ❌ absent |
| The gate | `MarkerVerdict` | ❌ absent |
| The command | `push()` | ❌ absent |

---

## Design

### `Forge` — the network, behind a door

`bower/src/forge.rs` (new):

```rust
/// A place a generated repository can be published to.
///
/// Everything that touches the network lives behind this trait, so the
/// decisions — the gate, the preconditions, what gets pushed — are tested
/// against a fake and only the last inch is not.
pub trait Forge {
    /// Whether `owner/name` exists, and whether it has any content.
    fn probe(&self, repo: &str) -> Result<RemoteState, ForgeError>;

    /// The remote's `STEPS.md` at its default branch, if it has one.
    fn read_steps_md(&self, repo: &str) -> Result<Option<String>, ForgeError>;

    /// Create `owner/name`. Only ever called for a remote that does not exist.
    fn create(&self, repo: &str, description: &str) -> Result<(), ForgeError>;

    /// Force-push-with-lease `dir`'s branch and tags to `owner/name`.
    fn push(&self, dir: &Path, repo: &str, branch: &str) -> Result<PushOutcome, ForgeError>;
}

pub enum RemoteState { Absent, Empty, HasContent }

pub struct PushOutcome { pub commits: usize, pub tags: usize }
```

### `MarkerVerdict` — the gate, as a pure function

`bower/src/push.rs` (new):

```rust
pub enum MarkerVerdict {
    /// Nothing to destroy.
    SafeNew,
    /// Ours, and for this book.
    SafeOurs,
    /// Refuse, with the reason a human needs.
    Refuse(String),
}

/// The gate. Pure: state and text in, verdict out, no I/O.
///
/// Written as a pure function on purpose. A safety check tangled with network
/// code is a safety check nobody can test every branch of, and every branch of
/// this one matters.
pub fn marker_verdict(
    state: RemoteState,
    steps_md: Option<&str>,
    book_name: &str,
) -> MarkerVerdict;
```

The book's name is matched against the sentence `steps_md` writes, using the
same `book_name` (`bower/src/replay.rs:260`) that produced it — not a second
guess. EPIC-04 learned this three times over: a checker that re-derives what it
checks reports its own disagreement first.

### `push` — the sequence

```rust
pub struct PushPlan {
    pub repo: String,
    pub remote: String,
    pub branch: String,
    pub tags: usize,
    pub verdict: MarkerVerdict,
}

/// Decide everything, touching nothing. `--execute` then acts on this.
pub fn plan_push(
    forge: &dyn Forge,
    cfg: &BookConfig,
    plan: &RepoPlan,
    dir: &Path,
    book_root: &Path,
) -> Result<PushPlan, PushError>;
```

Splitting *decide* from *act* is what makes dry-run free and the gate testable:
the same function runs in both modes, and `--execute` only chooses whether to
call `Forge::push` afterwards.

### `GitHubForge` — shelling out, deliberately

The real implementation shells out to `git` and `gh` rather than linking a
network stack.

- **`git push --force-with-lease --tags`** — git's credential handling is
  battle-tested and already configured on every machine that has a book to
  publish. Reimplementing it against `gix`'s network features would mean owning
  authentication, and owning authentication badly is how this command's risk
  gets worse rather than better.
- **`gh api`** for probe, `STEPS.md`, and creation — `gh` already holds a token
  and knows the API shapes.

The cost is two runtime dependencies on binaries rather than crates, and the
`Forge` trait is exactly the seam that makes a native implementation a later
swap rather than a rewrite. Both are checked for on startup, with the install
command named when absent.

---

## Work Items

### Phase 0 — The gate, pure and alone

- [ ] **0a.** `bower/src/forge.rs`: `Forge`, `RemoteState`, `PushOutcome`,
  `ForgeError`. No implementation yet.
- [ ] **0b.** `bower/src/push.rs`: `MarkerVerdict` and `marker_verdict()`.
- [ ] **0c.** Tests, one per row of the Scope table:
  `gate__absent_remote_is_safe`, `gate__empty_remote_is_safe`,
  `gate__our_own_book_is_safe`, `gate__a_different_book_is_refused`,
  `gate__content_without_steps_md_is_refused`,
  `gate__unreadable_remote_is_refused`.
- [ ] **0d.** `gate__refusal_names_the_repository_and_the_reason` — a refusal a
  reader cannot act on will be worked around.

### Phase 1 — Deciding, against a fake

- [ ] **1a.** `FakeForge` in `bower/src/forge.rs` under `#[cfg(test)]`,
  recording every call so a test can assert that `push` was **not** called.
- [ ] **1b.** `PushPlan`, `plan_push()`, `PushError`.
- [ ] **1c.** Preconditions: never built, or `repo_drift`
  (`bower/src/status.rs:132`) reporting `Stale`, both refuse before the forge is
  touched at all.
- [ ] **1d.** Tests: `plan__refuses_an_unbuilt_repo`,
  `plan__refuses_a_stale_repo`, `plan__counts_every_tag`,
  `plan__no_github_key_is_not_an_error`.

### Phase 2 — The command

- [ ] **2a.** `bower push [--repo R] [-o DIR] [--execute]` in
  `bower/src/main.rs`, dry run by default.
- [ ] **2b.** The dry-run report: remote, branch, tag count, verdict, and — when
  refusing — the reason and what to do about it.
- [ ] **2c.** Exit `0` on a clean dry run, `1` on any refusal.

### Phase 3 — The real forge

- [ ] **3a.** `GitHubForge`: `gh api` for probe, `STEPS.md`, and create;
  `git push --force-with-lease --tags` for the push itself.
- [ ] **3b.** Check for `git` and `gh` up front and name the install command
  when either is missing.
- [ ] **3c.** Creation sets the description and leaves `archived: false`
  (spec § 5.5).

### Phase 4 — Goldens and documentation

- [ ] **4a.** `bower/tests/push.rs` driving the binary in dry-run mode against
  the sample book: reports the remote and refuses nothing.
- [ ] **4b.** A golden that asserts a **refusal** — the fake forge returns
  content with no `STEPS.md`, and `Forge::push` is never called.
- [ ] **4c.** `README.md` and `BACKLOG.md`; document that there is no override
  and why, and that adopting an existing repo means pushing a `STEPS.md` by hand.
- [ ] **4d.** Flip Status rows, append the corrigendum.

---

## Test Plan

- `gate__*` — one per row of the Scope table. This is the only feature in the
  project where a missed branch is measured in someone else's lost work.
- `gate__refusal_names_the_repository_and_the_reason` — a refusal nobody can act
  on gets worked around.
- `plan__refuses_an_unbuilt_repo`, `plan__refuses_a_stale_repo` — a stale build
  must not be published, and `bower status` already knows.
- `plan__no_github_key_is_not_an_error` — a book that does not publish is
  normal, the same rule `status` follows for unbuilt repos.
- `push__is_not_called_when_the_gate_refuses` — the assertion that matters most:
  not that a refusal is *reported*, but that nothing was *sent*.
- `push__dry_run_calls_no_mutating_method` — the default mode is inert.
- `push__execute_pushes_branch_and_tags` — against the fake.

## Key Files

| File | Role |
|---|---|
| `bower/src/forge.rs` | new; `Forge`, `RemoteState`, `FakeForge` |
| `bower/src/push.rs` | new; the gate and `plan_push` |
| `bower/src/main.rs` | the `push` subcommand |
| `bower/src/config.rs:43` | `github`, finally used |
| `bower/tests/push.rs` | new; the goldens |

## Reuse (do NOT recreate)

- `bower/src/trailers.rs:50` — `steps_md` writes the marker. The gate reads what
  that function writes; do not invent a second marker format.
- `bower/src/replay.rs:260` — `book_name` produced the name in the marker. The
  gate must match with the same function.
- `bower/src/replay.rs:288` — `expected_tags` is what gets pushed.
- `bower/src/replay.rs:25` — `BRANCH` is the branch to push.
- `bower/src/status.rs:132` — `repo_drift` already answers "is this build
  current?". Do not re-implement the comparison.

## Compatibility

- **Preserves** every existing command and `bower-core`'s purity.
- **Adds** one subcommand and two modules. No new crate dependency; two runtime
  binary dependencies (`git`, `gh`) checked for at startup.
- **Breaks** nothing.

## Dependencies

- **Blocks:** spec § 11 Phase 5 (migration) only loosely — the real
  *Rust for Failers* repos have to be published eventually, but the book can be
  written and verified long before that.
- **Built on:** EPIC-01 (tags, branch, `STEPS.md`), EPIC-04 (`repo_drift`).
- **Related:** spec § 5.5, § 7, § 12 Q4 (should generated repos carry CI?).

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hp
cargo run -p bower -- --book books/hello-playbook push -o /tmp/hp    # dry run
```

Exit criteria:

1. A dry run reports the remote, the branch, the tag count, and the gate's
   verdict, and touches nothing.
2. A remote with content and no `STEPS.md` is refused, and the test proves
   `Forge::push` was never called — not merely that a refusal was printed.
3. A remote whose `STEPS.md` names a different book is refused.
4. An unbuilt or drifted local repository is refused before the forge is
   contacted at all.
5. There is no flag, environment variable, or configuration key anywhere in the
   codebase that bypasses the gate. `grep -ri "force\|override\|skip" bower/src`
   turns up nothing that does.
6. `cargo tree -p bower-core -e normal` still prints one line.
