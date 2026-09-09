---
type: Reference
title: bower-spec.md — the design specification
description: Draft 0.2, 31 August 2026 — the 850-line document this whole bundle distills, and still the authority on anything not yet built.
resource: https://github.com/ImperialBower/bower/blob/main/bower-spec.md
tags: [reference, spec, source-document]
timestamp: '2026-08-31T00:00:00Z'
---

# What it is

`bower-spec.md` at the repository root. **Draft 0.2, 31 August 2026.** It is the
design document the implementation was built from, and it remains the authority
on the parts that are still design rather than code.

Its closing note records the state at drafting: *"Phase 1 is built — 73 tests,
clippy-pedantic clean, spec §9's rank saga as an executable fixture, §15 play
cells bound in the plan, full state coverage."* Phases 2 to 4 have shipped since.

# Section map

| § | Title | Distilled in |
|---|---|---|
| 0–1 | Summary; the problem, precisely | [What Bower is](/overview.md) |
| 2 | Core model — vocabulary and the pipeline | [The pipeline](/architecture/pipeline.md), [the domain model](/model/index.md) |
| 3 | Annotation format, assembly mechanisms, display markers, scaffolding | [The directive](/model/directive.md), [assembly](/model/assembly.md), [display markers](/model/display-markers.md) |
| 4 | Ordering | [Ordering](/model/ordering.md) |
| 5 | Replay — plan to git history | [bower build](/commands/build.md) |
| 6 | Verification | [Expectation](/model/expectation.md), [bower verify](/commands/verify.md) |
| 7 | Configuration | [bower.toml](/config/bower-toml.md) |
| 8 | Crate layout | [Architecture](/architecture/index.md) |
| 9 | Worked example, validated against real pkcore material | — |
| 10 | What this deliberately does not do | [The invariants](/architecture/invariants.md) |
| 11 | Build plan | [Phases](/roadmap/phases.md) |
| 12 | Open questions, with decisions and dates | [Decisions](/decisions/index.md) |
| 13 | Toward a publishing system | [The publishing ladder](/roadmap/publishing-ladder.md) |
| 14 | Authoring bridges | [Authoring bridges](/roadmap/authoring-bridges.md) |
| 15 | The notebook target | [The notebook target](/roadmap/notebook-target.md) |
| 16 | Reader environments — devenv | [Reader environments](/roadmap/reader-environments.md) |

# How to read it against this bundle

Where the spec and the code disagree, **the code wins** — the spec is a draft
that predates the implementation, and several of its details were settled
differently in practice. Two known examples:

- §3.4 shows the elision comment as `// ⋯ 14 lines elided`. The implementation
  emits ASCII `...` and pins it with a test.
- §7's CLI sketch is close but not exact; see
  [commands](/commands/index.md) for the real flags.

For anything **not yet built**, the spec is the only source and should be
trusted.

# Citations

[1] `bower-spec.md`, drafted against pkcore @ HEAD 2026-08-31
