# Forges — a self-hosted remote per book

**Status:** proposed design, not yet implemented
**Date:** 2026-09-09
**Repo:** `ImperialBower/bower`
**Relates to:** `bower-spec.md` Draft 0.2 (§ 3.4 line-anchored source links,
§ 5.3 tags, § 5.5 publishing repos, § 13 M2 editions, § 16 devenv);
`docs/EPIC-05_Push.md` (the `Forge` trait); `docs/EPIC-08_Site.md` (the site
branch); `BACKLOG.md` open question 4 (repo CI, decided yes)

---

## 1. Purpose

Give every book its own forge: a container running a self-hosted git host
that does what GitHub does for a generated repository — hosts it, serves
line-anchored file views at every tag, holds releases, serves the rendered
site, and re-verifies on push — without GitHub in the loop. GitHub stays a
target; it stops being *the* target.

The design rests on one fact Bower already proves: replay is deterministic.
A forge holding generated repositories is therefore a **cache of the book**,
not a source of anything. It can be thrown away and rebuilt from an empty
volume at any time, which is exactly what makes one container per book the
right unit — no shared state, nothing to migrate, and an edition
(`bower-spec.md` § 13 M2) can pin a forge image alongside `bower.lock` and
the tags.

This is a second *target* for `push` and `publish`. Replay does not change.
`bower-core` does not change beyond one template variable (§ 6.1).

## 2. Non-goals

Deliberately out, recorded in `BACKLOG.md` when this ships:

- **Mirroring.** No Forgejo push-mirror from the local forge to GitHub.
  Bower pushes to every declared remote itself; a mirror adds a second clock
  to history Bower already controls.
- **Hosting the book's own source.** The forge holds generated repositories.
  This workspace stays where it is.
- **Reader-facing distribution of the container** — the "book in a box"
  (§ 10). This spec makes it possible; it does not ship it.
- **Multi-tenant or shared forges.** One book, one container. Two books
  wanting one forge is a later decision, not a config option now.
- **Forge kinds beyond `github` and `forgejo`.** The trait admits more; none
  are designed here.

## 3. Forge choice: Forgejo

**Forgejo.** One Go binary, a small OCI image, SQLite is sufficient for one
book, and its REST API is Gitea's — GitHub-shaped enough that every `Forge`
method maps to one endpoint. Its Actions read `.github/workflows` as-is, so
the generated repos' CI (open question 4) runs unchanged. Codeberg runs
Forgejo, so the same `kind` also targets a public, non-GitHub host.

Considered and set aside:

| Option | Why not |
|---|---|
| GitLab CE | Multi-gigabyte footprint for a single-book host. |
| Gogs | Stale; Gitea/Forgejo is its maintained line. |
| soft-serve | No web file view with line anchors — `links.blob` has nothing to point at. |
| Radicle | The sovereignty-aligned answer; its explorer cannot serve the link templates yet. Revisit. |

## 4. Configuration

Forges are named, repos name their remotes per forge. `bower.toml`:

```toml
[forges.github]
kind = "github"                                  # `gh` + `git`, as today

[forges.local]
kind  = "forgejo"
url   = "http://localhost:3000"
token = "env:BOWER_FORGE_LOCAL_TOKEN"             # never a literal

[repos.rust4failures]
site_branch = "gh-pages"
assets = "published"

[repos.rust4failures.remotes]
github = "abstecker/rust4failures"
local  = "ImperialBower/rust4failures"

[repos.rust4failures.links.local]                # declarable; derived when absent
blob = "http://localhost:3000/ImperialBower/rust4failures/src/tag/{tag}/{path}#L{start}-L{end}"
```

Rules, in `config.rs`:

- `github = "o/r"` on a repo remains valid and means `remotes.github = "o/r"`
  over an implicit `[forges.github] kind = "github"`. Every existing
  `bower.toml` parses unchanged.
- A remote naming a forge that is not declared is a config error, by name.
- `token` accepts only `env:NAME`. A literal token is a config error — the
  file is committed.
- `[repos.<r>.links]` with no forge suffix keeps meaning the `github` remote,
  for compatibility. `[repos.<r>.links.<forge>]` is the general form.
- `LinkTemplates` are **derived per forge** from `kind`, `url`, and the
  remote name when not declared. Today's GitHub derivation (`checkout`,
  `fork`, `clone`) generalises; `blob`, `tree`, `commit`, and the new `diff`
  (§ 6.1) get derivations for both kinds:

