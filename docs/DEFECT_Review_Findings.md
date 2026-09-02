# DEFECT: three bugs found by review

**Found** 2 September 2026 by an automated review of `main`. The review hit a
session limit and died partway, having confirmed one finding and flagged
several candidates. Each was verified by hand against the real binary before
being fixed, and each fix landed with a failing test first.

---

## 1. The scaffolding was deleted at step 1 (worst)

**What happened.** `Replayer::run` committed the `template/` directory as step 0,
then built every subsequent step's tree from `blobs_of(&step.tree)` alone. A
step's tree *replaces* the tree, so step 1 deleted everything step 0 had added.

Observed in a real build before the fix:

```
step 0 (scaffolding): .gitignore CODE_OF_CONDUCT.md CONTRIBUTING.md
                      LICENSE-APACHE LICENSE-GPLv3 LICENSE-MIT README.md SECURITY.md
step 1:               Cargo.toml src/main.rs
```

**Why it matters.** Every generated repository shipped with **no licence file**,
no `README.md` — the one saying "this repo is generated, do not open PRs" — and
no `.gitignore`. EPIC-01 argued the book "is not honest without a licence"; the
commit that added them was immediately undone.

**Why nothing caught it.** Three ways:

- `bower verify` *did* overlay the template, so verify and build disagreed about
  what a step's tree is, and only verify was right.
- `final_blobs` omitted the scaffolding too, so `bower status` compared against
  the same wrong set and reported `in sync` on a repo with no licence. The drift
  detector agreed with the bug.
- `final_worktree_is_the_kernels_tree_plus_steps_md` asserted the worktree was
  exactly the kernel's tree plus `STEPS.md` — it *encoded* the bug as expected
  behaviour.

**The fix.** `replay::scaffolding()` is now one public function, and `build`,
`verify`, `status`, and `push` all call it. Every step's tree is the scaffolding
with the step's own files laid over it. `final_blobs` takes the scaffolding as an
argument, so `status` and `push` cannot drift from what `build` writes.
`the_scaffolding_survives_every_step` pins it.

---

## 2. `verify --step` failed on a book with two repos

**What happened.** `run_verify` looped over every repo and passed `--step` to
each. `Verifier::run` rejects a step id absent from *that* repo's plan, so a
valid `--step alpha-only` failed as soon as the loop reached `beta`.

**Why nothing caught it.** No book fed two repos. This was listed in
`docs/TECHNICAL_DEBT.md` as "multi-repo books are untested" — and it was hiding
a bug, not merely an absence of proof.

**The fix.** Repos that do not hold the named step are skipped, and a step no
repo holds is still an error — `a_step_in_no_repo_is_still_an_error` is the
counterweight that stops a typo becoming a silent success.

---

## 3. The `show=` key was ignored at render time

**What happened.** A directive may narrow a block to named spans with
`show="second"`, and the kernel implements it (`display::filter_by_show`). But
`render::body_lines` walked the raw markers and showed every marked span, so a
page rendered spans the footer did not link.

**The fix.** `body_lines` now takes the `BlockDisplay` and shows only spans the
kernel kept. The names are the kernel's answer; render formats it.

---

## Also fixed

`bower plan --repo <unknown>` printed nothing and exited `0`. `build`, `status`,
and `push` all refuse an unmatched `--repo`; `plan` and `verify` now do too.

## What the review got right, and what it did not reach

It found all three. It confirmed only the multi-repo one before dying; the other
two came from its candidate list and needed verifying by hand. Three further
candidates were never reached and are recorded in `docs/TECHNICAL_DEBT.md`
rather than dropped: the push marker's book identity is a directory basename,
`push` does not consider lock drift, and the preprocessor has not been tried on
a book with a `README.md` chapter.

## Tests

Five new: `the_scaffolding_survives_every_step`, `render__honours_the_show_key`,
and a new `bower/tests/multi_repo.rs` with four covering both multi-repo bugs
and the unmatched-`--repo` case. One existing test was corrected, because it had
encoded bug 1 as the expected result. 212 tests pass; `make ayce` is green.
