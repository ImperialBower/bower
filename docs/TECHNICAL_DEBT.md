# Technical Debt

> Maintained by the `/backlog` skill. Items tagged 🤖 were proposed by automated
> review — review and edit them; they are suggestions, not facts.
>
> Created 1 September 2026 at `b3def07`. Last refreshed 5 September 2026, after
> the first real Phase 5 book, which turned up four defects before it served a
> single page: the book-name fallback, a garbled refusal message, a
> `--force-with-lease` that could never push twice (all three fixed), and Pages
> never being enabled (below).
>
> There are **no `TODO`, `FIXME`, `HACK`, or `XXX` markers anywhere in this
> codebase**, so nothing here came from a code comment. Most items were written
> down deliberately in an EPIC corrigendum at the moment the shortcut was taken;
> the rest were found by `make ayce`.

## Tracked debt

- [x] ~~**`bower verify`'s default work directory was inside the workspace.**~~
  Closed 3 September 2026. `--work` defaulted to `target/bower-verify`, which
  put the scratch cargo package inside the book's own workspace; cargo refused
  to build it and all twenty of the sample book's true claims reported false.
  The default now lives under the system temp directory, keyed by book, and a
  nested work directory stops with a named error rather than judging the book
  (`bower/src/verify.rs`, `default_work_dir` and `nested_workspace`).

- [ ] **`bower verify` does not run the book's own gate.**
  `books/hello-playbook/bower.toml` declares `check = "cargo check"` and
  `verify = "cargo test"`. Neither runs `cargo fmt --check` or clippy, so the
  generated repo can fail `make ayce` while every step verifies clean. This is
  not hypothetical: it happened, and was caught by eye rather than by the tool
  (EPIC-02 corrigendum items 4 and 5). The obvious fix — point `verify` at
  `make ayce` — does not work, because the `Makefile` does not exist until step
  3 and `make audit` needs `cargo-audit` and `cargo-deny` installed. Likely
  shape: a list of verify commands per step range, or a `verify_from = "<step>"`
  key.

- [x] ~~**Multi-repo books are untested.**~~ Closed 2 September 2026 by
  `bower/tests/multi_repo.rs`. The gap was hiding a real bug — see
  `docs/DEFECT_Review_Findings.md` item 2.

- [ ] **The push gate identifies a book by its directory basename.**
  `replay::book_name` is `book_root.file_name()`, so two books in different
  directories that happen to share a basename would each pass the other's
  `STEPS.md` marker check in `bower push`. Narrow, but the gate is the one place
  in this project where a wrong answer costs somebody else's repository. A
  stronger identity — a `book_id` in `bower.toml`, or the `github` remote itself
  — would close it. Flagged by review, not yet verified.

- [ ] **`bower push` does not consider lock drift.**
  `plan_push` refuses an unbuilt or stale *repository*, but ignores a stale
  `bower.lock`. What gets pushed is still correct, because `repo_drift` compares
  against the current plan — so this is a warning worth printing rather than a
  bug. Flagged by review, not yet verified.

- [ ] **The preprocessor has never seen a `README.md` chapter.**
  mdBook treats `README.md` specially, rendering it as `index.html`.
  `mdbook::chapter_path` prefixes whatever `path` the JSON carries with `src/`,
  and `trailers::html_name` maps `.md` to `.html` — neither knows about that
  rename, so a book with a `README.md` chapter may produce a `Book-Url` that
  404s. No current book has one. Flagged by review, not yet verified.

- [ ] **Verification is sequential.**
  Spec § 6 calls verification embarrassingly parallel. It is not parallel
  (`bower/src/verify.rs`). Twenty steps take 16 seconds thanks to one shared
  `CARGO_TARGET_DIR`, so parallelism would have been premature — but a real
  book with hundreds of steps will need it, and `std::thread::scope` would add
  no dependency.

- [ ] **No stderr snapshots for `compile_fail`.**
  `bower verify` asserts the check command failed and captures stderr for the
  report, but matches nothing against a stored pattern. Spec § 12 Q3 was closed
  as failure-only (EPIC-02, Phase 5c). A book that says "here is what the
  compiler tells you" cannot yet prove the message is still that.

