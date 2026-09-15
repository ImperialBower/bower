# Technical Debt

> Maintained by the `/backlog` skill. Items tagged 🤖 were proposed by automated
> review — review and edit them; they are suggestions, not facts.
>
> Created 1 September 2026 at `b3def07`. Re-checked 13 September 2026 at
> `d8f24a4`: EPIC-11 closed two 🤖 findings; the other four were re-read against
> the code and still stand. Re-checked 15 September 2026 at `f765897` against
> EPIC-09: PR #8 closed one more of its items (a branch off main's last
> step); the thirteen still open were re-read against the code and stand.
> The same day, the first deep automated review ran over the whole workspace
> and added thirteen 🤖 findings. Two were fixed the same day (a `site_branch`
> that git reads as `--mirror`, reproduced first; and a Pages enable never
> retried); eleven are open.
> Last substantive refresh 5 September 2026, after
> the first real Phase 5 book, which turned up four defects before it served a
> single page — the book-name fallback, a garbled refusal message, a
> `--force-with-lease` that could never push twice, and a site branch that was
> pushed but never served. All four are fixed.
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

- [x] ~~**No stderr snapshots for `compile_fail`.**~~ Closed 12 September 2026
  by EPIC-11. A book records what the compiler said in an `output="check"` or
  `output="verify"` fence; `bower verify` compares its normalized text and
  `--record` writes it (`bower-core/src/capture.rs`, `bower/src/record.rs`).
  Grounding it found that `bower verify` under `make` ignored every tree's
  toolchain pin, because rustup's cargo proxy exports `RUSTUP_TOOLCHAIN` to its
  children; `run` now removes it.

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

- [x] ~~**`bower push` ships a site branch but never turns Pages on.**~~
  Closed 5 September 2026, the same day it was found. `Forge::enable_pages`
  runs after a site branch this run *created*, decides with the pure
  `pages_action`, and reports what it did. The rule it enforces is "never take
  a served site away from its owner": no Pages at all is created, Pages that
  has never built is repointed (the state a real repository was found in),
  Pages already pointed here is only asked to build, and a repository serving
  anything else is left alone and said so. Twelve tests, two of them driven by
  the exact API bodies GitHub returned before and after the manual fix. The
  original write-up follows, because the reasoning is why the shape is what it
  is.

  ~~Original:~~
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

- [ ] **A PR's description renders as a plain fence.**
  The block form's fence is printed as a `markdown` code block, not as a
  styled box like an exercise's detail (`bower/src/render.rs`, `chapter`).
  Known at slice 1's design (EPIC-09); a styled box is cosmetic, not a
  correctness gap.

- [ ] **`status` cannot see a moved branch.**
  Branches are compared by name, as tags already are (EPIC-09 Decision 17); a
  branch ref pointing at the wrong commit reads as in sync until a rebuild.
  "Moved" would need the expected SHA, and that needs a replay, which
  `status` never does.

- [ ] **Remote branches the book dropped are never deleted.**
  `bower push` leaves a branch the plan no longer names on the forge, by
  design — Bower never deletes — but nothing reports it either. A dry run
  that named a remote branch absent from the plan would close this. Cheaper
  since slice 2: `schedule` (`bower/src/schedule.rs`) already receives every
  remote head, so the difference against the plan's branches is data in hand,
  and only the dry-run line is left to write.

- [ ] **The push gate reads `STEPS.md` from the repository's default branch,
  not from `main`, the branch it force-pushes.** `read_steps_md`
  (`bower/src/forge.rs`) trusts whatever the forge currently calls default.
  EPIC-09 slice 2 (Decision 26) closes the gap for a fresh remote — main is
  pushed first and the schedule then sets it as the default branch — but a
  repository whose owner chose another default before Bower ever pushed to it
  is still read there. Reading `?ref=main` explicitly would harden it against
  that case.

- [ ] **`|` in a PR title or branch name breaks the `PULLS.md` table**, as it
  already does `STEPS.md` rows (`bower/src/trailers.rs`, `pulls_md`). Same
  shape as the existing 🤖 finding on `STEPS.md` below; EPIC-14 Phase 0's
  machine-readable twin would close both at once rather than escaping `|` in
  two hand-written table writers.

