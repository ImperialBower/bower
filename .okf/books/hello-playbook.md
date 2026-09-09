---
type: Book
title: Hello, Playbook
description: The sample book and the test fixture — 20 steps that teach a Rust project foundation while building their own repository.
resource: https://github.com/ImperialBower/bower/tree/main/books/hello-playbook
tags: [book, fixture, sample]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

> A Rust project foundation, taught as a book that builds its own repository.

It has two jobs, and the second is the load-bearing one:

1. A demonstration book, small enough to read end to end.
2. **The fixture** that `bower/tests/*` and `bower-testkit/tests/sample_book.rs`
   drive. The corpus proves the kernel handles constructed books; this proves it
   handles one a reader would actually read.

# Chapters

| File | Title |
|---|---|
| `ch01-a-repo-that-builds.md` | A repo that builds |
| `ch02-the-gate.md` | The gate |
| `ch03-lints-and-format.md` | Lints and format |
| `ch04-tests-and-failing-on-purpose.md` | Tests, and failing on purpose |
| `ch05-supply-chain.md` | Supply chain |
| `ch06-ci.md` | CI |
| `appendix-credits.md` | Appendix: credits |

# 20 steps, and the three that are not green

`step-001-cargo-init` through `step-020-drop-scratch`. Three carry something
other than `expect=pass`:

- `011 test-that-fails` — `test_fail`, and it carries the corpus's only
  lock-visible [exercise](/model/exercise.md)
  (`form=key answer=test-that-passes task="Make the test pass"`).
- `013 wont-compile` — `compile_fail`.

Between them they exercise the whole [expectation](/model/expectation.md)
matrix, which is why this book — not a synthetic fixture — is what
`bower/tests/verification.rs` sweeps.

# One deliberate setting worth knowing

```toml
[output.html.playground]
runnable = false
```

The mdBook playground is switched off on purpose. A `compile_fail` step is
*supposed* to break, and the playground would report a different error than the
book's — teaching the reader the wrong lesson about the right failure.

# Configuration and assets

`epoch = 2026-09-01T00:00:00Z`, `version = "0.1.0"`, publishing to
`abstecker/hello-playbook` with `site_branch = "gh-pages"` and
`assets = "published"`. It ships `cover.svg` + `cover.png`, a `template.typ`,
a step-0 `template/` directory, and `theme/step-meta.css`.

# The Makefile targets

```
make book                 # render HTML
make epub / make pdf      # the download formats
make ship-hello           # build, verify, render, and report what a push would do
make ship-hello-execute   # the same, then actually push
```

# Citations

[1] `books/hello-playbook/`, `bower-testkit/tests/sample_book.rs`
[2] `Makefile`
