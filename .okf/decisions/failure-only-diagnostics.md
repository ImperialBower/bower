---
type: Decision
title: compile_fail asserts failure, not a diagnostic snapshot
description: Decided 2026-09-01 (EPIC-02) — verify asserts the check command fails and captures stderr for the report, but matches nothing against a stored pattern.
tags: [decision, verification, testing]
timestamp: '2026-09-01T00:00:00Z'
---

# The question

Should a `compile_fail` [expectation](/model/expectation.md) also match the
compiler's stderr against a stored pattern, trybuild-style?

# The decision

**Failure-only.** Recorded in EPIC-02, 1 September 2026.

[`bower verify`](/commands/verify.md) asserts that the check command fails, and
captures its stderr for the failure report. It matches nothing.

# Why

Snapshots remain a plausible opt-in — nothing in the verifier's shape prevents
adding them later — but **no book has yet needed the stronger claim.** The
decision is explicitly reversible, and was made on the grounds of demand rather
than principle.

# The tension it leaves open

The spec's own argument for verification is that "the day a new rustc changes
the diagnostic, the build tells you before a reader does". Failure-only
verification catches a step that *stops failing*; it does not catch a step that
fails for a **different reason** than the chapter's prose describes.

That gap is acknowledged, and the reader-side answer is a different lever
entirely: pinning the toolchain per edition, so a `compile_fail` step keeps
failing *for the stated reason* years later. See
[reader environments](/roadmap/reader-environments.md).

# Citations

[1] [`bower-spec.md` §12 Q3, §16.1](/references/bower-spec.md), `docs/EPIC-02_Verification.md`