| Template | `github` | `forgejo` |
|---|---|---|
| `blob` | `{base}/blob/{tag}/{path}#L{start}-L{end}` | `{base}/src/tag/{tag}/{path}#L{start}-L{end}` |
| `tree` | `{base}/tree/{tag}` | `{base}/src/tag/{tag}` |
| `diff` | `{base}/compare/{prev_tag}...{tag}` | `{base}/compare/{prev_tag}...{tag}` |
| `fork` | `{base}/fork` | `{base}/fork` |
| `clone` | `git clone {base}.git` | `git clone {base}.git` |

where `{base}` is `https://github.com/o/r` or `<url>/o/r`. `commit` stays
declarable but is no longer derived (§ 6.1).

## 5. The `Forge` trait: one new implementation

`forge.rs` already has the seam: `probe`, `read_steps_md`, `create`,
`probe_branch`, `read_site_marker`, `push_tree`, `push`, `publish_release`,
`enable_pages`, behind `FakeForge` for the logic and `GitHubForge` for the
last inch. `ForgejoForge` is a second last inch. Nothing in `push.rs`'s
decisions changes.

| Method | `ForgejoForge` |
|---|---|
| `probe` | `GET /api/v1/repos/{o}/{r}`; 404 → absent; empty → `Empty` |
| `read_steps_md` | `GET /api/v1/repos/{o}/{r}/raw/STEPS.md` |
| `create` | `POST /api/v1/orgs/{o}/repos` (or `/user/repos` when `{o}` is the token's user) |
| `probe_branch` / `read_site_marker` | `GET …/branches/{b}`, `GET …/raw/.bower-site?ref={b}` |
| `push` / `push_tree` | `git push --force-with-lease` over HTTP with the token as credential; same helpers as GitHub, `remote_url` becomes per-forge |
| `publish_release` | `POST …/releases` by tag; `POST …/releases/{id}/assets` per file; re-shipping replaces assets, as today |
| `enable_pages` | § 7.3 |

The marker gate — refuse to force-push over a repository that does not
carry Bower's own `STEPS.md` for this book, no override — applies to every
forge identically. It is decided in `push.rs` against the trait, not in
either implementation.

`remote_url(repo)` currently assumes github.com. It becomes a method of the
forge, or a field of a resolved remote; either way it stops being a free
function with a host in it.

**What the local forge buys the test suite.** `GitHubForge` is the
acknowledged untested inch (`BACKLOG.md` § Known gaps) and has cost two
defects. A forge that runs in a container is a forge the tests can talk to:
`bower/tests/forgejo.rs`, `#[ignore]`d in the `slow` lane, stands up the
container and drives `ForgejoForge` through every method against a real
server. The trait's behaviour finally gets exercised end to end somewhere.

## 6. Rendering per forge

Link bases are absolute, so a site rendered for the local forge is not the
site rendered for GitHub. Both are pure folds of the same render plan
against different link tables, and both are byte-reproducible.

- `bower publish --target html --forge <name>` renders against that forge's
  templates. Output goes under `published/site/<forge>/`. No `--forge` means
  the repo's first remote in declaration order.
- The `.bower-site` marker gains a `forge:` line. `status` reports a site
  rendered against one forge and pushed to another as drift.
- Epub and PDF carry links too; they are rendered per forge the same way,
  and a release on forge X attaches the assets rendered for forge X.

### 6.1 The diff link and `{prev_tag}`

`bower-spec.md` § 5.3 links a step's diff as `…/commit/{tag}` and relies on
GitHub resolving a tag there. Forgejo's commit route is documented against
a SHA. Rather than depend on either host's leniency, a step's diff becomes
a **compare between consecutive tags**, which both hosts serve.

The kernel adds `{prev_tag}` to the template vocabulary: the previous step's
tag in the same repo, or the repo's initial tag (`step-000-scaffolding`) for
the first step. It is one field on `PlannedStep`, computed at plan time from
the ordered steps; no I/O, no new error variants. Template substitution
already lives beside the line map and learns one more name.

## 7. The container: `bower forge`

### 7.1 Layout

Per book, `books/<book>/forge/compose.yml`, three services:

| Service | Image | Role |
|---|---|---|
| `forgejo` | Forgejo, SQLite, one named volume | The forge; `:3000` |
| `runner` | `forgejo-runner` | Actions: re-verifies every push (§ 8) |
| `site` | Caddy, one bind mount | Serves the rendered book; `:8080` |

The compose file is a template output, not hand-maintained per book:
`bower forge init` writes it from `bower.toml` (ports, org name), and it is
committed so it is reviewable.

### 7.2 `bower forge up | down | status`

`up` is thin and idempotent:

1. `docker compose up -d`, wait for `/api/healthz`.
2. If no admin exists: `docker exec forgejo forgejo admin user create …`
   with a generated password written to `forge/.env` (gitignored).
3. If the org named by the remote does not exist: create it.
4. If `forge/.env` holds no token: create one scoped to repo, org, and
   package/release write, write it to `forge/.env`.
5. Print the `export` line for `BOWER_FORGE_LOCAL_TOKEN`.

Running `up` twice does nothing the second time. `down` stops the
services; `down --purge` also removes the volume — safe by construction,
because the next `push --forge local --execute` rebuilds every repository
from the book. `status` reports reachability, which repositories exist, and
whether each carries this book's `STEPS.md`.

Bootstrap logic (which of steps 2–4 to run) is decided against probes
behind a small trait, so it is tested with a fake; `docker` and the API are
the last inch, exercised by the `slow` lane.

### 7.3 Pages

Forgejo has no Pages. The `site` sidecar serves a bind-mounted directory.
`ForgejoForge::enable_pages` copies the pushed site tree into that
directory and returns `PagesAction::Served { url }`. The site branch is
still pushed — the marker gate and `status` reason about it exactly as for
GitHub — the sidecar is simply what serves it. The generated README's
"rendered book lives on the `gh-pages` branch" line becomes a template
variable per forge.

## 8. Actions on the local forge

Generated repos already carry a workflow that re-verifies on push. Forgejo
runs `.github/workflows` unchanged, so the badge works on the first push.
The runner needs a container runtime: the host Docker socket is the
pragmatic default; Docker-in-Docker is the isolated one, chosen per
`forge/.env`.

The interesting follow-on: register a second runner labelled `devenv` that
runs jobs inside the repo's own `devenv shell` (`bower-spec.md` § 16). Then
"verified in CI" and "verified in the reader's shell" are the same fact
about the same toolchain, on the reader's own machine. That runner is a
backlog entry, not this spec.

## 9. Tests

Same shape as everything else in the workspace.

- **Config** (`config.rs`): a `github =` key alone still yields a
  `github` remote with today's derived links; `[forges.*]` + `remotes`
  parse; undeclared forge, literal token, and a remote with no forge each
  produce their named error; derived templates for both kinds match the
  § 4 table.
- **Kernel**: `{prev_tag}` is the previous step's tag, the scaffolding tag
  for the first step, and never crosses repos; the lock text carries it.
- **Push goldens** (`push.rs`, `FakeForge`): a two-remote repo produces
  two independent plans; `--forge` narrows to one; the marker gate refuses
  per remote, not per repo.
- **Publish** (`bower/tests/publish.rs`): the same book rendered for two
  forges differs only in link hosts; `status` reports a site marker whose
  forge does not match the push target.
- **Slow lane** (`bower/tests/forgejo.rs`, `#[ignore]`, needs Docker):
  `bower forge up` from nothing; `push --forge local --execute` creates the
  repo, pushes tags, publishes a release with the epub, serves the site;
  `up` again is a no-op; `down --purge` then `push` rebuilds it all with the
  same SHAs.
- **Sample book**: `books/hello-playbook` declares a `local` forge; `make
  forge-hello` is `forge up` + `ship-hello-execute --forge local`.

## 10. Backlog entries this creates

Added to `BACKLOG.md` when this ships:

- **Book in a box** — `bower forge export`: a compose bundle with a
  pre-populated volume so a reader runs one command and gets forge, repos,
  and site offline, every link resolving.
- **A `devenv` runner** (§ 8).
- **Codeberg pages-server** as an alternative to the Caddy sidecar — it
  serves a branch straight from Forgejo, no copy step, at the cost of
  another image.
- **Radicle** as a third `kind` once its explorer can serve line anchors.

## 11. Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Pin the forge image through Nix?** nixpkgs packages Forgejo; devenv can export the container. Then `devenv.lock` pins rustc *and* the forge, and an edition freezes both. Costs a Nix build on `forge up`. |
| 2 | **Default push target when several remotes are declared** — all of them (this spec's lean; `--execute` is already deliberate) or the first, with `--forge all`? |
| 3 | **Does `forge up` belong in `bower`** or in `make` + a script? In `bower` for the idempotence logic and `.env` handling; the trade is a `docker` dependency in the CLI's I/O layer. |
| 4 | **Does Forgejo resolve `commit/<tag>`?** If it does, `commit` can keep a derivation; § 6.1's compare link stands either way. |