- [ ] **File modes are inferred from a shebang.**
  The kernel does not model file modes — `FileBody` is text or bytes
  (`bower-core/src/tree.rs:15`) — so `bower/src/materialize.rs` marks a file
  executable when it starts `#!`. That covers `bin/security-scan` and nothing
  else. A book needing an executable without a shebang has no way to say so;
  the honest fix is a `mode=` directive key in the kernel, not a longer
  heuristic (EPIC-01 corrigendum item 2).

- [ ] **The `template/` directory has no test.**
  `books/hello-playbook/template/` is applied as step 0 by
  `bower/src/replay.rs`, and `bower verify` overlays it before checking a step
  — but nothing asserts its contents. A licence file deleted by accident would
  be noticed by nobody.

- [x] ~~**The epub elision rendering is designed but unbuilt.**~~ Closed
  2 September 2026 by EPIC-06: `bower publish --target epub` renders it, and the
  comment now carries the `full file:` link spec § 3.4 asked for.

- [ ] **One copyleft crate in the dependency tree.**
  `uluru` (MPL-2.0) arrives through `gix-pack` → `gix`, and is the only
  non-permissive licence in a workspace that is otherwise MIT OR Apache-2.0.
  MPL-2.0 is weak, file-level copyleft: using `uluru` unmodified as a dependency
  does not affect this workspace's own terms; modifying its files would. It is
  allowed explicitly in `deny.toml` with that reasoning written beside it, so it
  is a decision rather than an accident. Revisit if `gix` is ever swapped for
  `git2` (EPIC-01 corrigendum) or if a downstream consumer forbids copyleft
  outright.

- [ ] **`GitHubForge` is untested.**
  `bower/src/forge.rs`'s real implementation — every `gh api` call, both
  `git push` invocations, and now `publish_release`'s `gh release view` /
  `create` / `upload --clobber` — is executed by no test. This is the
  acknowledged last inch of EPIC-05: the *decisions* are tested against
  `FakeForge`, and the pure parts that could be extracted (`remote_url`,
  `state_from`, `is_not_found`, `release_absent`) were. What remains untested is
  the shelling-out itself, and it cannot be covered without a network and a
  repository we are willing to destroy.

  `bower push --execute` **has** now been run for real — `hello-playbook` is
  live with its `gh-pages` site — but the release path has not. The first
  `make ship-hello-execute` carrying a release will be its first real exercise,
  and `gh release view`'s "not found" wording is the one string that decides
  between creating a release and clobbering one.

- [ ] **`bower push` ships a site branch but never turns Pages on.**
  `push_site` creates and force-pushes `gh-pages`, and stops there. Nothing
  calls `PUT /repos/{repo}/pages`, so on a repository that has never served a
  site GitHub keeps whatever default it had and the URL in `[book] site` returns
  a 404. The failure is silent in the worst way: `bower push` reports
  `site  gh-pages — 46 files`, `status` reports `site  in sync`, and every
  claim the tool makes is true — the branch really is published — while the
  page a reader visits does not exist.

  Found 5 September 2026 on `folkengine/rust4failures`, the first site branch
  pushed to a repository nobody had configured by hand. GitHub reported
  `build_type: "workflow"` and `source.branch: "main"`: waiting on an Actions
  workflow that was never written, and pointed at the wrong branch regardless.
  `abstecker/hello-playbook` never showed it because its Pages settings were
  switched on manually before Bower ever pushed to it.

  Shape of the fix: when `push_site` creates the branch — the
  `RemoteState::Absent` arm it already distinguishes — follow it with
  `PUT /repos/{repo}/pages` carrying `build_type: "legacy"` and
  `source: {branch: <site_branch>, path: "/"}`. Only on create: a repository
  whose Pages are already configured, perhaps deliberately to a workflow, must
  not be reconfigured underneath its owner. A `409` saying Pages already exist
  is a success, not an error. The dry run should say `pages  will enable` so
  the one step that reaches outside git is visible before it happens.

  **Enabling is not sufficient**, and this is the half that is easy to miss.
  Pages builds on a *push* to its source branch. Bower pushes the branch and
  only then could configure Pages, so the push that would have triggered the
  build has already happened and no build is ever queued: `GET /pages` reports
  the right `build_type` and the right `source.branch`, `builds` is `[]`, and
  the URL 404s indefinitely. Verified 5 September 2026 on
  `folkengine/rust4failures` — correct settings, zero builds, and a live site
  one `POST /repos/{repo}/pages/builds` later. So the create path is: push the
  branch, `PUT /pages`, then `POST /pages/builds` once. Subsequent pushes need
  none of it, because by then a push to `gh-pages` is an event Pages listens
  for.

  Two smaller things belong with it. `bower status` cannot currently tell
  "branch pushed" from "site served", so it says `in sync` about a 404; a
  `GET /repos/{repo}/pages` reading `status` and `source.branch` would let it
  say "site branch pushed, but Pages serves `main`" instead. And this is one more reason the `GitHubForge` gap above keeps costing: the decision (enable
  or leave alone) is pure and testable against `FakeForge`, and only the `gh
  api` call itself is the last inch.

