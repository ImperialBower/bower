//! The rank saga, with two branches. See `fixture-book.md` for what the
//! same steps look like as directives in a chapter.

use crate::{Block, Expect, Op, Pr, Step};

fn lines(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }

fn step(id: &str, msg: &str) -> Step {
    Step { id: id.into(), msg: msg.into(), expect: Expect::Pass, blocks: vec![], branch: None, from: None, merge: None, pr: None }
}
fn blk(file: &str, op: Op, body: &str) -> Block {
    Block { file: Some(file.into()), op, lines: lines(body) }
}

pub const RANK: &str = "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum Rank { Ace, King, Queen, Blank }";
pub const LIB: &str = "pub mod rank;";
pub const FROM_CHAR_BROKEN: &str = "impl From<char> for Rank {\n    fn from(c: char) -> Self {\n        match c { 'A' => Rank::Ace, 'K' => Rank::King, 'Q' => Rank::Queen }\n    }\n}";
pub const FROM_CHAR_FIXED: &str = "impl From<char> for Rank {\n    fn from(c: char) -> Self {\n        match c { 'A' => Rank::Ace, 'K' => Rank::King, 'Q' => Rank::Queen, _ => Rank::Blank }\n    }\n}";
pub const LOOKUP: &str = "pub const RANKS: [Rank; 3] = [Rank::Ace, Rank::King, Rank::Queen];\n#[test] fn lookup_covers_every_char() { assert_eq!(RANKS.len(), 4); }";
pub const LIB_DOC: &str = "//! A deck of cards, built the failing way.\npub mod rank;";

/// The happy path: one abandoned branch, one merged PR, main moving meanwhile.
pub fn rank_saga() -> Vec<Step> {
    let mut s1 = step("rank-enum", "ch01: the Rank enum");
    s1.blocks = vec![blk("src/rank.rs", Op::Create, RANK), blk("src/lib.rs", Op::Create, LIB)];

    // A branch that fails and is left standing — "try/lookup-table".
    let mut s2 = step("lookup-table", "ch01: try a lookup table instead");
    s2.branch = Some("try/lookup-table".into());
    s2.expect = Expect::TestFail;
    s2.blocks = vec![blk("src/rank.rs", Op::Append, LOOKUP)];
    s2.pr = Some(Pr { title: "Try a lookup table for ranks".into(), body: lines("Faster? Maybe. Correct? The test says no.") });

    // Main keeps moving on a different file.
    let mut s3 = step("lib-doc", "ch01: document the crate");
    s3.blocks = vec![blk("src/lib.rs", Op::Replace, LIB_DOC)];

    // The PR branch: broken first, then fixed, then merged.
    let mut s4 = step("from-char-broken", "ch02: From<char>, non-exhaustive");
    s4.branch = Some("from-char".into());
    s4.expect = Expect::CompileFail;
    s4.blocks = vec![blk("src/rank.rs", Op::Append, FROM_CHAR_BROKEN)];
    s4.pr = Some(Pr { title: "Rank::from(char)".into(), body: lines("Every char needs an answer.\n\nCloses the gap the reviewer found.") });

    let mut s5 = step("from-char-fixed", "ch02: From<char>, every arm answered");
    s5.branch = Some("from-char".into());
    s5.blocks = vec![blk("src/rank.rs", Op::Replace, &format!("{RANK}\n{FROM_CHAR_FIXED}"))];

    let mut s6 = step("merge-from-char", "ch02: merge from-char");
    s6.merge = Some("from-char".into());

    vec![s1, s2, s3, s4, s5, s6]
}

/// The PR branch forks at `rank-enum`; main then edits `src/rank.rs` (step
/// `lib-doc`) and the merge brings no resolution. Note that without the
/// explicit `from=` this is *not* a conflict — the branch would fork after
/// main's edit and simply build on it.
pub fn conflict_unresolved() -> Vec<Step> {
    let mut v = rank_saga();
    v[2].blocks = vec![blk("src/rank.rs", Op::Replace, &format!("// main touched this\n{RANK}"))];
    v[3].from = Some("rank-enum".into());
    v
}

/// Same, but the merge step replaces the file: the resolution is a step.
pub fn conflict_resolved() -> Vec<Step> {
    let mut v = conflict_unresolved();
    v[5].blocks = vec![blk("src/rank.rs", Op::Replace, &format!("// resolved by hand\n{RANK}\n{FROM_CHAR_FIXED}"))];
    v[5].msg = "ch02: merge from-char, resolving rank.rs".into();
    v
}

pub fn step_after_merge() -> Vec<Step> {
    let mut v = rank_saga();
    let mut late = step("from-char-late", "ch02: one more on a closed branch");
    late.branch = Some("from-char".into());
    late.blocks = vec![blk("src/rank.rs", Op::Append, "// too late")];
    v.push(late);
    v
}

pub fn merge_unknown() -> Vec<Step> {
    let mut v = rank_saga();
    v[5].merge = Some("no-such-branch".into());
    v
}

pub fn merge_twice() -> Vec<Step> {
    let mut v = rank_saga();
    let mut again = step("merge-again", "ch02: merge from-char again");
    again.merge = Some("from-char".into());
    v.push(again);
    v
}

pub fn explicit_from() -> Vec<Step> {
    let mut v = rank_saga();
    // Fork the PR branch from the enum step, before main documented the crate.
    v[3].from = Some("rank-enum".into());
    v
}

pub fn from_not_on_main() -> Vec<Step> {
    let mut v = rank_saga();
    v[3].from = Some("lookup-table".into());
    v
}
