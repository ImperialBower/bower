# EPIC-08: the site branch — shipping the rendered book (SITE)

## Context

Seven EPICs shipped, and on 2 September 2026 the whole chain ran for real:
`abstecker/hello-playbook` holds 21 commits and 26 tags, and every one of the
114 repo links in the published epub and PDF resolves. The book's central claim
— stable links in both directions — is proven against a live remote.

**Except the site half of it was fixed by hand, and nothing keeps it fixed.**

`bower publish --target html` renders the book. `bower push` sends
`refs/heads/main` and every tag (`bower/src/forge.rs:326`). Between those two
facts there is no command that ships the rendered book anywhere, so publishing
the site meant a manual `git init`, `git commit`, `git push origin gh-pages`,
plus a hand-written `.nojekyll`.

That is not merely inconvenient. It fails **silently and in the worst
direction**: regenerate the book, `bower push`, and `main` gains new code while
`gh-pages` keeps serving the old chapters. Every link still returns **HTTP 200**.
A stale site that answers 200 is worse than a 404, because nothing anywhere
signals that the reader is looking at last week's book. `bower status`
(`bower/src/status.rs:234`) reports the lock and the repo, and knows nothing
about a site.

**This EPIC does not** render the book — `MdBookRenderer`
(`bower/src/publish.rs:393`) already does, and EPIC-06 owns that. It does not
manage DNS, custom domains, or Pages settings beyond what a branch push
requires. It does not publish the epub or PDF as release assets; that is a
different verb and deserves its own EPIC. And it does not touch `bower-core`,
which stays pure and dependency-free — an eighth EPIC running.

---

## Status

| Component | Status |
|---|---|
| `site_branch` in `bower.toml` | **Complete** |
| `.bower-site` marker + `.nojekyll` | **Complete** |
| The site gate | **Complete** |
| `bower push` ships the site | Planned |
| `bower status` reports site drift | Planned |
| Goldens | Planned |

---

## Goals

- One command makes the remote match the book: **code, tags, and the rendered
  site**.
- A **stale site is visible**. `bower status` says so, rather than leaving every
  link returning 200 over last week's content.
- The site branch is **guarded like the code branch**. `bower push` refuses to
  overwrite a site Bower did not generate, with no override — the rule EPIC-05
  established and this EPIC must not weaken.
- The renderer emits what a static host needs, so nobody hand-writes
  `.nojekyll` again.

## Scope

### Configuration

```toml
[repos.hello-playbook]
github      = "abstecker/hello-playbook"
site_branch = "gh-pages"        # absent means: this book ships no site
```

Absent is the default and is **not** an error, the same rule `github` already
follows (`bower/src/config.rs:43`): a book that does not publish a site is a
normal book.

### What the site branch contains

The rendered HTML, plus two files the renderer writes:

| File | Why |
|---|---|
| `.nojekyll` | Without it GitHub Pages runs Jekyll, which silently drops files and directories beginning with `_`. mdBook emits none today; the failure mode is missing CSS with no error, which is exactly the kind nobody catches. |
| `.bower-site` | The marker. Carries `marker_line(book_name)` (`bower/src/trailers.rs:55`) plus the `bower.lock` digest of the plan it was rendered from. |

`.bower-site` does two jobs, and they are why it is a file rather than a
convention:

1. **The gate.** `book_named_in` (`bower/src/trailers.rs:65`) reads it, and the
   verdict rules are `marker_verdict`'s (`bower/src/push.rs:60`) unchanged. A
   branch with content and no marker is refused. **There is no override**, for
   the reason EPIC-05 gives at length.
2. **Staleness.** The digest lets `bower status` compare the site against the
   plan the book would produce now, without fetching every page.

### Rules

- `bower push` pushes the site branch when `site_branch` is configured and a
  rendered book exists; it says what it skipped when either is missing.
- The rendered book is **not** re-rendered by `push`. Push pushes. If the site
  on disk is older than the plan, `push` refuses and names
  `bower publish --target html` as the fix — the same shape as refusing a stale
  repo today (`bower/src/push.rs:135`).
- Dry run stays the default, and covers the site push too.
- `bower status` gains a `site` row: `never published`, `in sync`, or
  `STALE — N steps behind`.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| The remote's name | `RepoConfig::github` `bower/src/config.rs:43` | ✅ done |
| Which branch serves the site | `RepoConfig::site_branch` | ✅ done |
| The rendered book | `MdBookRenderer` `bower/src/publish.rs:393` | ✅ done |
| Is this branch ours? | `marker_verdict` `bower/src/push.rs:60` | ✅ reused, unchanged |
| What the site was built from | `.bower-site` digest | ✅ done |
| Pushing a branch | `Forge::push` `bower/src/forge.rs:86` | 🟡 takes one branch |
| Is the site current? | `SiteDrift` | ❌ absent |

---

## Design

### `Forge::push` already takes a branch

`fn push(&self, dir: &Path, repo: &str, branch: &str)`
(`bower/src/forge.rs:86`) is branch-parameterised already, so the site push is
the **same call with a different directory and branch**. What it lacks is a way
to push a directory that is not a git repository: the rendered book is a plain
folder.

