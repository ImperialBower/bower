# EPIC-16: Editions — the living book and the frozen one (ED)

## Summary

- **Builds:** editions: `bower edition cut` pins a book as a manifest with
  immutable `edition-<id>/…` tags, and `bower diff --edition` computes errata.
- **Why:** `[book] version` is only a label: nothing is pinned, epub links
  follow force-pushed tags, and a re-ship clobbers the release files.
- **Shape:** a pure `edition_state` with five states that `status`, `build`,
  `publish`, `push`, and `diff` all ask, and a `TagScope` making HTML living
  and epub and PDF frozen.
- **Proves it:** `edition cut` on the sample book writes 20 `upheld` rows, and
  `FakeForge` records no delete, no `--prune`, no forced edition ref.
- **Status:** Planned, filed 10 September 2026, nothing started.

---

## Context

A book has one version today, and it is a label. `[book] version` is a free
string on `BookConfig` (`bower/src/config.rs:33-39`), deliberately not
semver. Three things read it. `artifact_name` (`bower/src/publish.rs:598`)
puts it in the epub and PDF filenames. `run_publish` hands it to
`render_plan` (`bower/src/main.rs:580-590`). `plan_release`
(`bower/src/push.rs:274`) turns it into a tag with `release_tag`
(`push.rs:321`), so `0.1.0` becomes `v0.1.0`, and hangs a GitHub release off
that tag with every `.pdf` and `.epub` in `assets` attached. Both sample books
declare one: `hello-playbook` at `0.1.0`, `rust4failures` at `0.1.1`. The
release path has never run against real GitHub (`BACKLOG.md:187-190`).

Spec § 13 M2 asks for something stronger: a published edition is a pinned
triple (book source commit, `bower.lock`, and the generated repo tags), and
edition tags are never deleted or moved, so a reader of the 1.0 epub follows
1.0 links forever while `main` moves on. The shipped release code does the
reverse in four places:

- **Nothing is pinned.** Replay creates `step-NNN-id` tags and `<chapter>-end`
  tags and nothing else (`bower/src/replay.rs:141-150`). No `v0.1.0` tag
  exists locally. `GitHubForge::publish_release` calls `gh release create`
  with no `--target` (`bower/src/forge.rs:718-728`), so GitHub creates the tag
  at whatever the default branch holds when the release is made. The tag is
  lightweight and unstamped, and Bower's determinism does not cover it.
- **Every link moves.** The epub footer links `step.tag()`
  (`bower/src/render.rs:294`), and so do the checkout line (`render.rs:343`)
  and the exercise box (`render.rs:371`). Those tags are force-pushed on
  every regeneration (`forge.rs:672-675`). A reader of the 0.1.0 epub who
  follows a link after an erratum lands sees the corrected code under the old
  prose.
- **Frozen files get replaced.** A re-ship runs `gh release upload --clobber`
  (`forge.rs:706-716`), and the README calls that safe (`README.md:154-156`).
  It is safe only while the book is unchanged. Edit a chapter, keep `version`,
  push, and the 0.1.0 PDF is silently replaced by a different book.
- **Tags are pushed by glob.** `git push --force <url> --tags`
  (`forge.rs:675`) force-pushes every local tag. That is harmless while the
  only tags are step tags. Once an edition tag exists locally, the same line
  moves it.

The triple is also a quadruple. Every generated commit carries
`Generated-By: bower v<version>` (`bower/src/trailers.rs:15`, `:37`), so the
same book, lock and epoch replayed by a different `bower` produce different
SHAs. An edition that cannot be rebuilt byte for byte is not pinned. The
record has to name the tool that made it.

The rest is already in place: deterministic replay with epoch-stamped
tags (`replay.rs:193-217`), `lock_text` as the manifest
(`bower-core/src/plan.rs:369`), `site_fingerprint` for the prose the lock
omits (`publish.rs:854`), a pure FNV-1a `digest` (`publish.rs:831`), and the
`Forge` seam with `FakeForge` (`forge.rs:72`, `:169`). Ideas B1, B3, B4 and
C4 (`docs/bower-ideas.md:396-512`, `:684-705`) rank 6–7 on that document's
shortlist; `BACKLOG.md:37` assigns all four here.

**This EPIC does not** mint DOIs or write `CITATION.cff` (B1), build the
per-step toolchain board (B4), publish an Atom feed (B3), freeze the HTML
site, or pin devenv or a forge image. § Scope says why for each. It never
writes to the book's source repository: Bower records the source commit and
prints the command for the author.

---

## Status

