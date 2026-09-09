---
type: CLI Command
title: bower status
description: The drift report between book, lock, built repository, and rendered site — with absence carefully distinguished from drift so it is safe in CI on a fresh checkout.
tags: [command, drift, ci]
timestamp: '2026-09-09T00:00:00Z'
---

# Usage

```
bower --book <DIR> status [--repo <NAME>] [-o|--out <DIR>] [--site <DIR>]
```

`--out` defaults to `out`; `--site` to `<book>/book`.

# The question it answers

What here is out of date? It compares the book against
[`bower.lock`](/config/bower-lock.md), against a previously built repository,
and against a previously rendered site.

# Three axes, each with an explicit "absent" state

| Axis | Variants |
|---|---|
| `LockDrift` | `NeverPlanned` · `InSync` · `Stale { only_in_lock, only_in_book }` |
| `RepoDrift` | `NeverBuilt` · `TagsPacked` · `InSync` · `Stale { missing_tags, unexpected_tags, differing_files, missing_files, unexpected_files }` |
| `SiteDrift` | `NotConfigured` · `NeverPublished` · `InSync { files }` · `Stale { rendered_from, current }` |

Lock comparison is a line-**set** difference, not positional, so a reordering
reports as the lines that moved rather than as everything after the change.

# The output

```
{repo} — {n} steps
  lock      in sync
  repo      never built — run `bower build`
  site      STALE — rendered from {a}, book is now {b}
```

Drift detail is indented 14 spaces, using `-`/`+` for lock lines and
`missing tag` / `extra tag` / `differs` / `missing` / `extra` for the repo. At
most 5 are shown, the rest summarized as `… and N more`.

# Absence is not drift — and this is the load-bearing rule

`has_drift()` is true for **`Stale` only**. `NeverPlanned`, `NeverBuilt`,
`NeverPublished`, `TagsPacked` and `NotConfigured` are absence or admission, not
drift.

That is what makes the command safe to run in CI on a fresh checkout: an
unplanned book and an unbuilt repo report themselves and exit 0. Real drift
prints `something is out of date` on stderr and exits non-zero.

# The site fingerprint covers prose

```
site_fingerprint(book, lock) = digest(lock_text ++ each chapter.path ++ chapter.text)
```

Deliberately **not the lock alone**. The lock omits commit subjects and prose,
so before this was fixed, editing a paragraph or changing a `msg=` left the site
reporting in-sync while serving different text — found live on 2 September 2026.

The asymmetry is stated in the source: a false "stale" costs a re-render; a
false "in sync" serves the wrong book.

# Reading tags without gix

Tags are read as loose refs from `.git/refs/tags` directly, on the reasoning
that *"a drift report that shares a library with the thing it inspects can share
a bug with it."* The cost is the honest `TagsPacked` admission.

# Citations

[1] `bower/src/status.rs`, `bower/tests/status.rs`, `bower/tests/site.rs`
[2] `docs/EPIC-04_Status.md`, `docs/EPIC-08_Site.md`