```rust
/// Publish a directory of files as the entire content of `branch`.
///
/// The rendered book is not a git repository and has no history worth keeping —
/// each publish replaces it wholesale, exactly as a replayed repo replaces its
/// own history. Implemented as an orphan commit, so the site branch never
/// accumulates a history nobody reads.
fn push_tree(&self, dir: &Path, repo: &str, branch: &str) -> Result<PushOutcome, ForgeError>;
```

An orphan commit each time, rather than a growing history: the site is a build
artifact, and a thousand commits of regenerated HTML is noise that costs clone
time and tells nobody anything.

### `SiteDrift`

`bower/src/status.rs`, beside `RepoDrift` (`bower/src/status.rs:99`):

```rust
pub enum SiteDrift {
    /// No `site_branch` configured. A book that ships no site is normal.
    NotConfigured,
    /// Configured, but nothing rendered locally yet.
    NeverPublished,
    /// The rendered book on disk was built from this plan.
    InSync,
    /// It was not. The digest names which.
    Stale { rendered_from: String, current: String },
}
```

Deliberately about the **local** rendered book, not the remote. Reading the
remote's `.bower-site` needs a network call, and `status` is the one command in
this project that touches nothing outside the machine. `push` does the remote
check, because `push` is already talking to the forge.

### The digest

The `bower.lock` text, hashed. `lock_text` (`bower-core/src/plan.rs:216`) is
already the canonical serialization of a plan, and `status` already compares
against it — so the site's fingerprint is the same value the lock check uses,
and a site is stale exactly when the lock would be.

No new hashing dependency: a short, stable, non-cryptographic digest written by
hand is enough for "did this change", and this project has resisted adding a
crate for less.

---

## Work Items

### Phase 0 — Configuration and the marker

- [x] **0a.** `RepoConfig::site_branch`, optional, absent by default.
- [x] **0b.** `write_site_files()` writes both, and `MdBookRenderer` calls it —
  **but only when the render is destined for a site**. A `site_marker: None`
  renders HTML for local reading and leaves no stray dotfiles behind.
- [x] **0c.** `site_marker()` and `digest()`. **Deviation:** the digest is
  FNV-1a written out by hand rather than a hashing crate. "Did this change"
  needs no cryptography, it must be stable across runs and machines, and this
  project has declined a dependency for less.
- [x] **0d.** Seven tests, including `digest__is_stable_and_changes_with_content`
  and `site_marker__changes_when_the_plan_does`.

**Three stale test assertions found and fixed, and the process failure behind
them.** Repointing the sample book at `abstecker` on 2 September broke
`config__loads_the_sample_book`, a golden in `bower/tests/publish.rs`, and
`a_book_without_a_github_key_publishes_nothing_and_says_so` — and **none of it
was noticed, because the suite was never run after that edit**. Building,
pushing, and publishing all succeeded on a red test suite.

Two of the three were simple staleness. The third was a design fault worth
naming: that test asserted the *sample book* declares no `github` key, so it
broke the day the sample book started publishing. Its intent — "a book that does
not publish is normal" — now owns its own fixture, a copy of the sample with the
key stripped. A test that asserts what a live fixture does **not** contain is a
test waiting to break. `the_sample_book_now_declares_a_remote` is its
counterweight. Verified on branch `review2`, 2 September 2026: 248 tests,
`make ayce` green.

### Phase 1 — The gate

- [x] **1a.** `marker_verdict` reused **with no change at all** — only the file
  it reads differs. That is the claim EPIC-05 made about the gate's shape, and
  `site_gate__our_own_book_is_safe` is the proof.
- [x] **1b.** **Two** `Forge` methods, not one. The design listed
  `read_site_marker`; `probe_branch` had to join it, because *absent branch* and
  *branch with content and no marker* are different verdicts — the first is safe
  to create, the second must be refused. One method returning `Option` would
  have collapsed them, which is the same mistake `is_not_found` exists to
  prevent on the code branch. `FakeForge::with_site` configures both.
- [x] **1c.** Six tests. **Additions:**
  `site_gate__a_different_books_site_is_refused` (two books, one `gh-pages`),
  `site_probe__absent_and_unmarked_are_different_answers` (the distinction the
  gate rests on), and
  `site_probe__an_unreachable_remote_errors_rather_than_reporting_absent` —
  because *absent* is the **safe** verdict here, so a swallowed error would open
  the gate rather than close it. Verified on branch `review2`, 2 September 2026:
  254 tests, `make ayce` green.

  `site_push__is_not_called_when_the_gate_refuses` belongs to Phase 2, where
  there is a site push to not call.

### Phase 2 — Pushing the site

- [ ] **2a.** `Forge::push_tree`, implemented in `GitHubForge` as an orphan
  commit in a temporary clone, and recorded by `FakeForge`.
- [ ] **2b.** `plan_push` (`bower/src/push.rs:135`) gains a site half:
  `PushPlan::Ready` carries what the site push would do, and a stale or missing
  rendered book blocks with `bower publish --target html` named as the fix.