| Component | Status |
|---|---|
| Kernel: `EditionId`, `digest` moved in, scoped tag names | **Planned** |
| Kernel: `EditionManifest` text and parse, per-step digests | **Planned** |
| Kernel: `EditionState` and `errata` | **Planned** |
| Testkit: edition textures, round-trip and errata properties | **Planned** |
| `bower edition cut`: source pin, toolchain capture, the receipt | **Planned** |
| `status`: the edition row | **Planned** |
| Replay: edition tags in the `Cut` state; `expected_tags` learns them | **Planned** |
| `push`: named tag refspecs, unforced edition tags, `Forge::remote_tags` | **Planned** |
| `push`: frozen release: create once, complete, never clobber | **Planned** |
| Render: `TagScope` for frozen targets, the colophon | **Planned** |
| `bower diff --edition`, living banner, "report a problem" link | **Planned** |
| Sample book cut at `0.1.0`; docs | **Planned** |

---

## Goals

- Make an **edition** a value: a name, a **pin** (source commit, lock digest,
  render fingerprint, `bower` version), and a **receipt** of every step's
  verdict on a named toolchain, all written to one file in the book.
- Make **edition tags immutable** in fact and not only in policy. Bower
  creates them deterministically, pushes them without force, and refuses
  when one on the forge disagrees.
- Keep the **living book** living. `main`, the step tags and the HTML site
  move on every push, exactly as today.
- Make **frozen artifacts** honest. The epub and PDF of an edition link that
  edition's tags, carry a colophon, and are never replaced on the forge.
- Produce **errata mechanically**: `bower diff --edition 0.1.0` compares the
  record with the book as it stands, step by step, with no memory involved.
- Keep the kernel **pure**. Every decision is a function of text and plans,
  and `cargo tree -p bower-core -e normal` still prints one line.

## Scope

### Things

