---
type: Playbook
title: Releases and the site branch
description: A book that declares an edition ships its downloads with it, and a book that names a site_branch ships its HTML in the same command — so code and pages cannot drift apart.
tags: [operations, publishing, releases, github-pages]
timestamp: '2026-09-09T00:00:00Z'
---

# The shape

[`bower push`](/commands/push.md) publishes three things in one command: the
generated repository, the rendered site, and the release. That is deliberate —
a site published separately from its code is a site free to drift from it.

# Declaring an edition

```toml
[book]
version = "0.1.0"          # the edition; absent means no releases

[repos.hello-playbook]
assets = "published"       # book-relative; where the epub and PDF are rendered
```

`push` then hangs a GitHub release off `v0.1.0` and attaches every `.pdf` and
`.epub` **directly inside** that directory — not recursively, because the
renderers' own scratch folders live under the same roof.

Re-shipping the same edition **replaces** the files rather than failing. That is
safe precisely because both artifacts are byte-reproducible; see
[the invariants](/architecture/invariants.md).

# Half-configured is an error

`assets` without a `version`, or an `assets` directory holding neither format,
refuses the push and says which. The alternative — pushing quietly with no
downloads — is the failure that looks like success.

# The site branch

A book naming a `site_branch` ships its HTML there in the same command. Pages is
enabled **only if the branch was created on that run**, so a repointed or
manually configured Pages setup is left alone.

`publish --target html` writes two files when any repo declares a `site_branch`:

- `.nojekyll` — empty, and load-bearing. Without it, GitHub Pages runs Jekyll,
  which silently drops `_`-prefixed paths.
- `.bower-site` — the book marker plus `plan-digest: {fingerprint}`, which is
  what lets [`bower status`](/commands/status.md) tell a stale site from an
  unpublished one.

# The order, and why

Repository → site → release. The release is decided locally and published
**last**, after the repository and the site are really there:

> a release pointing at an unfetchable tag is worse than no release.

# The two-target rule

```
make ship-hello           # build, verify, render, and report what a push would do
make ship-hello-execute   # the same, then actually push
```

Two targets rather than one flag, so `--execute` is written down in exactly one
place. The dry run's prerequisites include `epub` and `pdf` on purpose: *a
preview that cannot see the release it would publish is not a preview.*

# Known gap

Nothing yet reports whether a **published** artifact is current — `status`
covers the local site directory, not the remote. It is an open item in
`docs/TECHNICAL_DEBT.md`.

# Citations

[1] `bower/src/push.rs`, `bower/src/publish.rs`, `Makefile`
[2] `docs/EPIC-05_Push.md`, `docs/EPIC-08_Site.md`
