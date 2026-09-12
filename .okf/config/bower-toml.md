---
type: Configuration
title: bower.toml
description: The book's configuration at its root — epoch, identity, and per-repo remotes, templates, commands and link templates. Unknown fields are a load-time error.
tags: [config, toml]
timestamp: '2026-09-12T00:00:00Z'
---

# Where it lives

At the book root, named by the global `--book` flag. `BookConfig::load` runs
before subcommand dispatch, so a malformed file fails every command identically.

Every config struct carries `#[serde(deny_unknown_fields)]` — a typo is an error
at load time, not a setting silently ignored.

# The schema

| Table | Key | Notes |
|---|---|---|
| `[book]` | `epoch` | **Required.** A TOML datetime, re-parsed as RFC3339. Pins commit SHAs *and* the PDF. |
| | `site` | Optional. Enables the `Book-Url` commit trailer. |
| | `version` | Optional. The edition. A free string — deliberately **not** semver. |
| `[identity]` | `name`, `email` | Both required. Used for author *and* committer. |
| `[repos.<name>]` | `github` | Optional. `<owner>/<repo>`; its absence means the book does not publish that repo. |
| | `site_branch` | Where the rendered book ships, e.g. `gh-pages`. |
| | `assets` | Book-relative directory holding release downloads. |
| | `template` | Book-relative step-0 scaffolding. |
| | `check`, `verify` | The commands [`bower verify`](/commands/verify.md) runs. |
| | `toolchain` | Optional. The rustup toolchain for a step whose tree pins none. A tree's own `rust-toolchain.toml` wins. |
| | `keep_region_markers` | Bool, default false. **The only key that crosses into the kernel.** |
| `[repos.<name>.links]` | `blob`, `tree`, `commit`, `fork`, `error_code` | URL templates. |

# Derived, not declared

Several values are computed rather than configured, which is why they cannot
drift from what the tool actually does:

- `checkout` = `git checkout {tag}` (only when `github` is set)
- `clone` = `git clone https://github.com/{github}.git`
- `fork` = the declared value, else `https://github.com/{github}/fork`
- `links.error_code` = the declared value, else
  `https://doc.rust-lang.org/error_codes/{code}.html` when `check`'s first word
  is `cargo` — the link each error code gets in a
  [captured output](/model/captured-output.md)'s caption
- `links.check` / `links.verify` fall back to `DEFAULT_CHECK` (`cargo check`)
  and `DEFAULT_VERIFY` (`cargo test`) — **the same defaults the verifier uses**,
  so an [exercise](/model/exercise.md) box can never print a command
  [`bower verify`](/commands/verify.md) would not run.

# The kernel boundary

`BookConfig::catalog()` is the only crossing into
[`bower-core`](/architecture/crate-bower-core.md). It projects `RepoName` and
`RepoSpec { keep_region_markers }` and drops everything else — remotes,
identities, commands, links. A test named
`config__catalog_carries_only_kernel_settings` pins that by equality on a
one-field struct. See [the domain kernel pattern](/architecture/domain-kernel.md).

# A real example

`books/hello-playbook/bower.toml`:

```toml
[book]
epoch = 2026-09-01T00:00:00Z
site = "https://abstecker.github.io/hello-playbook"
version = "0.1.0"

[identity]
name = "ImperialBower Bower"
email = "bower@imperialbower.example"

[repos.hello-playbook]
github = "abstecker/hello-playbook"
site_branch = "gh-pages"
assets = "published"
template = "template"
keep_region_markers = false
check = "cargo check"
verify = "cargo test"

[repos.hello-playbook.links]
blob = "https://github.com/abstecker/hello-playbook/blob/{tag}/{path}#L{start}-L{end}"
tree = "https://github.com/abstecker/hello-playbook/tree/{tag}"
commit = "https://github.com/abstecker/hello-playbook/commit/{tag}"
```

The `blob` template is why the forge is not an assumption. Point it at Codeberg,
sourcehut, or a self-hosted forge and every link in the book follows.

# Citations

[1] `bower/src/config.rs`, `books/hello-playbook/bower.toml`
[2] [`bower-spec.md` §3.4, §7](/references/bower-spec.md)