- [ ] **A book whose steps are all on branches, with no scaffolding, leaves
  HEAD unborn.** `bower build` prints the empty-tree id as HEAD
  (`bower/src/replay.rs`, `run`) rather than refusing or naming the gap. No
  current book does this — every book's step 0 is scaffolding on main — so it
  is a theoretical edge, not yet a defect.

- [ ] **Some single mistakes in a book still produce two errors.**
  A failed branch block is re-applied at its merge, so one bad block reports
  once where it happened and again at the merge that replays it; a refused
  branch step's displays resolve against main, so a rendering error can
  follow a plan error that already named the real problem
  (`bower-core/src/branch.rs`). Each error is true on its own terms, but a
  reader fixing a chapter sees two messages for one mistake.

- [ ] **A PR block silently ignores keys that belong to other blocks.**
  `resolve`'s `pr_block` arm only checks that a fence is present; unlike the
  `output` and play-cell arms beside it, it never refuses `exercise=`,
  `merge=`, or `from=` on a PR block — those keys are parsed and then simply
  never read for that block, rather than raising `OutputConflictingKeys` or
  `PlayCellConflictingKeys`'s equivalent (`bower-core/src/block.rs`).

- [x] ~~**A branch forked from main's last step shows `STEPS.md`/`PULLS.md` as
  deleted in its diff against main.**~~ Closed 14 September 2026 in PR #8
  (`215da7a`, EPIC-09 slice 2 corrigendum item 15). `Replayer::run` now lays
  the generated files into the last main step's tree and into every branch
  step descended from it (`carries_generated`, `bower/src/replay.rs`); every
  existing SHA is unchanged. Test:
  `replay__a_branch_off_the_last_main_step_changes_only_its_own_files`.

- [ ] **Branch names are substituted into link templates unescaped.** `git
  check-ref-format` allows `#` and `)` in a branch name, and `branch_link`
  splices the name straight into the link template with `str::replace`, no
  escaping (`bower/src/render.rs`). A branch named `try/fix)`, say, would
  close the markdown link early.

- [ ] **A branch named like a `<chapter>-end` tag makes `git checkout`
  ambiguous.** `chapter_ends` names its tags `<stem>-end`; `branch_name_problem`
  refuses `main`, `HEAD`, and `step-`, but nothing refuses a branch whose name
  collides with a chapter-end tag a book will also create.

- [x] ~~**`report_push`'s forwarding of branches to `Forge::push` is covered
  only by the compiler.**~~ Closed 14 September 2026, EPIC-09 slice 2.
  `Forge::push` and the bare forwarding it named are gone: `execute`
  (`bower/src/push.rs`) is a testable library function now, and runs a real
  `Schedule` through `FakeForge` in tests
  (`execute__runs_moves_in_order_and_chains_main_leases` and its neighbors);
  `report_push` calls it by its fully qualified path (`bower/src/main.rs`).

- [ ] **No golden pins a straight-line book's SHAs.** Decision 13's promise —
  that a book without branches replays to the SHAs it always did — is
  guarded by structural tests (parents, trailers, lock text) and a one-time
  manual comparison at EPIC-09's own review, not by a committed golden SHA a
  future change could diff against.

