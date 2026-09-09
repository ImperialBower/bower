---
type: Reference
title: Getting started — orienting in this repository
description: The shortest path to a working mental model of Bower, and the daily loop for changing it.
tags: [getting-started, orientation]
timestamp: '2026-09-09T00:00:00Z'
---

# The one-sentence model

Annotated code blocks in a book's markdown are the source of truth; Bower
replays them into a real git repository, one commit per teaching step, and links
page and code to each other in both directions. Start at
[what Bower is](/overview.md).

# The four things worth understanding first

1. **[The domain kernel](/architecture/domain-kernel.md).** `bower-core` has zero
   dependencies and does no I/O. Almost every design oddity elsewhere follows
   from protecting that.
2. **[Determinism](/architecture/invariants.md).** Same book in, byte-identical
   SHAs out. This is why timestamps are synthetic, why `TreeState` is a
   `BTreeMap`, and why replay always starts from an empty tree.
3. **[Expectations](/model/expectation.md).** A step can declare that it *should*
   fail, and the tool checks. This is the whole point of the project.
4. **[Absence is not drift](/commands/status.md).** An unplanned book and an
   unbuilt repo report themselves and exit 0. Only staleness is failure.

# The daily loop

```
make ayce                 # the gate; also the default target
make help                 # every target, self-documented
make slow                 # the tool-dependent lanes
```

See [make ayce](/operations/make-ayce.md) for what is folded in where, and
[CI lanes](/operations/ci-lanes.md) for what runs on a PR versus on `main`.

# Driving a book by hand

```
cargo run -p bower -- --book books/hello-playbook plan
cargo run -p bower -- --book books/hello-playbook build  -o /tmp/hello-playbook
cargo run -p bower -- --book books/hello-playbook verify
cargo run -p bower -- --book books/hello-playbook status -o /tmp/hello-playbook
cargo run -p bower -- --book books/hello-playbook publish --target epub -o published
```

# The trap that catches everyone once

Editing chapter prose shifts the line numbers that
[`bower.lock`](/config/bower-lock.md) records as step anchors, so `bower status`
reports the lock `STALE` and `bower/tests/status.rs` fails. That is the drift
report working. Run [`bower plan`](/commands/plan.md) — or just `make build`,
which now does it for both books — and commit the lock diff.

# Where to look when

| You want | Go to |
|---|---|
| The vocabulary | [The domain model](/model/index.md) |
| How a command behaves | [Commands](/commands/index.md) |
| Why a component is shaped that way | The matching `docs/EPIC-*.md`; see [design docs](/references/design-docs.md) |
| What is decided vs. still open | [Decisions](/decisions/index.md), [roadmap](/roadmap/index.md) |
| What is broken or missing | [Open work](/roadmap/open-work.md) |