| Thing | What it is |
|---|---|
| **Edition id** | `[book] version`, checked as a git ref component. `0.1.0`, `2e`. |
| **Pin** | Source commit, `digest(lock_text)`, `site_fingerprint`, `bower` version. |
| **Receipt** | Per repo: head SHA, `check`/`verify` commands, toolchain lines, one row per step (expect, verdict, `touched` and `tree` digests). |
| **Manifest** | Pin plus receipts plus chapter digests, in `editions/<id>.edition`. The pin and the receipt are one file. |
| **Edition tags** | `v0.1.0` (the head, the release's tag) and `edition-0.1.0/step-012-rank-enum`, `edition-0.1.0/ch01-end`. |
| **Edition state** | `Unversioned`, `Drafting`, `Cut`, `Moved`, `Unreproducible`: one pure function every command asks. |
| **Erratum** | One difference between a manifest and today's plan, located by step id and chapter. |
| **Frozen target** | epub and PDF. HTML is always living. |

### Business requirements

1. An edition tag, once on a forge, is never deleted, moved, or re-pointed
   by Bower. A disagreement is a refusal, not a repair.
2. A frozen artifact links only tags that satisfy rule 1.
3. A frozen release's files never change after the first upload. Missing
   files may be added.
4. An edition can only be cut from a book in which every claim holds, whose
   lock is current, and whose source is committed.
5. Anyone who checks out the source commit and runs the recorded `bower`
   gets the same head SHA the manifest names.
6. Errata are computed, not written: every difference between the manifest
   and the plan is reported, and nothing else.
7. A book with no `version` is a normal book. A book with a `version` and no
   manifest behaves as it does today, minus the clobber.

The business logic is § Design and § Work Items.

### What the reader gets

- In the 0.1.0 epub, `git checkout edition-0.1.0/step-012-rank-enum`, which
  still resolves after fifty errata.
- A colophon naming the edition, the source commit, and the `rustc` every
  step was verified with; the receipt attached to the release beside the
  epub and PDF.
- In the living HTML, once main has moved: *"This is the living book.
  Edition 0.1.0 was cut on 2026-09-12; 3 errata since."* Under every block,
  a "report a problem" link that files an issue carrying step, edition, and
  tree link.

### Not in scope

- **B4, the toolchain board.** N receipts on N toolchains, joined by step;
  the receipt row is keyed so the join is free. It needs parallel
  verification (`docs/TECHNICAL_DEBT.md:63-68`: 20 steps, 16s, sequential)
  times the matrix, plus a CI home. Open question 7.
- **B1, resolver, `CITATION.cff`, DOIs.** A mint is irreversible and manual;
  Zenodo keys on a release of the *source* repository, which here is one
  workspace of several books (spec § 12 Q2); a `CITATION.cff` changes every
  generated repo's final tree. Open question 6.
- **The rest of B3** (Atom feed, contributor credits, QR codes), **a frozen
  site** (open question 2), **devenv and forge-image pins** (open question
  8), and **B2's translated editions**.

---

## Decisions

1. **The pin is a quadruple:** source commit, `digest(lock_text)`,
   `site_fingerprint`, and the `bower` version, forced by `Generated-By`
   (`trailers.rs:34`). Epoch and identity live in `bower.toml` at the source
   commit, so they are covered.
2. **`[book] version` is the edition id, and this EPIC owns it.** Still a
   free string (`config.rs:36-38` says why), with one rule: it names git
   tags, so it must be a ref component — no `/`, `..`, whitespace,
   `~^:?*[\`, leading `-`, or `.lock` suffix. `EditionId::new` is pure;
   `ConfigError::Edition` sits beside `Epoch` (`config.rs:124`); `0.1.0`,
   `0.1.1` and `2e` pass. `release_tag` (`push.rs:321`) becomes
   `EditionId::head_tag`.
3. **Five states, one function.** `edition_state(version, manifest, now)`
   is `Unversioned` (no `version`), `Drafting(id)` (no manifest), `Cut(id)`
   (manifest matches plan, fingerprint and running `bower`), `Moved { id,
   errata }` (content differs), or `Unreproducible { id, recorded, running }`
   (content matches, `bower` does not, so SHAs would not). `status`, `build`,
   `publish`, `push` and `diff` ask it; none re-derives it.
4. **The manifest is the pin and the receipt, in one file:**
   `editions/<id>.edition`, written by `edition_text` and read by
   `parse_edition`, both in the kernel, with a round-trip property.
   `lock_drift` refuses to parse the lock (`status.rs:52-60`) because the
   plan can always be regenerated; the past cannot, so this file is parsed.
   C4's receipt and M2's pin record the same facts; two files would drift.
5. **The source commit is recorded, never made.** `cut` reads `HEAD` with
   `gix::discover` and refuses if `git status --porcelain -- <book>` is not
   empty. It neither tags the workspace nor commits the manifest; it prints
   the command. Bower's only git writes are to generated repositories. The
   workspace holds several books, so a source tag would be `<book>-<id>`,
   the author's call (open question 3).
6. **The release tag is the head; step tags get a namespace.** Head:
   `EditionId::head_tag` (`v0.1.0`), as shipped. Steps and chapter ends:
   `edition-<id>/<tag>` — a ref cannot be file and directory at once, so
   `v0.1.0/step-012-…` is impossible beside `v0.1.0`. Replay creates them
   only in `Cut`, on the step tags' commits, with the same stamps
   (`replay.rs:193`). **Bower creates the release tag; `gh` never does:**
   `gh release create --verify-tag` refuses rather than invents one.
7. **Edition tags go out unforced; a mismatch refuses the push.** `--tags`
   (`forge.rs:675`) goes. Step tags from `expected_tags` are pushed by name,
   forced as today; edition tags by name, unforced, so the forge itself
   refuses a move. First, `Forge::remote_tags` (`git ls-remote --tags`, as
   `remote_head` at `forge.rs:928`) feeds the pure `edition_tag_verdict`:
   absent is `Push`, the same tag-object SHA is `Unchanged`, anything else
   is `Refuse`, naming the tags. A refusal blocks the whole push, as a
   half-configured release does (`push.rs:241-247`). Bower never deletes a
   remote tag: no `--prune`, no `--mirror`, no `:refs/tags/…`.
8. **A frozen release is created once, then only completed.** `--clobber`
   (`forge.rs:709-716`) goes. `Forge::release_assets` lists what is there;
   the pure `release_action` answers `Create`, `AddMissing(names)` or
   `Unchanged`. In `Cut` a re-upload would add identical bytes, so nothing
   is lost. In `Moved` and `Unreproducible`, `plan_release` publishes
   nothing and says why. `README.md:154-156` is retired.
9. **HTML is living; epub and PDF are frozen.** `TagScope { Living,
   Edition(EditionId) }` threads through `render::chapter` (`render.rs:23`)
   and all three `step.tag()` calls. Frozen targets in `Cut` render
   `Edition`; everything else — the preprocessor, `--target html`, any
   target in another state — renders `Living`. A frozen render gets a
   colophon from the manifest.
10. **The receipt records what ran, not what was meant.** Rows carry expect
    and verdict (`cut` refuses any `Broken`, so `Upheld` or `Skipped`).
    Repos carry the exact `check` and `verify` strings and `rustc --version`
    / `cargo --version` run inside the verify tree, which honours a
    `rust-toolchain.toml` like `hello-playbook`'s step 006. Recording the
    commands keeps a known gap visible: `verify` does not run the book's own
    gate (`TECHNICAL_DEBT.md:27-36`). With EPIC-11, `compile_fail` rows add
    a diagnostic digest.
11. **Errata are a diff of digests, per step, matched by id.** `touched`
    digests the files a step touches as they read after it; `tree` the whole
    tree. A changed `touched` is `CodeChanged`; a changed `tree` alone is
    `Inherited`, folded to one count per chapter — otherwise one fix at step
    3 lists every step after it. Plus `StepAdded`, `StepRemoved`,
    `StepMoved`, `ExpectChanged`, and `ProseChanged` from chapter digests.
    Ids, because tags already rely on their stability (spec § 5.3).
12. **`digest` moves into the kernel.** The FNV-1a at `publish.rs:831` is
    pure and dependency-free; `publish` re-exports it — one hash, not two. It
    detects change, not tampering; an edition's cryptographic anchor is the
    git object ids its tags name.
13. **A cut is refused, not repaired, cheapest check first:** `version`
    declared and valid; no manifest for that id; lock in sync
    (`status.rs:64`); source clean; every step verifies; replay into `-o`
    succeeds and its head SHA is recorded. Each failure is a named refusal.
    `cut` reads the clock once, for `verified`: a cut is an event, not a
    replay, and "replay never reads a clock" (`config.rs:28-29`) stands.
14. **`Moved` is not drift.** Living main moves on by design. `status`
    prints the edition row; `has_drift` (`status.rs:319`) is unchanged,
    keeping EPIC-04's "absent is not drift". `Unreproducible` is printed
    loudly and is not drift either: nothing local is stale.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| The edition's name | `EditionId`; today a free `Option<String>` at `config.rs:39` | 🟡 validated |
| The release tag | `EditionId::head_tag`; today `release_tag` at `push.rs:321` | 🟡 moves in |
| Pin and receipt | `EditionManifest`, `editions/<id>.edition` | 🔴 new |
| Change detection | `bower_core::digest`; today `publish.rs:831` | 🟡 moves in |
| Which world we are in | `EditionState`, `edition_state` | 🔴 new |
| Frozen step links | `TagScope`; today `step.tag()` at `render.rs:294`, `:343`, `:371` | 🟡 scoped |
| Tags a build must hold | `expected_tags(plan, edition)`; today `replay.rs:351` | 🟡 plus edition tags |
| Tags on the forge | `Forge::remote_tags` | 🔴 new |
| Pushing tags | `Forge::push` `forge.rs:134`; today `--force --tags` at `:675` | 🟡 by name, two classes |
| A frozen release | `release_action`, `Forge::release_assets`; today `--clobber` at `:709-716` | 🟡 no clobber |
| Errata | `Erratum`, `errata` | 🔴 new |
| Toolchain evidence | `verify::toolchain` | 🔴 new |
| The colophon | `publish::colophon` | 🔴 new |

---

## Design

### Kernel — `bower-core/src/edition.rs` (new)

```rust
pub struct EditionId(String);
impl EditionId {
    pub fn new(raw: &str) -> Result<Self, EditionError>;   // git ref-component rules
    pub fn head_tag(&self) -> String;                       // release_tag's rule: v0.1.0, 2e
    pub fn scoped(&self, tag: &str) -> String;              // edition-0.1.0/step-012-rank-enum
}

pub struct Pin { pub source: String, pub bower: String, pub lock: String, pub render: String }

pub struct ReceiptRow { pub seq: usize, pub id: StepId, pub expect: Expect,
    pub verdict: Recorded /* Upheld | Skipped */, pub touched: String, pub tree: String }
pub struct RepoReceipt { pub repo: RepoName, pub head: String /* cut's replay */,
    pub check: String, pub verify: String, pub toolchain: Vec<String>, pub rows: Vec<ReceiptRow> }
pub struct EditionManifest {
    pub id: EditionId, pub pin: Pin, pub verified: String,  // YYYY-MM-DD, the cut's one clock read
    pub repos: Vec<RepoReceipt>, pub chapters: Vec<(String, String)>,
}

pub fn step_digests(step: &PlannedStep) -> (String, String);   // (touched, tree)
pub fn edition_text(m: &EditionManifest) -> String;
pub fn parse_edition(text: &str) -> Result<EditionManifest, EditionError>;

pub struct Now<'a> {
    pub plan: &'a BookPlan, pub lock: &'a str, pub render: &'a str,
    pub chapters: &'a [(String, String)], pub bower: &'a str,
}
pub enum EditionState {
    Unversioned, Drafting(EditionId), Cut(EditionId),
    Moved { id: EditionId, errata: Errata },
    Unreproducible { id: EditionId, recorded: String, running: String },
}
pub fn edition_state(v: Option<&EditionId>, m: Option<&EditionManifest>, now: &Now) -> EditionState;

