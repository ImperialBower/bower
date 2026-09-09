---
type: Decision
title: Generated repositories carry their own CI
description: Decided 2026-09-04 — every generated repo ships GitHub Actions that re-verify on push, so each published step carries a public green badge.
tags: [decision, ci, generated-repos]
timestamp: '2026-09-04T00:00:00Z'
---

# The question

Should generated repositories carry GitHub Actions that re-verify on push, as a
public badge that every step passes?

# The decision

**Yes.** Decided 4 September 2026.

# Why it matters

A generated repository is a read-only build artifact, and the README banner says
so. But a reader who clones a step still wants to know the code they just
checked out actually works. A workflow in the repo turns
[`bower verify`](/commands/verify.md)'s private claim into a public one.

The workflow itself is ordinary book content: it arrives through the step-0
`template` directory, or as an annotated block like anything else. The
[sample book](/books/hello-playbook.md) teaches exactly this in its own
`ch06-ci.md`, and its `step-019-ci-workflow` writes the file — the book that
generates repositories with CI is itself a book about adding CI.

# Citations

[1] [`bower-spec.md` §12 Q4, §3.6](/references/bower-spec.md)
