---
type: Book
title: Rust for Failures
description: The real book — "How controlled failing is the way to build solid systems" — and the Phase 5 migration that is Bower's current in-flight work.
resource: https://github.com/ImperialBower/bower/tree/main/books/rust4failures
tags: [book, in-flight, phase-5]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

*Rust for Failures — How controlled failing is the way to build solid systems*,
by folkengine. This is the book Bower was built to produce, and the reason
[expectations](/model/expectation.md) exist at all: a book about controlled
failing whose controlled failures are executable claims.

# Current state, as of 2026-09-09

**Two steps**, both of them failures on purpose — which is fitting, and also
means the book is very early:

```
001 kata          expect=compile_fail   anchor=src/ch01-local_development.md:95
002 kata-compiles expect=test_fail      anchor=src/ch01-local_development.md:216
```

`SUMMARY.md` lists five entries — `ch00-introduction.md`,
`ch01-local_development.md`, `ch02-cicd.md`, and two `REJECTED_*` chapters still
in the summary. `ch03-rank.md` exists on disk but is commented out of the
summary. Nothing has been pushed to GitHub yet.

> **A stale claim to watch for.** `BACKLOG.md` says chapter 1 is "written and
> green: five steps", and the `README` describes the book as "one chapter so
> far". The lock says two steps. Trust `books/rust4failures/bower.lock` — it is
> generated — and treat the prose as out of date.

# Configuration

`epoch = 2026-09-05T00:00:00Z`, `version = "0.1.1"`, publishing both halves to
`abstecker/rust4failures` — code on the default branch, the rendered book on
`gh-pages`. It has no cover files yet, unlike
[hello-playbook](/books/hello-playbook.md).

# Why it lives in this workspace

An earlier decision put each book's source in its own repository. It was
reversed after being tried — see
[one workspace for every book](/decisions/one-workspace-for-books.md). Every
book's source now lives here under `books/`, so a change to the kernel and the
chapter that exercises it are one commit.

# The Makefile targets

```
make failures                # build, verify, render, and report drift
make failures-epub / -pdf    # the download formats
make ship-failures           # report what a push would do
make ship-failures-execute   # the same, then actually push
```

# What is next

Phase 5 migration is the project's in-flight work: moving the *pkcore*
`DIARY.md` and doc-comment material into chapters, one at a time. The next
concrete task on the backlog is the chapter map over that diary. See
[the roadmap](/roadmap/phases.md).

# Citations

[1] `books/rust4failures/bower.lock`, `books/rust4failures/src/SUMMARY.md`
[2] [`bower-spec.md` §11 Phase 5](/references/bower-spec.md), `BACKLOG.md`
