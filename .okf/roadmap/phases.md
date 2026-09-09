---
type: Roadmap
title: The build plan — phases 1 to 6
description: Phases 1 to 4 are built and their EPICs closed; Phase 5, migrating Rust for Failures, is the current in-flight work.
tags: [roadmap, status, phases]
timestamp: '2026-09-09T00:00:00Z'
---

# The plan, and where it stands

| Phase | Contents | Status |
|---|---|---|
| **1 — kernel** | `bower-core` parser, plan, tree fold, region + display engines, error enum; `bower-testkit` alongside, fixtures first. | Built |
| **2 — replay** | `build`, `plan`, [`bower.lock`](/config/bower-lock.md), deterministic commits, tags, trailers, `STEPS.md`. | Built |
| **3 — verify** | The [`expect`](/model/expectation.md) matrix, per-step verification. *"The phase that makes the tool worth having."* | Built |
| **4 — surfaces** | [preprocessor](/architecture/mdbook-bower.md), [`push`](/commands/push.md), [`status`](/commands/status.md), and (beyond the original scope) [`publish`](/commands/publish.md) for html, epub and PDF. | Built |
| **5 — migration** | Stand up *Rust for Failures*, move the DIARY/doc-comment material chapter by chapter, generate the repo for real. | **In flight** |
| **6+ — publishing maturity and authoring bridges** | See [the publishing ladder](/roadmap/publishing-ladder.md) and [authoring bridges](/roadmap/authoring-bridges.md). | Not started |

# The EPICs, all closed

| Doc | Title |
|---|---|
| `EPIC-01_Replay.md` | Replay — book plan to git history |
| `EPIC-02_Verification.md` | Verification — the book eats its own cooking |
| `EPIC-03_Preprocessor.md` | The mdBook preprocessor |
| `EPIC-04_Status.md` | `bower status` — the drift report |
| `EPIC-05_Push.md` | `bower push` — publishing, with a guard rail |
| `EPIC-06_Publish.md` | `bower publish` — one render, several targets |
| `EPIC-07_Pdf.md` | `--target pdf` — Typst, and a PDF that is the same twice |
| `EPIC-08_Site.md` | The site branch — shipping the rendered book |

Two defect documents, both fixed: `DEFECT_Path_Traversal.md` (path traversal via
book-controlled `file=` paths, found in review 2 September 2026, reproduced and
regression-tested) and `DEFECT_Review_Findings.md` (three bugs found by review —
scaffolding deleted at step 1, so every generated repo shipped with no licence
file; `verify --step` broken on multi-repo books; `show=` ignored at render
time). Each fix landed with a failing test first.

# Where Phase 5 actually is

[*Rust for Failures*](/books/rust4failures.md) has **two steps**, both
deliberate failures, and nothing pushed. The next concrete task is the chapter
map over *pkcore*'s `DIARY.md`.

Note that `BACKLOG.md` and the `README` both describe the book as further along
than its lock file does. The lock is generated; trust it.

# Citations

[1] [`bower-spec.md` §11](/references/bower-spec.md), `docs/EPIC-0*.md`, `BACKLOG.md`
