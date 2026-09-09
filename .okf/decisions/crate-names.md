---
type: Decision
title: The crates.io names are free — and unreserved
description: Checked 2026-09-02 — bower, bower-core, bower-testkit, bower-cli, imperial-bower and mdbook-bower all return 404. No rename is needed; nothing is reserved.
tags: [decision, naming, crates-io]
timestamp: '2026-09-02T00:00:00Z'
---

# The concern

`bower` was the name of a long-dead JavaScript package manager. The spec flagged
it as a collision worth checking before publishing, with `bower-cli` as a
possible fallback.

# What was found

Checked 2 September 2026: **all free.** `bower`, `bower-core`, `bower-testkit`,
`bower-cli`, `imperial-bower` and `mdbook-bower` all return 404 from the
crates.io API. The JavaScript package manager never occupied this registry.

No rename is needed and the fallbacks are unnecessary.

# The caveat that matters

**Names are not reserved by this check.** Anyone could take them first.
Reserving them is a separate, deliberate act of publishing — one nobody has
performed.

The workspace version is `0.1.0` and nothing has been published, so this
remains a live exposure rather than a closed question.

# Inside the organization

The binary is `bower` regardless of what happens on crates.io.

# Citations

[1] [`bower-spec.md` §0, §12 Q5](/references/bower-spec.md)