- [ ] **2c.** The dry run reports the site branch, its file count, and the
  gate's verdict.
- [ ] **2d.** Tests: `plan__site_not_configured_is_not_an_error`,
  `plan__a_stale_rendered_book_blocks`.

### Phase 3 — `bower status` sees the site

- [ ] **3a.** `SiteDrift` and its computation from the local rendered book.
- [ ] **3b.** A `site` row in `StatusReport` (`bower/src/status.rs:234`), and
  `has_drift` counting `Stale` but **not** `NeverPublished` or
  `NotConfigured` — absent is not drift, the rule EPIC-04 settled.
- [ ] **3c.** Tests: `site__not_configured_is_not_drift`,
  `site__a_stale_render_is_drift_and_names_the_fix`.

### Phase 4 — Goldens and documentation

- [ ] **4a.** `bower/tests/site.rs`: against `FakeForge`, a configured book
  plans a site push; an unconfigured one does not; a refused gate sends nothing.
- [ ] **4b.** `README.md`, `BACKLOG.md`, and `docs/TECHNICAL_DEBT.md` — the
  "site goes stale silently" gap this closes.
- [ ] **4c.** Flip Status rows; append the corrigendum.

---

## Test Plan

- `site_marker__round_trips_through_book_named_in` — the guard recognizes what
  the renderer wrote, the lesson EPIC-05 Phase 0 learned.
- `renderer__writes_nojekyll` — its absence fails by silently dropping files,
  which is the failure mode nobody notices.
- `site_gate__content_without_a_marker_is_refused` — a hand-built site must not
  be overwritten.
- `site_push__is_not_called_when_the_gate_refuses` — not that a refusal is
  printed, but that nothing was sent.
- `plan__site_not_configured_is_not_an_error` — a book that ships no site is
  normal.
- `plan__a_stale_rendered_book_blocks` — pushing yesterday's HTML is the bug
  this EPIC exists to prevent.
- `site__not_configured_is_not_drift` — CI stays green on a book with no site.
- `site__a_stale_render_is_drift_and_names_the_fix` — a report that cannot be
  acted on gets ignored.

## Key Files

| File | Role |
|---|---|
| `bower/src/config.rs:41` | `RepoConfig` gains `site_branch` |
| `bower/src/publish.rs:393` | `MdBookRenderer` writes `.nojekyll` and `.bower-site` |
| `bower/src/forge.rs:86` | `Forge` gains `push_tree` and `read_site_marker` |
| `bower/src/push.rs:135` | `plan_push` gains the site half |
| `bower/src/status.rs:99` | `SiteDrift` beside `RepoDrift` |
| `bower/tests/site.rs` | new; the goldens |

## Reuse (do NOT recreate)

- `bower/src/push.rs:60` — `marker_verdict` is the gate. The site branch is a
  different file to read, not a different rule to apply.
- `bower/src/trailers.rs:55,65` — `marker_line` and `book_named_in` are one
  definition of the marker. `.bower-site` uses them, not a second format.
- `bower-core/src/plan.rs:216` — `lock_text` is the canonical plan
  serialization; the site digest hashes it rather than inventing a fingerprint.
- `bower/src/publish.rs:393` — `MdBookRenderer` renders. This EPIC adds two
  files to its output and nothing else.
- `bower/src/forge.rs:96` — `FakeForge` counts calls, which is how
  `site_push__is_not_called_when_the_gate_refuses` can assert anything.

## Compatibility

- **Preserves** every existing command. A book with no `site_branch` behaves
  exactly as today.
- **Adds** one config key, two files in the rendered output, two `Forge`
  methods, and one `status` row.
- **Breaks** nothing.

## Dependencies

- **Blocks:** editions (spec § 13 M2) — an edition pins book, lock, and repo
  tags, and a published edition that cannot pin its own site is half an edition.
- **Built on:** EPIC-05 for the gate and `FakeForge`; EPIC-06 for the renderer;
  EPIC-04 for `status` and the "absent is not drift" rule.
- **Related:** spec § 5.5, § 13 M1.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook publish --target html -o books/hello-playbook/book
cargo run -p bower -- --book books/hello-playbook status  -o /tmp/hp
cargo run -p bower -- --book books/hello-playbook push    -o /tmp/hp            # dry run
cargo run -p bower -- --book books/hello-playbook push    -o /tmp/hp --execute
curl -sI https://abstecker.github.io/hello-playbook/ch01-a-repo-that-builds.html | head -1
```

Exit criteria:

1. A dry run reports the site branch, its file count, and the gate's verdict,
   and sends nothing.
2. `--execute` publishes the rendered book to `site_branch`, and the live page
   reflects the current plan.
3. Editing a chapter and re-running `status` reports the site **STALE** and
   names `bower publish --target html` as the fix.
4. A branch with content and no `.bower-site` is refused, and a test proves
   nothing was sent.
5. A book with no `site_branch` publishes code and tags exactly as today, and
   says the site was skipped.
6. `cargo tree -p bower-core -e normal` still prints one line.
