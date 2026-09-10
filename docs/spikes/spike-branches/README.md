# bower-spike-branches

The spike behind `docs/EPIC-09_Branches.md`: **branches, merges, and pull
requests as plan-time values**. Zero dependencies, edition 2021, runs on
rustc 1.75 so it builds anywhere; replay shells out to `git` plumbing.

```sh
cargo test          # 16 tests: the fold, the errors, replay determinism
cargo run           # prints the proposed bower.lock shape, replays into
                    # /tmp/bower-spike-failers, prints `git log --graph`
```

What it proves:

- a branch is a line of history the fold keeps a tree for; it forks at the
  main step preceding its first step, or at `from=`;
- a merge is a main step whose tree is *the branch's blocks re-applied over
  main*, plus any resolution blocks; a file both sides touched since the
  fork is a plan-time `MergeConflict` unless the merge step replaces it;
- every step's parents are plan values, so replay is a pure function of the
  plan — SHAs are byte-identical across runs, and editing a branch step
  changes only what descends from it;
- an unmerged branch survives as a ref with an open PR; a merged one is a
  two-parent commit on main with a merged PR.

Three hand-mutations (drop the conflict filter, drop the second merge
parent, fork from an empty tree) each break tests. `fixture-book.md` shows
the same six steps as chapter directives.

Not here, on purpose: markdown parsing, regions, display markers, the forge.
The EPIC maps each onto the real `bower-core` / `bower` symbols.