- [ ] **A push interrupted while main stands on a stepping stone leaves it
  there, with no put-back.** `execute` (`bower/src/push.rs`) puts main back
  on its head when a *move* fails after a stone, but a signal or a dropped
  connection between `main → <stone>` and `main → <head>` stops the process
  itself, and nothing runs. The remote is left on the stone, which carries no
  `STEPS.md`, so every later `bower push` is refused by the marker gate as
  "not generated by this tool" — true of the commit it reads, and wrong about
  the repository. The fix would name stepping stones in that refusal, or
  detect a stone on the remote (a step tag of this book at main's SHA) and
  offer the head. Found in EPIC-09 slice 2's final review.

- [ ] **A book whose steps are all on branches never pushes main.** The
  kernel admits a plan with branch steps and no main step
  (`replay::last_main_seq` is `None`), and `schedule` (`bower/src/schedule.rs`)
  then has no head to move main to: such a book's push never pushes main nor
  sets the default branch — only the branches, their PRs, and the tags. On a
  fresh remote the forge picks the default itself (likely the first branch
  pushed), and since no tree carries `STEPS.md` without a main step
  (`replay::final_blobs`), the next push is refused by the marker gate. Checked in EPIC-09 slice 2's final review against a two-step,
  one-branch plan; no current book does this (every book's first step is on
  main), and it pairs with the unborn-HEAD item above.

## 🤖 Automated review findings

<!-- Promote good ones up to "Tracked debt", delete the rest. -->

### Deep review, 15 September 2026

The whole workspace at `f765897`, split four ways (kernel and testkit; push,
forge, and schedule; publish and render; replay, verify, and status), with
this file as the do-not-repeat list and the crates' own contracts as the
standard. Fourteen findings came back and every one was re-checked against the
cited lines. Thirteen are listed; one duplicate was folded into its parent, and
one reviewer's claim (a symlink cycle overflows the stack) was corrected: the
OS's `ELOOP` limit cuts the recursion off. The first item was reproduced
against a local bare repository, and is fixed; so is the second. The other
eleven are open.

- [x] 🤖 ~~**`site_branch` reaches `git push` as a flag, and `--mirror` deletes
  the remote's refs.**~~ Closed 15 September 2026, test first, with two
  guards. `plan_push` refuses a `site_branch` that `branch_name_problem`
  refuses, beside the existing clash check and before any forge call
  (`site_branch_problem`, `bower/src/push.rs`). And `push_tree` names the
  remote branch only in an explicit `HEAD:refs/heads/<branch>` refspec,
  staged on a fixed local branch (`site_push_args`, `bower/src/forge.rs`). The
  refspec was re-run against a local bare repository: `gh-pages` is created and
  replaced as before, and `--mirror` deletes nothing. Tests:
  `plan__a_site_branch_git_would_read_as_an_option_blocks`,
  `site_push_args__name_the_branch_only_inside_a_refspec`. As built, the name
  is checked when a push is planned, not when `bower.toml` loads: only `push`
  uses it, and `build` should not refuse a book over a branch it never
  touches. The original finding follows.

  `[repos.<name>] site_branch` is never validated
  (`bower/src/config.rs:64`); book branches go through `branch_name_problem`,
  which refuses a leading `-`, but the site branch does not. `git init -q -b
  --mirror` accepts the name, so `push_tree` runs `git push --force <url>
  --mirror` (`bower/src/forge.rs:828`). Reproduced 15 September 2026: every
  branch and every tag on the remote was deleted. Only the checked-out branch
  survived, because the remote refused to delete it (GitHub refuses the same
  for the default branch). On GitHub, deleting an open PR's head branch closes
  the PR. `probe_branch` finds `--mirror` absent, so the gate waves it through,
  and `site_branch_clash` (`bower/src/push.rs:699`) checks only `main` and the
  book's own branches. Suggested: run `site_branch` through
  `branch_name_problem` when `bower.toml` loads, and push an explicit refspec
  (`HEAD:refs/heads/<branch>`) so no ref name is ever read as an option.
  (`bower/src/forge.rs:828`)
- [x] 🤖 ~~**A failed `enable_pages` is never retried.**~~ Closed 15 September
  2026, test first. After the gates pass, the plan reads Pages
  (`Forge::pages`, read-only; skipped when the repository does not exist yet)
  and decides with a pure `planned_pages` (`bower/src/push.rs`). A branch this
  run creates gets whatever `pages_action` decides, as before. A branch that
  already exists gets `enable_pages` only while Pages is not serving it
  (`Create` or `Repoint`). A branch Pages already serves gets no call, so a
  steady-state push does not request a second build. `SitePush::pages`
  carries the decision; `publish_site` calls `enable_pages` when it is `Some`,
  and the dry run prints it. Tests:
  `planned_pages__an_existing_branch_is_retried_only_while_pages_is_not_serving_it`,
  `planned_pages__a_new_branch_gets_whatever_pages_action_decides`,
  `plan__an_existing_site_branch_whose_pages_never_took_plans_to_enable_them`,
  `plan__a_site_branch_pages_already_serves_plans_no_pages_call`. Not yet run
  against real GitHub: `GitHubForge::pages` is the existing `pages_state` read.
  The original finding follows.

  `SitePush::create` comes
  from the probe *before* the push (`bower/src/push.rs:775`), and
  `publish_site` calls `enable_pages` only `if s.create`
  (`bower/src/main.rs:817`). If the branch push succeeds and the `gh api` call
  after it fails, the next push finds the branch present with a matching
  marker. `create` is then `false`, Pages is never enabled, and the dry run's
  `pages` line says nothing. That is the closed "site branch pushed but never
  served" defect again, reached through a partial failure. Suggested: decide
  from Pages' own state. Call `enable_pages` on every site push and let
  `pages_action`, which already leaves a served site alone, choose.
  (`bower/src/main.rs:817`)
- [ ] 🤖 **`verify --record` rewrites chapters in place, one at a time.**
  `write_chapters` runs `std::fs::write` on each changed chapter
  (`bower/src/record.rs:86`): truncate, then write. If it is killed partway,
  the author's chapter is left cut short. If one chapter fails in a
  multi-chapter run, the earlier ones are already rewritten and `bower.lock`
  (written after, `bower/src/main.rs:290`) is not. This is the only code path
  that edits an author's source, and no test covers a failed write. Suggested:
  write each chapter to a sibling temp file and `rename` it into place, and
  add a test where the second of two chapters cannot be written.
  (`bower/src/record.rs:86`)
- [ ] 🤖 **`verify` has no timeout.** `run` blocks on `read_to_end` and then
  `wait` (`bower/src/verify.rs:591`). A step whose test loops or deadlocks
  hangs `bower verify` forever, and CI's job timeout never names the step.
  Suggested: poll `try_wait` against a deadline (configurable per book), kill
  the child, and report a distinct timed-out verdict naming the step.
  (`bower/src/verify.rs:591`)
- [ ] 🤖 **Two regions with one name in one file: the second can never be
  edited.** `apply_region` finds the *first* `begin`/`end` pair for a name
  (`bower-core/src/tree.rs:247`). `nested_region` catches one region inside
  another but not two same-named siblings, so every `op="region"` rewrites
  the first and the second goes stale with no error. Suggested: refuse a
  repeated region name in one file, at the block that wrote it, the way
  `RegionNested` is refused. (`bower-core/src/tree.rs:247`)
- [ ] 🤖 **A display span whose text appears twice in a file links to the
  first copy.** `find_window` returns the first match
  (`bower-core/src/display.rs:241`), so a short span such as `Ok(())` or
  `todo!()` anchors its footer and line links at an earlier occurrence. The
  `line_map_is_exact` property cannot see it, because the text at the wrong
  range is identical. Suggested: search from the block's own position in the
  file, or refuse a span that matches more than once.
  (`bower-core/src/display.rs:241`)
- [ ] 🤖 **A malformed `cover.svg` publishes a blank title band.** `nest_svg`
  returns `""` when it cannot find `<svg` or the end of its start tag
  (`bower/src/publish.rs:281`). `compose_cover` and `load_cover` pass that
  along as `Ok`, so the epub and PDF ship with the title missing and no
  warning. Suggested: return a `Result` and refuse the cover with a named
  `PublishError`. (`bower/src/publish.rs:281`)
- [ ] 🤖 **A block-form exercise ignores `branch=`, `from=`, `merge=`, and
  `pr=`.** The `exercise_block` arm of `resolve` checks only for a fence
  (`bower-core/src/block.rs:373`); the play-cell arm above it refuses line
  keys with `carries_line_keys`. A stray `merge="…"` on an exercise plans
  cleanly and is never read. The same family as the tracked "PR block
  silently ignores keys" item; fix both together. Suggested: refuse line keys
  in the exercise arm, as the play-cell arm does. (`bower-core/src/block.rs:373`)
- [ ] 🤖 **`check_fonts` ignores `typst fonts`' exit status.** Every other
  tool call in `publish.rs` checks `status.success()`; this one reads stdout
  regardless (`bower/src/publish.rs:1161`). A broken `typst` gives empty
  output, and the error then says a font is not installed. Suggested: return
  `PublishError::Failed { what: "typst fonts", … }` when the command fails.
  (`bower/src/publish.rs:1161`)
- [ ] 🤖 **The render-asset walk follows symlinks under `src/`.**
  `collect_assets_into` recurses on `path.is_dir()`
  (`bower/src/publish.rs:709`), which follows links. A symlink out of `src/`
  pulls outside files into the render directory. A cycle is cut off only by
  the OS's symlink limit (`ELOOP`), after copying its assets dozens of times.
  The same shape as the tracked `template/` symlink item, in a second walker.
  Suggested: skip symlinks via `entry.file_type()`, and share one walker with
  `materialize::read_dir_recursive`. (`bower/src/publish.rs:709`)
- [ ] 🤖 **`report_push`'s sequencing is untested.** The order it enforces
  (create the repository, `execute` the schedule, the site, Pages, then the
  release) and the rule that a failure stops what follows live in private
  functions in `bower/src/main.rs:858`, a file with no test module, always
  handed a real `GitHubForge`. Unlike `execute`, nothing drives it with
  `FakeForge`. Suggested: move it into `push.rs` behind `&dyn Forge` and
  test the order and each short-circuit. (`bower/src/main.rs:858`)
- [ ] 🤖 **`ConflictingRepoInStep` has no fixture and no test.** It is the one
  `BowerError` variant nothing constructs outside its call site
  (`bower-core/src/step.rs:115`), and `corpus.rs` does not require every
  variant to have a broken fixture. Suggested: add a two-repo `broken()`
  fixture, and have the corpus fail when a variant has none.
  (`bower-core/src/step.rs:115`)
- [ ] 🤖 **`push_tree` reports a file count as `PushOutcome::tags`.**
  (`bower/src/forge.rs:837`). `FakeForge::push_tree` returns `tags: 0`, so
  nothing notices, and the only reader (`bower/src/main.rs:809`) knows the
  trick. Suggested: give the site push its own `files` field.
  (`bower/src/forge.rs:837`)

### Grounding EPIC-11 to EPIC-16, 10 September 2026

Found while grounding EPIC-11 to EPIC-16 against the code.
Each was checked against the cited lines. Each EPIC that owns a fix says so.

- [ ] 🤖 **`test_fail` accepts any failing test.** A step is upheld when the
  verify command exits non-zero (`bower/src/verify.rs:261-270`). A broken test
  helper upholds an exercise, and a solution that deletes the failing test
  passes. Suggested: name the tests that must fail. EPIC-13 Phase 2.
  (`bower/src/verify.rs:261`)
- [x] 🤖 ~~**Test output is thrown away.**~~ Closed 12 September 2026 by
  EPIC-11 Phase 0: `Outcome::output` now holds both streams, interleaved in
  the order they were written (`bower/src/verify.rs:48-55`).
- [x] 🤖 ~~**A CI comment promises a check that does not exist.**~~ Closed
  12 September 2026 by EPIC-11: `make failures` in `slow.yml` now verifies the
  recorded `output=` blocks, so the comment at `.github/workflows/slow.yml:130`
  is true as written. Since 14 September 2026 the promise rests on
  `hello-playbook`'s recorded outputs, which `make slow` verifies; CI no
  longer builds *Rust for Failures*.
- [ ] 🤖 **A shipped release can be overwritten.** `release upload --clobber`
  (`bower/src/forge.rs:707-716`) replaces the 0.1.0 PDF with a changed book if
  `[book] version` is not bumped. `release create` has no `--target`
  (`bower/src/forge.rs:718-728`), so GitHub, not Bower, picks the tagged commit.
  And `push --force --tags` (`bower/src/forge.rs:675`) moves every tag.
  Suggested: releases only gain files; name each tag pushed. EPIC-16 Phases 2–3.
  (`bower/src/forge.rs:707`)
- [ ] 🤖 **A `|` in a commit subject breaks its `STEPS.md` row.** The table
  cells are not escaped (`bower/src/trailers.rs:97-103`). Suggested: escape
  `|`. EPIC-14 Phase 0 adds a machine-readable twin instead of parsing the
  table. (`bower/src/trailers.rs:97`)
- [ ] 🤖 **The PDF has no step anchors.** The raw `<a id>` from
  `bower/src/render.rs:72` is dropped by pandoc's Typst writer. Nothing links
  to one yet, so nothing breaks; the first link would fail with "label does
  not exist". Suggested: emit a pandoc span `[]{#step-id}`. EPIC-12 Phase 3.
  (`bower/src/render.rs:72`)
