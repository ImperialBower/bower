# The rank saga, with branches — what `fixtures::rank_saga()` looks like as a chapter

Four new directive keys: `branch=`, `from=`, `merge=`, `pr=`. Everything else
is unchanged. The spike hand-builds these six steps as data; the kernel will
parse them from a chapter exactly like this one.

## The rank itself  *(main, step 1)*

<!-- bower repo="failers" step="rank-enum" file="src/rank.rs" msg="ch01: the Rank enum" -->
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rank { Ace, King, Queen, Blank }
```
<!-- bower repo="failers" step="rank-enum" file="src/lib.rs" -->
```rust
pub mod rank;
```

## A branch that fails, and stays  *(branch `try/lookup-table`, step 2)*

The branch forks from the main step just before it in the book. It is never
merged; the reader can `git checkout try/lookup-table` and watch the test fail.
The `pr=` key declares a pull request for the branch — its body is the fence.

<!-- bower repo="failers" step="lookup-table" branch="try/lookup-table" file="src/rank.rs" op="append" expect="test_fail" msg="ch01: try a lookup table instead" -->
```rust
pub const RANKS: [Rank; 3] = [Rank::Ace, Rank::King, Rank::Queen];
#[test] fn lookup_covers_every_char() { assert_eq!(RANKS.len(), 4); }
```
<!-- bower repo="failers" branch="try/lookup-table" pr="Try a lookup table for ranks" -->
```markdown
Faster? Maybe. Correct? The test says no.
```

## Meanwhile, on main  *(main, step 3)*

<!-- bower repo="failers" step="lib-doc" file="src/lib.rs" op="replace" msg="ch01: document the crate" -->
```rust
//! A deck of cards, built the failing way.
pub mod rank;
```

## The pull request  *(branch `from-char`, steps 4–5; merged at step 6)*

<!-- bower repo="failers" step="from-char-broken" branch="from-char" file="src/rank.rs" op="append" expect="compile_fail" msg="ch02: From<char>, non-exhaustive" -->
```rust
impl From<char> for Rank {
    fn from(c: char) -> Self {
        match c { 'A' => Rank::Ace, 'K' => Rank::King, 'Q' => Rank::Queen }
    }
}
```
<!-- bower repo="failers" branch="from-char" pr="Rank::from(char)" -->
```markdown
Every char needs an answer.

Closes the gap the reviewer found.
```
<!-- bower repo="failers" step="from-char-fixed" branch="from-char" file="src/rank.rs" op="replace" msg="ch02: From<char>, every arm answered" -->
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rank { Ace, King, Queen, Blank }
impl From<char> for Rank {
    fn from(c: char) -> Self {
        match c { 'A' => Rank::Ace, 'K' => Rank::King, 'Q' => Rank::Queen, _ => Rank::Blank }
    }
}
```

A merge is a step on main. With no blocks it is a pure merge; blocks on a
merge step are the resolution, applied after the branch's changes land.

<!-- bower repo="failers" step="merge-from-char" merge="from-char" op="none" msg="ch02: merge from-char" -->

### Forking earlier, and what a conflict looks like

`from="rank-enum"` on the branch's first step forks it at that main step
instead of the nearest one. Had main edited `src/rank.rs` after that point,
the merge above would fail at plan time with `MergeConflict` — unless the
merge step replaced the file:

<!-- bower repo="failers" step="merge-from-char" merge="from-char" file="src/rank.rs" op="replace" msg="ch02: merge from-char, resolving rank.rs" -->
```rust
// resolved by hand …
```