- [ ] **Nothing reports whether a published artifact is current.**
  `bower status` names lock, repo, and site drift. It says nothing about the
  `.pdf` and `.epub` in a book's `assets` directory, so a hand-run
  `bower push --execute` can attach an artifact rendered from an older book.
  Mitigated, not fixed: `make clean` wipes `books/*/published`, and
  `make ship-hello` re-renders both before it previews. The honest fix is an
  `ArtifactDrift` beside `SiteDrift`, digesting the same fingerprint the site
  marker already uses.

- [ ] **The epub cover is an SVG, and no reader has been tested.**
  `publish::compose_cover` always emits SVG, and pandoc embeds it as
  `EPUB/media/file0.svg`. That is legal EPUB 3 and renders in Apple Books and
  Calibre; Kindle and older EPUB 2 readers handle SVG covers unevenly, and none
  has been checked. A raster fallback — rendering the composed SVG to PNG when
  a raster cover is wanted — needs either a rasterizer dependency or a shell out
  to one, which is why it was not done.

- [ ] **A book's cover is outside every drift check.**
  Editing `cover.svg` or `cover.png` changes the epub and the PDF and nothing
  else. `site_fingerprint` covers the HTML render only, and correctly so — but
  it means no command anywhere will tell you the published artifacts no longer
  match the cover on disk. Subsumed by the artifact-drift item above.

- [ ] **Symlinks in a `template/` directory are followed.**
  A book cannot create a symlink — the kernel has no op for it — but
  `materialize::read_dir_recursive` follows one it finds while reading a
  configured `template/`, so a symlinked file would be copied by content into
  every generated repo. Related to `docs/DEFECT_Path_Traversal.md`, and out of
  scope for that fix.

- [ ] **Path case is not normalized.**
  On a case-insensitive filesystem, `SRC/lib.rs` and `src/lib.rs` are one file
  but two `TreeState` keys. Not an escape; a way for two steps to collide
  without anyone noticing.

- [ ] **The site's fingerprint is coarser than the render.**
  `publish::site_fingerprint` hashes the lock plus every chapter's *source*, so
  a change that renders identically — a trailing space, a reworded HTML comment
  — reports the site stale. Deliberate: a false "stale" costs one re-render, a
  false "in sync" serves the wrong book at HTTP 200. Digesting the *rendered*
  markdown instead would be exact, but `status` would have to render, and
  `status` is the one command that touches nothing outside the machine.

## 🤖 Automated review findings

<!-- Promote good ones up to "Tracked debt", delete the rest. -->

_None yet._ The deep automated review pass that normally seeds this section on
first creation was **not run** — it needs a code-review subagent, and this
session is configured not to spawn one unasked. Say the word and I will run it.