pub enum Erratum {
    StepAdded { repo: RepoName, id: StepId, seq: usize },
    StepRemoved { repo: RepoName, id: StepId, was: usize },
    StepMoved { repo: RepoName, id: StepId, from: usize, to: usize },
    ExpectChanged { repo: RepoName, id: StepId, from: Expect, to: Expect },
    CodeChanged { repo: RepoName, id: StepId },
    Inherited { repo: RepoName, chapter: String, steps: usize },
    ProseChanged { chapter: String },
}
pub struct Errata(pub Vec<Erratum>);
pub fn errata(m: &EditionManifest, now: &Now) -> Errata;
```

`EditionError` is its own enum. It is not `BowerError` (`lib.rs:63`), because
every `BowerError` points at a chapter line, and a bad version string or a
malformed manifest has no chapter line. The prelude (`lib.rs:39`) exports the
types.

The manifest text follows the lock's shape. It is line-oriented, one line
per step, and made to be reviewed in a diff:

```
# bower edition — written by `bower edition cut`; never edit
edition  = 0.1.0
source   = 4ad21bf3c9e0d7a1b2f6e8c4a5d9b0e7f1c3a2d8
bower    = 0.1.0
lock     = 9f3c1e07a2b4d6e8
render   = 51aa0c93e4f2b7d1
verified = 2026-09-12

