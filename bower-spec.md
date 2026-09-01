# Bower — Design Specification

**Book-driven repository generation and publishing for the ImperialBower books**
Draft 0.2 · 31 August 2026 · ImperialBower

---

## 0. One-paragraph summary

The books — *Rust for Failers: How Controlled Failing Is the Way to Build Solid
Systems* and *Controllability: The Craft of Building Observable Systems* — are the
single source of truth. Code examples live in the book source as annotated fenced
blocks. A Rust CLI (`bower`) parses the annotations, assembles a deterministic
ordered plan per target repository, and replays it as a sequence of git commits —
rebuilding each example repo from scratch, in exactly the order the book teaches
it, with stable links in both directions: every code block in the rendered book
links to the repo state after that step, and every generated commit links back to
the exact section of the book that produced it. Edit an example in chapter 3 and
rerun; the repo recreates itself with the change threaded through every subsequent
commit. Because *Rust for Failers* is a book about controlled failing, steps can
declare an **expected failure** (`compile_fail`, `test_fail`) and the generator
*verifies the failure happens* — the book's thesis, enforced by its own toolchain.
Repos are the first output target; § 13 lays the road toward Bower as a complete
publishing system, and § 14 the authoring bridges to Obsidian and Scrivener.

**The name.** In euchre, the right bower is the card that controls the game.
The book is the right bower here — everything else is derived from it — and
ImperialBower is the house. (One collision to note: `bower` was the name of a
long-dead JavaScript package manager; the crates.io ecosystem is unaffected, but
check crate-name availability — `bower` vs `bower-cli` — before publishing.
Within the org, the binary is `bower` regardless.)

---

## 1. The problem, precisely

Today the book material is littered through pkcore: narrative doc comments (the
"Play Out Saga" in `src/play/game.rs`), `DIARY.md`'s outline of the work in the
order it was done, `docs/LESSONS_LEARNED.md`, the DECON epics, and an epub built
by pandoc over every markdown file in the repo. Three things are missing:

1. **A single home.** Snippets are duplicated between prose and source, and drift.
2. **Replayability.** A reader who wants to *follow along* has no repo that grows
   chapter by chapter. The pedagogical order (`DIARY.md`'s order) is not the git
   history of pkcore, and never will be — pkcore's history is the real, messy one.
3. **Stable cross-references.** There is no way to link "this paragraph" to "this
   exact state of the code" that survives editing the book.

The fix is to invert the relationship: the book stops quoting the repo, and the
repo becomes a *build artifact of the book*.

---

## 2. Core model

### 2.1 Vocabulary

| Term | Meaning |
|---|---|
| **Book** | An mdBook project. One book can feed many repos. |
| **Block** | One annotated fenced code block (or file directive) in a chapter. |
| **Step** | One commit-to-be in one target repo. A step groups one or more blocks. |
| **Plan** | The fully-resolved, ordered list of steps for one repo — a pure value. |
| **Tree state** | The complete file tree of the repo after a step — also a pure value. |
| **Replay** | Turning a plan into an actual git repo, one commit per step. |

### 2.2 The pipeline

```
book source (.md)
      │  parse annotations
      ▼
  Vec<Block> ──group by (repo, step)──▶ Vec<Step> ──order──▶ Plan   (pure)
      │                                                        │
      │                                                        ▼
      │                                              fold: TreeState per step   (pure)
      │                                                        │
      ▼                                                        ▼
 link map (step ⇄ book anchor)                        git replay + verify (I/O)
      │                                                        │
      ▼                                                        ▼
 mdbook-bower preprocessor                        local repo, tags, optional push
 (injects links into rendered book)               (commit trailers link back)
```

Everything left of "git replay" is a **domain kernel**: no I/O, no git, no
serialization in its public API. Input is chapter text; output is a `Plan` and a
sequence of `TreeState`s. This is the crate that gets the exhaustive test suite,
and it is what makes the whole system auditable: given the same book source, the
plan and every tree state are pure functions of it.

---

## 3. Annotation format

### 3.1 Design constraints

- Must be **invisible in every renderer** used today: mdBook HTML, pandoc → epub
  (the existing `build_epub.sh` flow), and plain GitHub markdown preview.
- Must not fight mdBook's own info-string flags (`rust,ignore`, `no_run`) or its
  `#`-hidden-line convention — it should *exploit* them.
- Must be writable by hand mid-sentence while drafting, without breaking flow.

HTML comments satisfy all three. An annotation is an HTML comment immediately
preceding a fenced block, carrying TOML-flavored key-values. The canonical
directive word is `bower`; `bf` is accepted as a short alias for drafting speed.

### 3.2 The directive

```markdown
<!-- bower repo="failers" file="src/rank.rs" -->
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rank {
    Ace, King, Queen, Jack, Ten, // …
}
```​
```

Full key set:

| Key | Required | Default | Meaning |
|---|---|---|---|
| `repo` | yes | — | Target repo name, declared in `bower.toml` |
| `file` | yes* | — | Path inside the repo this block writes |
| `op` | no | `create` | `create` \| `replace` \| `region` \| `append` \| `delete` \| `copy` \| `none` |
| `step` | no | auto | Explicit step id for grouping/ordering (see § 4) |
| `msg` | no | derived | Commit message subject for the step |
| `expect` | no | `pass` | `pass` \| `compile_fail` \| `test_fail` \| `none` |
| `region` | with `op="region"` | — | Named region in the file to replace |
| `src` | with `op="copy"` | — | Book-relative asset path (binary files, fixtures) |
| `hidden` | no | `true` | Whether mdBook `#`-hidden lines are included in the repo file |
| `show` | no | all marked | Named display span(s) to render from this block (see § 3.4) |
| `include` | no | — | Pull block content from the block library instead of inline (see § 14.3) |
| `notebook` | no | — | `play`: a live notebook cell, not repo content (see § 15) |

*`file` is not required for `op="delete"` steps declared with `paths=[…]`, nor
for pure-narrative steps (`op="none"`) that exist only to carry a commit message
(e.g., a refactoring the book describes but doesn't show in full).

### 3.3 Fragments vs. full files — the two assembly mechanisms

The central tension: the book wants to show 10 lines; the repo needs the whole
compiling file. Two complementary mechanisms:

**Mechanism 1 — mdBook hidden lines.** In a `rust` block, lines starting with
`# ` are hidden by mdBook's renderer but are part of the block. The repo file
gets *all* lines (hidden included, `# ` stripped); the rendered book shows only
the visible ones. One block is then simultaneously the honest full file and the
pedagogical fragment. Fine for small files and a stray line or two.

**Mechanism 2 — named regions.** For files that grow across chapters, the first
`create` establishes region markers as ordinary Rust comments:

```rust
// bower:begin from_char
// bower:end from_char
```

A later block with `op="region" region="from_char"` replaces exactly the text
between the markers, showing the reader *only the new code* while the kernel
computes the full resulting file. Markers are stripped from the final tree by
default (configurable per repo — leaving them in is itself a nice observability
touch for readers browsing the repo).

`append` covers the common "now add the test module at the bottom" move without
requiring markers.

### 3.4 Display markers — what the reader sees

Assembly (§ 3.3) decides what ends up in the repo file. **Display markers** are
the independent, orthogonal concern: which part of a block the *rendered book*
shows. The full code is always there — in the block, in the generated repo, and
(in HTML) one click away — but the final build prints only the spans that earn
their place on the page. Inside any block, ordinary comments mark the spans:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rank { /* … established earlier … */ }

// bower:show
impl From<char> for Rank {
    fn from(c: char) -> Self {
        match c {
            'A' | 'a' => Rank::Ace,
            // …
        }
    }
}
// bower:show end
```

Rules:

- **No markers → the whole block renders** (the status quo; small examples pay
  no tax).
- **Any marker present → only marked spans render.** Unmarked code is elided.
- Spans can also be named — `// bower:show begin from_char` … `// bower:show end`
  — so one full-file block can be reused across chapters with a directive key
  `show="from_char"` selecting a different slice each time.
- Display markers are **always stripped** from the generated repo tree (unlike
  region markers, which are configurable) — they are book concerns, not code.

**How elision renders, per target.** In mdBook HTML, elided spans are emitted as
`#`-hidden lines, so the reader gets mdBook's standard eye-toggle: the code is
literally all there in the page, expandable inline. In the pandoc/epub build
(which has no toggle), each elided span collapses to a single comment line —
`// ⋯ 14 lines elided — full file: <link>` — pointing at the real file.

**Line-anchored source links.** Because replay is deterministic, Bower knows
the exact path *and line range* of every displayed span in the generated tree at
that step. The preprocessor footer under each block therefore links both to the
full file at the step's tag and to the precise lines shown, e.g.
`…/blob/step-012-rank-enum/src/rank.rs#L18-L31`. The link target is a template,
so GitHub is the default but not an assumption:

```toml
[repos.failers.links]
blob = "https://github.com/ImperialBower/failers/blob/{tag}/{path}#L{start}-L{end}"
```

Point it at Codeberg, sourcehut, or a self-hosted forge and every book link
follows. `bower verify` recomputes the line map on every build, so links can
never drift from the code — a stale line anchor is a build error, not a
reader's discovery.

Display markers deliberately overlap with mechanism 1 (`#`-hidden lines): hidden
lines remain fine for a stray line or two, but marking what to *show* scales to
large files where prefixing every boilerplate line with `# ` does not, and it
keeps the block copy-paste-runnable while drafting.

### 3.5 Multi-file steps

One step = one commit, but a commit often touches several files (a new module
plus its `mod` declaration plus `Cargo.toml`). Blocks sharing an explicit
`step` id merge into one step:

```markdown
<!-- bower repo="failers" step="rank-enum" file="src/rank.rs" -->
…block…
<!-- bower repo="failers" step="rank-enum" file="src/lib.rs" op="region" region="mods" -->
…block…
```

Blocks without a `step` id get an auto-generated one and form single-block steps.

### 3.6 Scaffolding

Repo-level boilerplate the book never shows (LICENSE, `.gitignore`, `rustfmt.toml`,
the generated-repo README banner) comes from a template directory declared in
`bower.toml`, applied as step 0 ("Initial commit — scaffolding"). The
generated README states loudly: *this repo is generated from the book; do not
open PRs here; here is the link to the book.* The template is also where the
reader environment lives — `devenv.nix` and its lockfile, § 16 — so every tag
of every generated repo is one command away from a working toolchain.

---

## 4. Ordering

The default order is **document order**: books are read front to back, so steps
are sequenced by (chapter order in `SUMMARY.md`, block position within chapter),
filtered per repo. This makes the common case zero-config.

Explicit `step` ids plus an optional `after="step-id"` key handle the exceptions
(a chapter that interleaves two repos, an appendix that patches an early step).
Cycles are a plan-time error. The resolved plan is written to
`bower.lock` — a human-readable manifest of (seq, step id, chapter anchor,
files, expect) per repo — so that reordering shows up in diffs and code review,
not as silent history rewrites.

---

## 5. Replay: from plan to git history

### 5.1 Deterministic commits

Regeneration always starts from an empty tree and replays the full plan
(incremental regeneration is a non-goal — 100 commits of small files is
sub-second work for git). Determinism rules:

- Author and committer are fixed identities from `bower.toml`.
- Timestamps are synthetic: `book_epoch + seq × 1 minute`. Regenerating an
  unchanged book yields **byte-identical SHAs**.
- Commit subject: `msg` if given, else derived from the nearest heading
  (`ch03: Introduce the Rank enum`).

### 5.2 Commit trailers — repo → book links

Every generated commit carries trailers:

```
ch03: Introduce the Rank enum

Book-Source: rust-for-failers/src/ch03-ranks.md#step-rank-enum
Book-Url: https://imperialbower.github.io/rust-for-failers/ch03-ranks.html#step-rank-enum
Bower-Step: failers/012
Generated-By: bower v0.x
```

A generated `STEPS.md` at the repo root lists every step with its book link —
the repo's own table of contents back into the book.

### 5.3 Tags — book → repo links

SHAs change whenever an earlier step changes, so the book never links to SHAs.
Every step gets an annotated tag `step-012-rank-enum`; every chapter boundary
gets `ch03-end`. Tags are recreated on regeneration; names are stable as long as
step ids are. The rendered book links to
`…/failers/tree/step-012-rank-enum` (browse the state) and
`…/failers/commit/step-012-rank-enum` (see the diff — git resolves tags here).

### 5.4 The mdBook preprocessor

`mdbook-bower` (same crate, `preprocessor` feature) runs at book build time:

- strips `bower` directives from output (pandoc/epub ignores them natively);
- applies display markers (§ 3.4): marked spans render, elided code becomes
  toggle-hidden lines in HTML or a linked elision comment in epub;
- injects under each annotated block a small footer:
  *`src/rank.rs` L18–31 · step 12 of failers · [full file] · [view diff] ·
  [browse repo here]* — the file and line links resolved through the `links.blob`
  template against the generated tree;
- injects an anchor `#step-<id>` so commit trailers land on the exact block;
- fails the book build if a directive references an unknown repo or a region
  that doesn't exist yet — book CI catches annotation rot.

### 5.5 Publishing repos

`bower push` force-pushes with lease to the configured GitHub remote
(`ImperialBower/<repo>`), tags included, and can create the repo via the GitHub
API on first push. Local-only is the default; push is always an explicit flag or
a separate CI job. Generated repos get `archived: false` but a branch protection
exception — they are build artifacts, and force-push is their normal life.

---

## 6. Verification — where the book eats its own cooking

`bower verify` replays the plan and, at each step, runs the repo's
verification command (default `cargo test`, configurable per repo) against the
step's `expect`:

| `expect` | Verifier asserts |
|---|---|
| `pass` (default) | Build and tests succeed at this tree state |
| `compile_fail` | `cargo check` **fails** — and, optionally, stderr matches a stored pattern (trybuild-style) |
| `test_fail` | Build succeeds, named test **fails** |
| `none` | Step is skipped (prose-only steps, mid-refactor states) |

This is the load-bearing feature for *Rust for Failers*: the book's controlled
failures are executable claims. A chapter that says "this won't compile, and
here's what the compiler tells you" is verified in CI, and the day a new rustc
changes the diagnostic, the build tells you before a reader does. For
*Controllability*, `verify` doubles as state coverage over the book itself:
every intermediate state a reader can check out is a state the test suite has
visited.

Verification of N steps is embarrassingly parallel (each tree state is
independent); `verify --step <id>` and `--from <id>` support fast local loops.

### 6.1 Failure modes of the tool itself

In keeping with both books, plan-time errors are first-class and exhaustive:
unknown repo, conflicting writes to one file within a step (a whole-file write
sharing the file with anything else, or two region ops on the same region —
distinct regions and appends compose fine), `region` before its markers exist,
`replace` of a file never created, orphaned `after` references, two books
claiming the same repo. Every error carries the chapter, line, and step id. The
kernel's error enum *is* a chapter draft for *Rust for Failers*.

---

## 7. Configuration

`bower.toml` at the book root:

```toml
[book]
epoch = 2026-09-01T00:00:00Z          # timestamp base for deterministic SHAs
site  = "https://imperialbower.github.io/rust-for-failers"

[identity]
name  = "ImperialBower Bower"
email = "bower@imperialbower.example"

[repos.failers]
github   = "ImperialBower/failers"     # optional; enables push
template = "templates/failers"         # step-0 scaffolding
verify   = "cargo test --quiet"
keep_region_markers = false

[repos.failers-clock]                  # a second repo fed by the same book
github = "ImperialBower/failers-clock"
```

CLI surface:

```
bower plan   [--repo R]        # resolve + print the plan, write bower.lock
bower build  [--repo R] [-o D] # replay into local repo(s)
bower verify [--repo R] [--step S | --from S]
bower push   [--repo R]        # force-push-with-lease + tags
bower status                   # book ⇄ lock ⇄ local repo drift report
bower publish [--target T]     # render outputs: html | epub | pdf (§ 13)
```

---

## 8. Crate layout

Following the domain-kernel pattern:

| Crate | Purity | Contents |
|---|---|---|
| `bower-core` | pure | Directive parser, step grouping, ordering, `Plan`, `TreeState` fold, region + display engines, error enum. No git, no filesystem, no serde in the public API. |
| `bower-testkit` | pure | Generators for annotation textures: minimal book, multi-repo book, region-heavy book, every error case as a fixture. Property tests: replay determinism, region idempotence, plan ordering is total. |
| `bower` (bin) | I/O | CLI, `gix` (or `git2`) replay, verification runner, GitHub push, `mdbook-bower` preprocessor behind a feature. |

The testkit ships the **data textures** for book sources — pathological chapter
orderings, unicode in commit messages, CRLF blocks, nested fences — and a state
coverage report: which (op × expect × mechanism) combinations the fixture corpus
exercises — display-marker textures included (unclosed spans, nested spans,
markers inside strings, a block that is 100% elided). The line-map property
test is the important new one: for every displayed span, the linked line range
in the folded `TreeState` contains exactly that span's text. The tool for the
*Controllability* book gets built the way the *Controllability* book says tools
should be built.

---

## 9. Worked example — validated against real pkcore material

Taking the opening of `DIARY.md` ("EPIC: Display HandRank") as the first chapter
of *Rust for Failers*, the annotation flow for three steps of the `Rank` story:

**Step 1 — the enum (create).** One block, `file="src/rank.rs"`, visible lines
show the enum; hidden `# `-lines carry the derives boilerplate if the prose
hasn't earned them yet. Commit: `ch01: Introduce Rank`.

**Step 2 — `From<char>`, the failing version.** The book shows an intentionally
non-exhaustive match. `op="region" region="from_char"`, `expect="compile_fail"`.
The verifier asserts rustc rejects it — the "controlled failing" beat, enforced.

**Step 3 — the fix plus rstest brute-force tests.** Two blocks sharing
`step="rank-from-char"`: the corrected region, and an `op="append"` test module
using rstest ("TELL THE HEROES STORY" / brute-force testing, per the diary).
Commit: `ch01: Rank::from(char), tested the brute-force way`.

The "Play Out Saga" doc comment in `src/play/game.rs` maps the same way for
*Controllability*: the narrative moves into a chapter, the `PlayOut` trait
refactor becomes a region step over `src/play/game.rs` in a `controllability`
repo, and the "BOOM!!! post PlayOut" state is a tagged, browsable tree instead
of a memory. Both walkthroughs exercised every `op` except `delete`/`copy` and
surfaced one requirement now in § 3.2: prose-only steps (`op="none"`) for
refactors the book narrates without printing — the diary is full of those.

---

## 10. What this deliberately does not do

- **No incremental replay.** Full regeneration every time; determinism makes it
  cheap and removes an entire class of drift bugs.
- **No extraction from pkcore.** pkcore's real history stays untouched; snippets
  migrate *into* book chapters by hand (an editing task, not a tool feature).
  `bower status` can, later, grow a `--grep-source` helper that finds
  book-marked doc comments to migrate.
- **No support for editing generated repos.** They are read-only artifacts;
  the README banner and force-push make that unambiguous.
- **No mdBook lock-in in the kernel.** The parser sees markdown text; the
  preprocessor is one consumer. The pandoc/epub flow keeps working because
  directives are HTML comments.

---

## 11. Build plan

**Phase 1 — kernel.** `bower-core`: parser, plan, tree fold, region + display
engines, error enum. `bower-testkit` alongside, fixtures first.

**Phase 2 — replay.** `build` + `plan` + `bower.lock`, deterministic commits,
tags, trailers, `STEPS.md`. Golden test: byte-identical SHAs across two runs.

**Phase 3 — verify.** The `expect` matrix, per-step verification, parallel runs.
This is the phase that makes the tool worth having.

**Phase 4 — surfaces.** `mdbook-bower` preprocessor (links + anchors +
build-time validation), `push` with GitHub repo creation, `status` drift report.

**Phase 5 — migration.** Stand up the *Rust for Failers* mdBook, move the
DIARY/doc-comment material chapter by chapter, generate `failers` for real.

**Phase 6+ — publishing maturity and authoring bridges.** § 13 and § 14; each
milestone there is scoped only after Phase 5 has produced a real book end to end.

---

## 12. Open questions

1. **gix vs git2** for replay. gix is pure-Rust and fits the no-C-deps instinct;
   git2 is boring and complete. Phase 2 decision, kernel unaffected.
2. **One book per repo of book-source, or one workspace holding both books?**
   A single `books` workspace sharing templates and CI is the current lean.
3. **Diagnostic snapshots for `compile_fail`** — store expected stderr like
   trybuild, or assert failure only? Snapshots are stronger claims but churn
   with rustc versions; suggest failure-only by default, snapshot opt-in.
4. **Should generated repos carry GitHub Actions** that re-verify on push, as a
   public badge that every step passes? Cheap and on-message.
5. **Crate naming on crates.io.** `bower` may or may not be free (and carries a
   faint echo of the retired JS package manager); fallbacks `bower-cli` /
   `imperial-bower` with the binary still named `bower`. Check before Phase 1.
6. **Block library: core or extension?** (§ 14.3.) It starts as an extension;
   promote it to core only if Scrivener authoring proves worth keeping.
7. **Wheel distribution for notebooks** (§ 15.3): CI-built wheels attached to
   chapter-end tags, a per-book PyPI package, or build-on-install via maturin.
8. **`expect` for play cells** — a Python cell that *should* raise, extending
   the § 6 matrix across the language boundary.
9. **JupyterLite/pyodide** — native PyO3 wheels don't load in the browser
   today; a zero-install playable book is worth tracking maturin's pyodide
   support for.
10. **devenv generation** (§ 16): hand-author `devenv.nix` in each repo
    template, or have Bower derive it from `bower.toml` +
    `rust-toolchain.toml`? And do editions pin nixpkgs per-edition, or share
    one channel per book?

---

## 13. Toward a publishing system

Repos are the first output target, not the last. The same kernel discipline
extends: publishing stays a pure fold from book source to a *render plan*, with
renderers (mdBook, pandoc, a PDF engine) as replaceable I/O at the edges — the
way `TreeState` already relates to git. Maturity ladder, each rung shippable on
its own:

**M0 — this spec.** Generated repos, verified steps, mdBook HTML, the existing
pandoc epub flow untouched.

**M1 — unified targets.** `bower publish --target html|epub|pdf|ipynb` owns the
whole render: one display-marker engine feeding all targets (replacing
`build_epub.sh`), per-target elision rules (§ 3.4), PDF via Typst or LaTeX with
proper code typography, and the Jupyter notebook target (§ 15). The book builds
from one command, and every target agrees about what is shown and what is
linked.

**M2 — editions.** A published edition is a *pinned triple*: book source commit,
`bower.lock`, and the generated repo tags — stamped as `edition-1.0` across
book and repos. Edition tags are never deleted or force-moved, so a reader of
the 1.0 epub follows 1.0 links forever, while `main` moves on. Errata become
first-class: a correction regenerates `main`, and `bower diff --edition 1.0`
emits the reader-facing errata list (which chapters, which steps, what changed)
mechanically rather than by memory.

**M3 — author tooling.** `bower new chapter`, a watch mode that rebuilds book +
plan on save and fails fast on annotation errors, `draft = true` chapters
(verified but excluded from publish), and sample extraction — a free sample is
just a render plan over a subset of chapters, with its repo links pointing at
the full generated repos.

**M4 — the imprint.** Both books (and later ones) in one workspace: shared
templates and CI, cross-book step links (*Controllability* citing a *Failers*
step by id), a catalog page, and one `bower publish --all` that rebuilds every
book, every repo, every target, and verifies the whole estate. At this rung
Bower is a publishing system that happens to have started as a repo generator.

What Bower does **not** aspire to: DRM, storefronts, or payment. Leanpub, Gumroad
et al. remain the distribution channel; Bower's job ends at producing verified,
beautiful, deeply cross-linked artifacts.

---

## 14. Authoring bridges: Obsidian and Scrivener

Two very different tools, two very different integration stories. The honest
summary up front: **Obsidian is a natural, cheap, two-way fit; Scrivener is a
one-way street that needs a design accommodation (§ 14.3) to be safe around
code.**

### 14.1 Obsidian — the vault is the book source

Obsidian and Bower already speak the same language: plain markdown files on
disk. The integration is mostly *removing accidental friction*, in three
tiers:

**Tier 1 — works today, zero code.** Open the book source directory as a vault.
`bower` directives are HTML comments — invisible in Obsidian's reading view,
visible and editable in source mode. Fenced blocks, display markers, everything
round-trips because Obsidian never rewrites files it isn't editing.

**Tier 2 — adapters in the preprocessor.** Two Obsidian habits need translating
at build time, not in the vault: wikilinks (`[[ch03-ranks]]` →
mdBook-relative links) and the `SUMMARY.md` mdBook requires — generated from
folder order or a frontmatter `order:` key, so the vault's structure is the
book's structure and `SUMMARY.md` becomes a build artifact. Both are small,
pure functions in `bower-core`; both are also useful to non-Obsidian authors.

**Tier 3 — a Bower plugin (later, optional).** Obsidian plugins are TypeScript
against a stable API, which cuts against the all-Rust instinct — so the plugin
stays thin and dumb: live directive validation (squiggle on an unknown repo or
region), a step badge in the gutter showing `repo/seq`, and a "jump to
generated file" command that opens the `links.blob` URL. All the intelligence
stays in `bower-core`; the plugin can shell out to `bower plan --json`. The
vault's graph view over `[[step-id]]` links is a free visualization of the
book's dependency structure — pleasingly, the same shape as the DECON
MANIFEST's build-order digraph.

### 14.2 Scrivener — compile in, never round-trip

Scrivener's project format is an RTF-based package with an XML index; it is not
a source of truth Bower can parse, and pretending otherwise would put an
irreversible, lossy format at the head of the pipeline. Two integration paths
exist; both treat Scrivener as *upstream drafting*, never as the home of code:

**Path A — the compile pipeline (recommended).** Draft prose in Scrivener;
compile to MultiMarkdown/pandoc-flavored markdown (Scrivener 3's compiler is
good at this); Bower consumes the compiled output as read-only input, merged
with code that never entered Scrivener (§ 14.3). Direction is strictly
Scrivener → Bower; edits flow back by editing in Scrivener and recompiling.

**Path B — external folder sync.** Scrivener's sync-with-external-folder mirrors
the draft as plain-text files that Bower could read directly, giving a loose
two-way loop. It is tempting and fragile: sync conflicts, filename mangling,
and Scrivener's enthusiasm for typographic punctuation make it a poor place for
fenced code. Support it only as a variant of Path A — prose sync, code
elsewhere.

**The hazard to design around:** Scrivener's smart punctuation will silently
turn `"` into `"` inside anything it considers prose, and a single curly quote
in a code block is a compile error two chapters later. The rule, enforced by
`bower verify` refusing unexpected Unicode in code: **code never lives in
Scrivener.** Which motivates:

### 14.3 The block library — code beside the prose, not inside it

An optional indirection that serves both tools (and plain-editor authors): a
block may live in a companion file under the book source instead of inline in
the chapter, referenced by the `include` key:

```markdown
<!-- bower include="blocks/ch01/rank-enum.md" -->
```

The library file holds the directive defaults and the fenced block; the chapter
holds one line. The block library is still *book source* — same directory, same
git history, same single source of truth; this is not a second home for code,
just a second room. What it buys:

- **Scrivener safety**: prose drafts in Scrivener mention blocks by include
  line (easily typed, immune to smart quotes); code is authored in a real
  editor with rust-analyzer.
- **Obsidian ergonomics**: block files are notes; embedding and backlinking
  work on them natively.
- **Reuse**: one block, displayed in two chapters with different `show=` slices,
  without duplication.

Cost: one more level of indirection, and the drafting flow loses "the code is
right there in the prose." Hence open question 6: it ships as an extension, and
gets promoted to a core recommendation only if Scrivener authoring earns its
keep in practice.

### 14.4 What stays true regardless of editor — and of target

The invariant that makes all of this safe: **Bower's contract is with the
markdown on disk, never with the editor.** Obsidian, Scrivener-compiled output,
Zed, or `ed` — the kernel sees text, the plan is a pure function of it, and
`bower verify` is the arbiter no editor can charm. Authoring tools may be
adopted, mixed, and abandoned without a migration. The invariant extends to
§ 15's notebooks: every target is rendered from the same markdown on disk, so
adding a target never forks the book.

---

## 15. The notebook target: playable chapters

Reading about controlled failing is one thing; poking it is another.
`bower publish --target ipynb` emits **one Jupyter notebook per chapter**:
prose becomes markdown cells; each displayed span becomes a fenced, read-only
view of the Rust with its line-anchored source links; and **play cells**
become live Python code cells a reader can run and mutate. The execution
surface is the pkcore.py pattern: a PyO3/maturin binding crate wrapping the
chapter's Rust, imported from a Python kernel.

### 15.1 The play track

A play cell is a new block kind in the book source — Python, annotated, and
never part of any repo tree:

```markdown
<!-- bower repo="failers" notebook="play" -->
```python
from failers import Rank
Rank.from_char("A")   # now break it: what does 'Z' do?
```​
```

Binding: a play cell attaches to the **nearest preceding step** of its repo in
document order — you play with what you just built — or to an explicit
`step="rank-enum"`. Three new plan-time errors cover the failure modes: a play
cell with no step to attach to, a play cell naming a step that doesn't exist,
and a play cell carrying tree-affecting keys (`file`, `op`, `region`, `src`,
`paths`), which is a category confusion the kernel refuses.

Other targets keep their meaning: HTML renders play cells as a "try it" aside
under the step (with a link to the notebook); epub/pdf drop or footnote them
(per-book config). The notebook target renders them live. One source, every
target.

### 15.2 The wrapper is book content, not magic

Bower does **not** generate PyO3 code — auto-generated bindings would be
exactly the kind of unauditable machinery both books argue against. Instead
the binding crate is part of the generated repo and is *authored in the book*
like everything else: the repo template ships a `bindings/` skeleton (pyo3
dependency, maturin metadata, empty `#[pymodule]`), and chapters grow it with
ordinary blocks — `file="bindings/src/lib.rs"`, region ops, the works.

This is deliberate teaching material, not overhead: wrapping a kernel for a
scripting surface is the *Controllability* thesis made concrete (the
observable, controllable surface of a domain kernel), and the wrapper's error
mapping — Rust `Result` to Python exception — is a *Rust for Failers* chapter
waiting to happen. The bindings evolve step by step with everything else and
are visible at every tag.

### 15.3 Pinning and execution

Each chapter's notebook must import exactly the code state that chapter ends
at. The generated first cell installs the wheel for the chapter-end tag:
preferably a CI-built wheel published against the `ch03-end` tag; falling back
to `pip install "git+https://…/failers@ch03-end#subdirectory=bindings"` with
maturin building locally. Regenerating the book regenerates the pin — the
notebook can never drift ahead of or behind its chapter.

`bower verify --target ipynb` executes every notebook against its pinned wheel
(headless, `nbconvert --execute`) in CI: a play cell that errors fails the
build. Whether a play cell can *declare* an expected exception — controlled
failing crossing the language boundary — is open question 10; it is the
natural extension of the § 6 `expect` matrix and probably wants the same
vocabulary.

### 15.4 Scope and alternatives

The kernel work (directive key, play-cell binding, errors, `play_cells` on
planned steps) lands in `bower-core` now; the `.ipynb` renderer and wheel
plumbing are Phase 4 surfaces alongside the mdBook preprocessor. The evcxr
Rust kernel — notebooks in raw Rust, no wrapper — was considered and set
aside as the default: it puts a full Rust toolchain between the reader and
their first keystroke, while the pkcore.py pattern is already proven in this
codebase. It remains viable as a per-book execution mode later. JupyterLite
(browser-only notebooks) currently can't load native PyO3 wheels, but
maturin's experimental pyodide support is worth tracking — a zero-install
playable book would be a real distribution edge.

---

## 16. Reader environments: devenv

The books' promise is "check out any step, run it, and watch it fail on cue."
That promise dies at toolchain friction: the reader who clones
`failers@step-012-rank-enum` needs the *exact* rustc the book was verified
with — a `compile_fail` step is a claim about a specific compiler — plus, for
notebook chapters, Python, maturin, and Jupyter in compatible versions.
"Install these nine things first" is where follow-along readers quit.

### 16.1 The fit

devenv (Nix underneath, declarative on top) matches Bower's whole philosophy:
a `devenv.nix` describing the environment and a `devenv.lock` pinning it,
checked in, versioned, reproducible. `devenv shell` — or automatic entry via
direnv — and the reader is in the environment the book was built and verified
in, at any tag, on any machine Nix runs on.

This extends Bower's pinning discipline one layer down. The book pins the
code (steps, tags, editions § 13 M2); devenv pins what the code *runs on*.
An edition then freezes the toolchain too: readers of the 1.0 epub get the
1.0 rustc forever, which is exactly what keeps a `compile_fail` step failing
*for the stated reason* years later — the reader-side answer to open
question 3's diagnostic-drift problem.

### 16.2 Mechanics

- **Template, step 0.** `devenv.nix` + `devenv.lock` + `.envrc` ship in the
  repo template (§ 3.6), so they exist at every tag from the first commit.
  The Rust version derives from the repo's `rust-toolchain.toml`; repos with
  a `bindings/` crate add Python, maturin, and Jupyter.
- **Environment changes are steps.** Bumping `devenv.lock` or adding a tool
  is an ordinary annotated block — narratable in the book ("we now need
  maturin; here's why"), verified like any other step, visible in the diff.
- **One environment, three audiences.** CI's `bower verify` runs inside the
  same devenv the reader gets, collapsing "works in CI" and "works for the
  reader" into one fact. The books workspace itself gets its own devenv
  (bower, mdBook, pandoc/Typst, maturin) — contributor onboarding is one
  command too.
- **`devenv up` as the playground.** devenv's process runner gives notebook
  chapters a one-liner: it launches Jupyter with the chapter wheel on the
  path (§ 15.3), and can host a `cargo watch` test loop for the follow-along
  reader.

### 16.3 Posture: blessed path, not the only path

Nix is a real ask — smooth on macOS and Linux, WSL2 territory on Windows,
and a conceptual speed bump for readers who just want to see code fail. So
devenv is the *blessed* path, not a gate: the generated README keeps a plain
`rustup` fallback (three lines, honest about version requirements), and a
devcontainer/Codespaces route — generated from the same environment
definition, devenv can export containers — covers the browser-only reader.
The rule mirrors § 14.4: the repo's contract is with its committed files;
devenv is the most faithful way in, never the only one.

---

*Spec drafted against pkcore @ HEAD 2026-08-31 (DIARY.md, docs/deconstruct,
scripts/build_epub.sh, src/play/game.rs). Draft 0.2: system named Bower;
added display markers (§ 3.4), publishing roadmap (§ 13), authoring bridges
(§ 14); notebook target (§ 15); reader environments via devenv (§ 16).
**Phase 1 is built**: the `bower` workspace
ships `bower-core` and `bower-testkit` — 73 tests, clippy-pedantic clean, spec
§ 9's rank saga as an executable fixture, § 15 play cells bound in the plan,
full state coverage over (op × expect × mechanism). Next: Phase 2 replay.*