[hello-playbook]
head      = 7c2e…  v0.1.0
check     = cargo check
verify    = cargo test
toolchain = rustc 1.98.1 (…) | cargo 1.98.1 (…)
001 cargo-init expect=pass verdict=upheld touched=1a2b… tree=3c4d…
013 wont-compile expect=compile_fail verdict=upheld touched=… tree=…

[chapters]
src/ch01-a-repo-that-builds.md 77e1…
```

### Testkit — `bower-testkit`

Fixtures: `hello_playbook` (`fixtures.rs:88`), a manifest text, and one
edited copy per erratum kind. Properties: `parse_edition(edition_text(m)) ==
m`; `errata(m, now_of(m))` is empty; one edited block yields exactly one
`CodeChanged` and at most one `Inherited` per later chapter; `edition_state`
is `Cut` exactly when `errata` is empty and the versions match. The coverage
report (`coverage.rs:77`) gains an `erratum` axis.

### Cut, receipt and status — `bower`

`bower/src/edition.rs` (new, I/O): `source_pin` (`gix::discover`, `HEAD`,
the porcelain check), `load_manifests` (every `editions/*.edition`, errors
naming the file), `write_manifest`. `verify::toolchain(tree_dir)` runs the
two probes in the tree `Verifier::run` already writes (`verify.rs:183`); a
missing probe records `absent`. `main.rs` gains `Command::Edition { Cut { out } }` and
`Command::Diff { edition }`. The spec names the second one `bower diff
--edition` (§ 13 M2), and the name is kept. `StatusReport` (`status.rs:302`)
gains `edition: EditionState`, and its `Display` prints one row.

### Replay and push — `bower`

- **Replay.** `Replayer::run` (`replay.rs:84`) takes `edition:
  Option<&EditionId>`. When it is set, it writes `id.scoped(step.tag())` for
  every step tag, `id.scoped("<stem>-end")` for every chapter end, and
  `id.head_tag()` on the last commit, all through the existing `tag`
  (`replay.rs:193`). `build` passes the id only in `Cut`. `edition cut` passes
  it explicitly, since the state becomes `Cut` only once the file it is about
  to write exists.
- **Expected tags.** `expected_tags(plan, edition)` (`replay.rs:351`) adds
  the same names. Without that, `repo_drift` would report them as
  `unexpected_tags` (`status.rs:158-162`).
- **Push.** `Forge::push` takes `TagPush { forced: Vec<String>, frozen:
  Vec<String> }` in place of `--tags`. `plan_push` (`push.rs:162`) calls
  `forge.remote_tags(remote, "edition-<id>/*")` plus the head, and records
  the `edition_tag_verdict` on `PushPlan::Ready`. The dry run prints
  `edition   0.1.0 — 27 tags to push` or `edition   0.1.0 — on the forge,
  unchanged`.
- **Release.** `plan_release` (`push.rs:274`) takes the `EditionState` and
  the manifest path. It attaches the manifest as the receipt. That is a named
  path, not a third extension in `is_release_asset`'s closed list
  (`push.rs:345`).
- **Forges.** `FakeForge` records `remote_tags`, `release_assets` and every
  pushed refspec. `GitHubForge` covers the last inch with `git ls-remote
  --tags`, `gh release view --json assets`, and `gh release create
  --verify-tag`.

### Render — `bower`

`render_plan` (`publish.rs:500`) takes a `TagScope` beside `version`;
`RenderPlan` (`publish.rs:485`) carries it to `footer`, `checkout_line` and
`exercise_box`. A frozen plan gets one more chapter, `colophon(manifest,
links)`: pure markdown naming the edition, source commit, `bower` version,
toolchain, `verified` date, and the receipt and errata URLs.

In `Moved`, the living HTML gets a one-line banner above each chapter's
first heading. With the new `[book] issues` key — an issue-creation URL on
the repository holding the book source — the footer gains `[report]`, a
URL-encoded title and body carrying step id, tag in scope, tree link, and in
a frozen render the edition and toolchain. It goes to the book, never to a
generated repo, whose description says "Do not open pull requests"
(`main.rs:753`).

The errata page is back matter and lands through EPIC-12's generated-chapter
mechanism. Until then, `bower diff --edition <id> [-o FILE]` writes it as
markdown, one line per erratum, each linking edition tag to living tag.

---

## Work Items

### Phase 0 — Kernel

- [ ] **0a.** Move `digest` from `publish.rs:831` to `bower-core`, and
  re-export it from `publish`. `digest__is_stable_and_changes_with_content`
  (`publish.rs:1827`) moves with it.
- [ ] **0b.** `EditionId::new` with ref-component rules, `head_tag` (port
  `release_tag__prefixes_a_number_and_leaves_anything_else_alone`,
  `push.rs:452`), and `scoped`. Add `ConfigError::Edition`.
- [ ] **0c.** `step_digests`, `EditionManifest`, `edition_text`,
  `parse_edition`, `EditionError`.
- [ ] **0d.** `edition_state` and `errata`, with the `Inherited` fold.
- [ ] **0e.** Testkit: the fixtures, the four properties, and the `erratum`
  coverage axis. Confirm `make purity`.

### Phase 1 — Cut, receipt, status

- [ ] **1a.** `edition::source_pin`, `load_manifests`, `write_manifest`, and
  `verify::toolchain`.
- [ ] **1b.** `bower edition cut -o DIR` with the six ordered refusals. It
  prints the manifest path and the `git add`/`git commit` command, and runs
  neither.
- [ ] **1c.** The `status` edition row for all five states. `has_drift`
  does not change, and a test pins that.

### Phase 2 — Edition tags

- [ ] **2a.** `Replayer::run` gains `edition`, and `expected_tags` learns
  it. The determinism golden (`bower/tests/determinism.rs:94`) runs twice with
  an edition and gets identical tag-object SHAs.
- [ ] **2b.** `TagPush` on `Forge::push`: step tags forced by name, edition
  tags unforced. Delete the `--tags` line.
- [ ] **2c.** `Forge::remote_tags`, `edition_tag_verdict`, the
  `PushPlan::Ready` field, and the dry-run line.
- [ ] **2d.** Extend the no-override tests: no `--prune`, no `--mirror`, no
  delete refspec, and no `--force` on an `edition-` ref, for any state.

### Phase 3 — The frozen release

- [ ] **3a.** `Forge::release_assets` and `release_action`. Remove
  `--clobber`. `gh release create --verify-tag`.
- [ ] **3b.** `plan_release` reads `EditionState`: `Moved` and
  `Unreproducible` skip the release with a reason, and the manifest rides
  along as the receipt.
- [ ] **3c.** Retire `README.md:154-156`. Close the relevant half of
  `TECHNICAL_DEBT.md:176-183`, since a frozen release can no longer be fed a
  stale artifact.

### Phase 4 — Frozen render and errata

- [ ] **4a.** `TagScope` through `render_plan`, `chapter`, `footer`,
  `checkout_line` and `exercise_box`. The preprocessor is always `Living`.
- [ ] **4b.** `colophon`. The epub and PDF goldens in a `Cut` fixture link
  only `edition-` tags.
- [ ] **4c.** `bower diff --edition <id> [-o FILE]`, and the errata markdown
  with edition-tag-to-living-tag links.
- [ ] **4d.** The living banner in `Moved`. Add `[book] issues` and the
  `[report]` footer link, with URL encoding as a pure function.

### Phase 5 — Book and docs

- [ ] **5a.** Cut `hello-playbook` at `0.1.0` and run
  `make ship-hello-execute`. This is the release path's first real run, so
  record what `gh` said. Commit `editions/0.1.0.edition`.
- [ ] **5b.** Add `.okf/model/edition.md`, and update
  `.okf/config/bower-toml.md` (`version`, `issues`),
  `.okf/commands/push.md`, `.okf/commands/status.md`, `BACKLOG.md` (move
  Editions and file the deferred items) and `README.md` § Releases.
- [ ] **5c.** Flip the Status rows and append the corrigendum.

---

## Test Plan

Kernel: `edition_id__refuses_a_slash_a_space_and_dot_lock`,
`edition_id__head_tag_follows_the_release_rule`,
`edition_id__scoped_tags_never_collide_with_the_head`,
`manifest__round_trips_through_its_text`,
`manifest__parse_names_the_line_it_cannot_read`,
`state__no_manifest_is_drafting`, `state__matching_plan_is_cut`,
`state__a_new_bower_is_unreproducible_not_moved`,
`errata__unchanged_book_has_none`,
`errata__one_edited_block_is_one_code_change_and_the_rest_inherit`,
`errata__reordering_is_moved_not_removed_and_added`,
`errata__expect_change_is_named`, `errata__prose_edit_names_the_chapter`.

Cut: `cut__refuses_a_stale_lock`, `cut__refuses_a_dirty_book`,
`cut__refuses_a_broken_claim`, `cut__refuses_an_existing_manifest`,
`cut__records_the_head_the_replay_produced`,
`cut__never_writes_to_the_source_repo`.

Replay and status: `replay__edition_tags_point_at_the_step_tags_commits`,
`replay__edition_tags_are_byte_identical_across_runs`,
`status__edition_tags_are_expected_in_cut_and_only_then`,
`status__moved_is_reported_and_is_not_drift`.

Push: `push__step_tags_forced_edition_tags_not`,
`push__a_moved_remote_edition_tag_refuses_and_sends_nothing`,
`push__never_deletes_a_remote_tag`, `push__moved_state_publishes_no_release`,
`push__release_is_created_once_and_only_completed`,
`push__no_clobber_anywhere`.

Render: `render__frozen_targets_link_edition_tags_in_cut`,
`render__html_is_always_living`,
`render__colophon_names_source_bower_and_toolchain`,
`render__report_link_carries_step_and_edition`,
`render__banner_only_when_moved`.

## Key Files

| File | Role |
|---|---|
| `bower-core/src/edition.rs` | new: `EditionId`, manifest, state, errata |
| `bower-core/src/lib.rs` | `digest`, prelude exports |
| `bower-testkit/src/{fixtures,coverage}.rs` | edition textures, the `erratum` axis |
| `bower/src/edition.rs` | new: source pin, manifest I/O |
| `bower/src/config.rs` | `version` validated, `issues`, `ConfigError::Edition` |
| `bower/src/verify.rs` | `toolchain` |
| `bower/src/replay.rs` | edition tags, `expected_tags(plan, edition)` |
| `bower/src/status.rs` | the edition row |
| `bower/src/forge.rs` | `TagPush`, `remote_tags`, `release_assets`, no `--tags`, no `--clobber` |
| `bower/src/push.rs` | edition verdict, `release_action`, state-aware `plan_release` |
| `bower/src/render.rs`, `publish.rs` | `TagScope`, colophon, banner, report link |
| `bower/src/main.rs` | `edition cut`, `diff --edition` |

## Reuse (do NOT recreate)

- `publish.rs:831` `digest`: moved, not copied.
- `publish.rs:854` `site_fingerprint` is the pin's `render` field.
- `plan.rs:369` `lock_text` is the only lock format. The pin digests it and
  never re-derives it.
- `push.rs:321` `release_tag` becomes `EditionId::head_tag`, one rule.
- `replay.rs:193` `Replayer::tag` gives edition tags the same stamps.
- `replay.rs:351` `expected_tags`: edition tags are added here and nowhere
  else.
- `forge.rs:928` `remote_head` is the `ls-remote` shape `remote_tags`
  follows.
- `push.rs:60` `marker_verdict` is the model for `edition_tag_verdict`:
  state in, verdict out, and "could not look" is an error, never a verdict.
- `status.rs:64` `lock_drift` keeps its offline, text-only discipline.

## Compatibility

- **Preserves** every book without `version` byte for byte. It also
  preserves the `v0.1.0` release tag name and artifact names
  (`publish.rs:598`).
- **Adds** two subcommands, one config key (`issues`), a validation rule on
  `version`, one file per edition, two `Forge` methods, and one generated
  chapter.
- **Changes** `push`. Tags are now pushed by name, a release is never
  clobbered, and a book whose plan has moved past its cut edition publishes
  no release until `version` is bumped. The release tag is created by Bower,
  not by `gh`.
- **Breaks** nothing in `bower-core`'s purity.

## Dependencies

- **Built on:** EPIC-01 (deterministic replay, tags), EPIC-02 (`verify`),
  EPIC-04 (`status`, "absent is not drift"), EPIC-05 (`Forge`, `FakeForge`,
  the gate), EPIC-06/07 (render plan, byte-reproducible artifacts), EPIC-08
  (fingerprint), and the unnumbered Releases work of 3 September.
- **Coordinates with:** **EPIC-09**: edition tags cover every step on every
  line, branch refs are not frozen, and errata links use its `compare`
  template. **EPIC-11**: the receipt's diagnostic digest column, and the
  diagnostic diffs a future board shows. **EPIC-12**: the errata chapter and
  the colophon are back matter, through its generated-chapter mechanism.
  **EPIC-13**: exercise boxes in a frozen render check out edition tags.
  **EPIC-14**: `bower follow --edition <id>` walks edition tags.
  **EPIC-15**: the scrubber can scrub an edition. **`DESIGN_Forges.md`**:
  `ForgejoForge` needs `remote_tags` and `release_assets`, and its Q1
  forge-image pin.
- **Unblocks:** B4 (the toolchain board), B1 (DOIs over immutable
  releases), C5 (the living-free, frozen-paid split).

## Verification

```bash
make build                     # includes `make plan` for both books
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
make purity
cargo run -q -p bower -- --book books/hello-playbook edition cut -o target/hello-playbook
git -C target/hello-playbook tag --list 'edition-0.1.0/*' | wc -l
cargo run -q -p bower -- --book books/hello-playbook status -o target/hello-playbook
make ship-hello                # dry run: prints the edition line and the release action
cargo run -q -p bower -- --book books/hello-playbook diff --edition 0.1.0
```

Exit criteria:

1. `edition cut` on the sample book writes `editions/0.1.0.edition` with 20
   `upheld` rows, a toolchain line, and a head SHA equal to
   `git -C target/hello-playbook rev-parse v0.1.0^{commit}`.
2. Two builds in `Cut` produce identical SHAs for every tag, edition tags
   included; a book with no `version` builds exactly the tags it builds today.
3. After one block is edited, `status` reports `Moved` with one erratum,
   `build` creates no edition tags, and a dry-run `push` sends main and the
   site, skips the release, and names the reason.
4. In every state, `FakeForge` records no delete refspec, no `--prune`, no
   forced `edition-` ref, and no `--clobber`.
5. The frozen epub and PDF link only `edition-0.1.0/…`, and the HTML links
   only `step-…`.
6. `cargo tree -p bower-core -e normal` prints one line.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Living artifact names in `Moved`.** `publish --target epub` still writes `_0.1.0.epub` from a book that is no longer 0.1.0. `push` refuses to release it, which is the safety. Should the file also be renamed (`_0.1.0+errata`), or `publish` refuse frozen targets in `Moved`? The lean is that the name stays and `status` says why. |
| 2 | **Does an edition pin its site?** EPIC-08 calls an edition without its site "half an edition". The site branch is replaced wholesale as an orphan commit (`forge.rs:116-127`), so a frozen `editions/<id>/` subtree would need the old render at every push. The lean is no: the epub and PDF are the frozen reading copies, and the manifest records which `render` fingerprint the site served. |
| 3 | **Tagging the source.** Bower records the source commit and does not tag it. Should `cut` print a suggested `git tag <book>-edition-<id> <sha>` line, or say nothing about the workspace's refs? |
| 4 | **Toolchain probe.** `rustc --version` and `cargo --version` are right for both books and wrong for a non-Rust one. Should there be a `[repos.x] toolchain = "…"` key now, or when a non-Rust book exists? |
| 5 | **`Generated-By` and reproducibility.** Every `bower` release makes every cut edition `Unreproducible`, because the trailer carries the full version. Accept that, trim the trailer to `major.minor`, or drop it? Trimming it changes every SHA once. |
| 6 | **B1, DOIs and `CITATION.cff`.** Which repository gets the DOI when the source is a shared workspace: each generated repo, or a per-book release repo? `CITATION.cff` changes each final tree. Is a DOI worth that churn? |
| 7 | **B4's home.** Is the toolchain board this EPIC's Phase 6, or its own EPIC after parallel verification lands? It also overlaps EPIC-11's diagnostic drift. The lean is its own EPIC, consuming N receipts. |
| 8 | **Future pins.** When devenv (spec § 16, § 12 Q10) or a forge image (`DESIGN_Forges.md` Q1) exists, does the pin grow a field, and does a change to it count as an erratum or as `Unreproducible`? |

---

*Drafted 10 September 2026 against `ImperialBower/bower` @ `b6bfd19` ("docs:
file EPIC-09 branches, the ideas doc, and the spikes; fold them into the
backlog"). Merges spec § 13 M2, the Editions backlog row, and ideas B1, B3,
B4 and C4. No spike; the kernel half is small enough to drive out
test-first.*
