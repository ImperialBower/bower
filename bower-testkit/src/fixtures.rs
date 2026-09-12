//! The fixture corpus: every fixture is a complete, named book source with
//! its catalog. `valid()` fixtures must plan cleanly; `broken()` fixtures
//! must each produce their named error. A test that needs "a book shaped
//! like X" takes it from here instead of inventing one inline.

use bower_core::prelude::*;

/// A named book + catalog pair the corpus hands to tests.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub name: &'static str,
    pub book: BookSource,
    pub catalog: RepoCatalog,
}

impl Fixture {
    fn new(name: &'static str, book: BookSource, catalog: RepoCatalog) -> Self {
        Self {
            name,
            book,
            catalog,
        }
    }
}

fn failers() -> RepoCatalog {
    RepoCatalog::from_names(&["failers"])
}

fn chapter(path: &str, text: &str) -> Chapter {
    Chapter::new(path, text)
}

/// Every tag `hello-playbook` produces, in order. Public so the kernel's tests
/// and the CLI's replay tests assert against one list rather than two that can
/// drift apart.
pub const HELLO_PLAYBOOK_TAGS: &[&str] = &[
    "step-001-cargo-init",
    "step-002-hello-runs",
    "step-003-makefile",
    "step-004-make-help",
    "step-005-rustfmt",
    "step-006-toolchain",
    "step-007-lints",
    "step-008-gate-fmt-clippy",
    "step-009-greet-lib",
    "step-010-greet-test",
    "step-011-test-that-fails",
    "step-012-test-that-passes",
    "step-013-wont-compile",
    "step-014-scratch-fixed",
    "step-015-deny-toml",
    "step-016-security-scan",
    "step-017-gate-test-audit",
    "step-018-tool-versions",
    "step-019-ci-workflow",
    "step-020-drop-scratch",
];

/// Every path the book's twenty steps leave behind. The generated repository
/// adds `STEPS.md` on top of these; the kernel never sees that file.
pub const HELLO_PLAYBOOK_FINAL_PATHS: &[&str] = &[
    ".github/workflows/ci.yml",
    ".tool-versions",
    "Cargo.toml",
    "Makefile",
    "bin/security-scan",
    "deny.toml",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "src/lib.rs",
    "src/main.rs",
];

fn hello_playbook_catalog() -> RepoCatalog {
    RepoCatalog::from_names(&["hello-playbook"])
}

/// The sample book: a dev-playbook-shaped Rust hello world, taught in six
/// chapters that build the repository they describe, plus an appendix that
/// credits the cover artwork and carries no directives at all — which is what
/// makes it the fixture's one prose-only chapter.
///
/// The chapters live on disk under `books/hello-playbook/src/` and are pulled
/// in with `include_str!`, which resolves at compile time — the kernel still
/// sees nothing but text, and the testkit does no runtime I/O.
#[must_use]
pub fn hello_playbook() -> Fixture {
    Fixture::new(
        "hello-playbook",
        BookSource::from_chapters(vec![
            chapter(
                "src/ch01-a-repo-that-builds.md",
                include_str!("../../books/hello-playbook/src/ch01-a-repo-that-builds.md"),
            ),
            chapter(
                "src/ch02-the-gate.md",
                include_str!("../../books/hello-playbook/src/ch02-the-gate.md"),
            ),
            chapter(
                "src/ch03-lints-and-format.md",
                include_str!("../../books/hello-playbook/src/ch03-lints-and-format.md"),
            ),
            chapter(
                "src/ch04-tests-and-failing-on-purpose.md",
                include_str!("../../books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md"),
            ),
            chapter(
                "src/ch05-supply-chain.md",
                include_str!("../../books/hello-playbook/src/ch05-supply-chain.md"),
            ),
            chapter(
                "src/ch06-ci.md",
                include_str!("../../books/hello-playbook/src/ch06-ci.md"),
            ),
            chapter(
                "src/appendix-credits.md",
                include_str!("../../books/hello-playbook/src/appendix-credits.md"),
            ),
        ]),
        hello_playbook_catalog(),
    )
}

/// The smallest possible book: one chapter, one create block.
#[must_use]
pub fn minimal() -> Fixture {
    Fixture::new(
        "minimal",
        BookSource::from_chapters(vec![chapter(
            "ch01.md",
            concat!(
                "# Hello\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" -->\n",
                "```rust\npub fn hello() {}\n```\n",
            ),
        )]),
        failers(),
    )
}

/// The spec § 9 worked example as an executable fixture: the `Rank` story
/// from pkcore's DIARY — create, an intentionally failing region step, the
/// fix plus appended tests as one multi-block step.
#[must_use]
pub fn rank_saga() -> Fixture {
    Fixture::new(
        "rank_saga",
        BookSource::from_chapters(vec![chapter(
            "ch01-ranks.md",
            concat!(
                "# Introduce Rank\n\n",
                "<!-- bower repo=\"failers\" file=\"src/rank.rs\" step=\"rank-enum\" -->\n",
                "```rust\n",
                "#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n",
                "pub enum Rank { Ace, King }\n",
                "// bower:begin from_char\n",
                "// bower:end from_char\n",
                "// bower:begin tests\n",
                "// bower:end tests\n",
                "```\n\n",
                "## The failing version\n\n",
                "<!-- bower repo=\"failers\" file=\"src/rank.rs\" op=\"region\" region=\"from_char\" step=\"from-char-broken\" expect=\"compile_fail\" -->\n",
                "```rust\n",
                "impl From<char> for Rank {\n",
                "    fn from(c: char) -> Self { match c { 'A' => Rank::Ace } }\n",
                "}\n",
                "```\n\n",
                "## The fix, tested the brute-force way\n\n",
                "<!-- bower repo=\"failers\" file=\"src/rank.rs\" op=\"region\" region=\"from_char\" step=\"from-char\" -->\n",
                "```rust\n",
                "impl From<char> for Rank {\n",
                "    fn from(c: char) -> Self {\n",
                "        match c { 'A' => Rank::Ace, _ => Rank::King }\n",
                "    }\n",
                "}\n",
                "```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/rank.rs\" op=\"region\" region=\"tests\" step=\"from-char\" -->\n",
                "```rust\n",
                "#[cfg(test)]\n",
                "mod rank_tests {\n",
                "    use super::*;\n",
                "    #[test]\n",
                "    fn from__ace() { assert_eq!(Rank::from('A'), Rank::Ace); }\n",
                "}\n",
                "```\n",
            ),
        )]),
        failers(),
    )
}

/// One book feeding two repos, with an `after=` bend in the order.
#[must_use]
pub fn multi_repo() -> Fixture {
    Fixture::new(
        "multi_repo",
        BookSource::from_chapters(vec![chapter(
            "ch01.md",
            concat!(
                "# Two repos\n\n",
                "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"early\" after=\"late\" -->\n",
                "```rust\nfn a() {}\n```\n\n",
                "<!-- bower repo=\"clock\" file=\"c.rs\" -->\n",
                "```rust\nfn c() {}\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"b.rs\" step=\"late\" -->\n",
                "```rust\nfn b() {}\n```\n",
            ),
        )]),
        RepoCatalog::from_names(&["failers", "clock"]),
    )
}

/// Every display-marker texture at once: hidden lines, an anonymous span,
/// named spans with a `show=` filter, and a fully-elided block.
#[must_use]
pub fn display_textures() -> Fixture {
    Fixture::new(
        "display_textures",
        BookSource::from_chapters(vec![chapter(
            "ch02-display.md",
            concat!(
                "# Display\n\n",
                "<!-- bower repo=\"failers\" file=\"src/show.rs\" step=\"show\" -->\n",
                "```rust\n",
                "# use std::fmt::Debug;\n",
                "fn boilerplate() {}\n",
                "// bower:show\n",
                "pub fn the_point() {}\n",
                "// bower:show end\n",
                "```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/named.rs\" step=\"named\" show=\"two\" -->\n",
                "```rust\n",
                "// bower:show begin one\n",
                "fn first() {}\n",
                "// bower:show end\n",
                "// bower:show begin two\n",
                "fn second() {}\n",
                "// bower:show end\n",
                "```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/silent.rs\" step=\"silent\" hidden=\"false\" -->\n",
                "```rust\n",
                "# fn never_in_repo() {}\n",
                "fn in_repo() {}\n",
                "```\n",
            ),
        )]),
        failers(),
    )
}

/// Every remaining op in one book: replace, append, delete (with `paths`),
/// copy, and a prose-only step.
#[must_use]
pub fn op_sampler() -> Fixture {
    let mut book = BookSource::from_chapters(vec![chapter(
        "ch03-ops.md",
        concat!(
            "# Ops\n\n",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nv1\n```\n\n",
            "<!-- bower repo=\"failers\" file=\"b.rs\" -->\n```rust\nkeep\n```\n\n",
            "<!-- bower repo=\"failers\" file=\"a.rs\" op=\"replace\" -->\n```rust\nv2\n```\n\n",
            "<!-- bower repo=\"failers\" file=\"a.rs\" op=\"append\" msg=\"grow a\" -->\n```rust\nv3\n```\n\n",
            "<!-- bower repo=\"failers\" file=\"logo.png\" op=\"copy\" src=\"img/logo.png\" -->\n\n",
            "<!-- bower repo=\"failers\" op=\"none\" msg=\"a refactor the book narrates\" expect=\"none\" -->\n\n",
            "<!-- bower repo=\"failers\" op=\"delete\" file=\"a.rs\" paths=\"logo.png\" -->\n",
        ),
    )]);
    book.assets
        .insert("img/logo.png".to_string(), vec![0x89, 0x50, 0x4e, 0x47]);
    Fixture::new("op_sampler", book, failers())
}

/// The block library in use: chapter includes a library block and
/// overrides its message; unicode in the message for good measure.
#[must_use]
pub fn include_library() -> Fixture {
    let mut book = BookSource::from_chapters(vec![chapter(
        "ch04-include.md",
        "# Include\n\n<!-- bower include=\"blocks/rank.md\" msg=\"Rank enum — the café edition ♠\" -->\n",
    )]);
    book.library.insert(
        "blocks/rank.md".to_string(),
        concat!(
            "<!-- bower repo=\"failers\" file=\"src/rank.rs\" msg=\"library default\" -->\n",
            "```rust\npub enum Rank { Ace }\n```\n",
        )
        .to_string(),
    );
    Fixture::new("include_library", book, failers())
}

/// Directives that must NOT be parsed: inside a foreign fence, and inside
/// a wider fence showing Bower's own syntax.
#[must_use]
pub fn self_hosting_chapter() -> Fixture {
    Fixture::new(
        "self_hosting_chapter",
        BookSource::from_chapters(vec![chapter(
            "ch05-meta.md",
            concat!(
                "# How Bower works\n\n",
                "````markdown\n",
                "<!-- bower repo=\"failers\" file=\"would-be-a-bug.rs\" -->\n",
                "```rust\nnot real\n```\n",
                "````\n\n",
                "<!-- bower repo=\"failers\" file=\"real.rs\" -->\n",
                "```rust\nreal\n```\n",
            ),
        )]),
        failers(),
    )
}

/// The § 15 play track: a step, a doc-order play cell bound to it, and
/// an explicitly-bound play cell from a later chapter position.
#[must_use]
pub fn notebook_play() -> Fixture {
    Fixture::new(
        "notebook_play",
        BookSource::from_chapters(vec![chapter(
            "ch06-notebook.md",
            concat!(
                "# Playable Rank\n\n",
                "<!-- bower repo=\"failers\" file=\"src/rank.rs\" step=\"rank\" -->\n",
                "```rust\npub enum Rank { Ace }\n```\n\n",
                "<!-- bower repo=\"failers\" notebook=\"play\" -->\n",
                "```python\nfrom failers import Rank\nRank.from_char(\"A\")\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lo.rs\" step=\"lo\" -->\n",
                "```rust\npub enum Lo { Wheel }\n```\n\n",
                "<!-- bower repo=\"failers\" notebook=\"play\" step=\"rank\" -->\n",
                "```python\nRank.from_char(\"Z\")  # now break it\n```\n",
            ),
        )]),
        failers(),
    )
}

/// Every exercise form on every `expect`: a key form on a `compile_fail`
/// step, a positional block form on a `pass` step, an explicitly bound block
/// form on a `test_fail` step, and a key form on a prose (`none`) step. The
/// last step carries none, because nothing follows it to be the answer.
#[must_use]
pub fn exercise_forms() -> Fixture {
    Fixture::new(
        "exercise_forms",
        BookSource::from_chapters(vec![chapter(
            "ch07-exercises.md",
            concat!(
                "# Your turn\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"broken\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n",
                "```rust\npub fn answer() -> u32 { \"42\" }\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"fixed\" -->\n",
                "```rust\npub fn answer() -> u32 { 42 }\n```\n\n",
                "<!-- bower repo=\"failers\" exercise=\"Do it without a literal\" -->\n",
                "```markdown\nParse text instead.\n\n- `str::parse` is one way.\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"append\" step=\"tested\" expect=\"test_fail\" -->\n",
                "```rust\n#[test]\nfn answers() { assert_eq!(answer(), 41); }\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"green\" -->\n",
                "```rust\npub fn answer() -> u32 { 42 }\n#[test]\nfn answers() { assert_eq!(answer(), 42); }\n```\n\n",
                "<!-- bower repo=\"failers\" exercise=\"Make the test pass\" step=\"tested\" -->\n",
                "```markdown\nThe test is right. The function is not.\n```\n\n",
                "<!-- bower repo=\"failers\" op=\"none\" step=\"narrated\" expect=\"none\" msg=\"docs: a refactor the book only describes\" exercise=\"Try the refactor yourself\" -->\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"append\" step=\"end\" -->\n",
                "```rust\n// fin\n```\n",
            ),
        )]),
        failers(),
    )
}

/// Every legal `capture × expect` pair, bound both ways: `check` on a green
/// step and its `verify` reaching back by `step=` from the end of the chapter,
/// `check` on a `compile_fail` step, and both on a `test_fail` step. Two fences
/// are empty: recorded and not-yet-recorded outputs plan alike.
#[must_use]
pub fn captured_outputs() -> Fixture {
    Fixture::new(
        "captured_outputs",
        BookSource::from_chapters(vec![chapter(
            "ch08-outputs.md",
            concat!(
                "# What the compiler said\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"green\" -->\n",
                "```rust\npub fn answer() -> u32 { 42 }\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"check\" -->\n",
                "```text\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"broken\" expect=\"compile_fail\" -->\n",
                "```rust\npub fn answer() -> u32 { \"42\" }\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"check\" -->\n",
                "```text\nerror[E0308]: mismatched types\n[...]\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"red\" expect=\"test_fail\" -->\n",
                "```rust\npub fn answer() -> u32 { 41 }\n#[test]\nfn answers() { assert_eq!(answer(), 42); }\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"check\" -->\n",
                "```text\n```\n\n",
                "<!-- bower repo=\"failers\" output=\"verify\" -->\n",
                "```text\n[...]\ntest answers ... FAILED\n[...]\n```\n\n",
                "Back to the green step, by name:\n\n",
                "<!-- bower repo=\"failers\" output=\"verify\" step=\"green\" -->\n",
                "```text\n[...]\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n```\n",
            ),
        )]),
        failers(),
    )
}

/// Every valid fixture, for corpus-wide properties and the coverage report.
#[must_use]
pub fn valid() -> Vec<Fixture> {
    vec![
        minimal(),
        rank_saga(),
        multi_repo(),
        display_textures(),
        op_sampler(),
        include_library(),
        self_hosting_chapter(),
        notebook_play(),
        exercise_forms(),
        captured_outputs(),
        hello_playbook(),
    ]
}

/// A broken book that is one chapter of `text`, named for the
/// [`BowerError`] variant it must produce.
fn single(name: &'static str, text: &str) -> (&'static str, Fixture) {
    (
        name,
        Fixture::new(
            name,
            BookSource::from_chapters(vec![chapter("bad.md", text)]),
            failers(),
        ),
    )
}

/// Errors raised while reading the book source: directives, fences, and
/// the block library.
fn broken_source() -> Vec<(&'static str, Fixture)> {
    vec![
        single(
            "UnknownRepo",
            "<!-- bower repo=\"typo\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ),
        single(
            "MissingKey",
            "<!-- bower repo=\"failers\" -->\n```rust\nx\n```\n",
        ),
        single(
            "UnknownKey",
            "<!-- bower repo=\"failers\" file=\"a.rs\" flavor=\"salt\" -->\n```rust\nx\n```\n",
        ),
        single(
            "BadValue",
            "<!-- bower repo=\"failers\" file=\"a.rs\" op=\"explode\" -->\n```rust\nx\n```\n",
        ),
        single(
            "DirectiveParse",
            "<!-- bower repo=\"failers file=\"a.rs\" -->\n```rust\nx\n```\n",
        ),
        single(
            "DirectiveWithoutBlock",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\nprose instead\n",
        ),
        single(
            "UnclosedFence",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nnever closes\n",
        ),
        single(
            "IncludeMissing",
            "<!-- bower include=\"blocks/ghost.md\" -->\n",
        ),
    ]
}

/// Errors raised while folding steps into a tree, and while ordering them.
fn broken_tree() -> Vec<(&'static str, Fixture)> {
    vec![
        single(
            "FileAlreadyExists",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\ny\n```\n",
        ),
        single(
            "FileNotCreated",
            "<!-- bower repo=\"failers\" file=\"a.rs\" op=\"append\" -->\n```rust\nx\n```\n",
        ),
        single(
            "RegionMissing",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nplain\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" op=\"region\" region=\"ghost\" -->\n```rust\nx\n```\n",
        ),
        single(
            "RegionUnbalanced",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\n// bower:begin b\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" op=\"region\" region=\"b\" -->\n```rust\nx\n```\n",
        ),
        single(
            "DuplicateFileInStep",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"s\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" step=\"s\" op=\"append\" -->\n```rust\ny\n```\n",
        ),
        single(
            "ConflictingExpectInStep",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"s\" expect=\"pass\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" file=\"b.rs\" step=\"s\" expect=\"test_fail\" -->\n```rust\ny\n```\n",
        ),
        single(
            "OrphanAfter",
            "<!-- bower repo=\"failers\" file=\"a.rs\" after=\"ghost\" -->\n```rust\nx\n```\n",
        ),
        single(
            "OrderingCycle",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"a\" after=\"b\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" file=\"b.rs\" step=\"b\" after=\"a\" -->\n```rust\ny\n```\n",
        ),
    ]
}

/// Errors raised by display spans, copied assets, and notebook play cells.
fn broken_display() -> Vec<(&'static str, Fixture)> {
    vec![
        single(
            "UnclosedShowSpan",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\n// bower:show\nx\n```\n",
        ),
        single(
            "NestedShowSpan",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\n// bower:show\n// bower:show\nx\n// bower:show end\n```\n",
        ),
        single(
            "OrphanShowEnd",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\n// bower:show end\nx\n```\n",
        ),
        single(
            "UnknownShowSpan",
            "<!-- bower repo=\"failers\" file=\"a.rs\" show=\"ghost\" -->\n```rust\n// bower:show begin real\nx\n// bower:show end\n```\n",
        ),
        single(
            "AssetMissing",
            "<!-- bower repo=\"failers\" file=\"logo.png\" op=\"copy\" src=\"img/ghost.png\" -->\n",
        ),
        single(
            "PlayCellUnbound",
            "<!-- bower repo=\"failers\" notebook=\"play\" -->\n```python\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ),
        single(
            "PlayCellUnknownStep",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" notebook=\"play\" step=\"ghost\" -->\n```python\nx\n```\n",
        ),
        single(
            "PlayCellConflictingKeys",
            "<!-- bower repo=\"failers\" notebook=\"play\" file=\"a.rs\" -->\n```python\nx\n```\n",
        ),
    ]
}

/// Errors raised by exercises: binding, duplication, the missing answer, and
/// the play-cell collision.
fn broken_exercises() -> Vec<(&'static str, Fixture)> {
    vec![
        single(
            "ExerciseUnbound",
            "<!-- bower repo=\"failers\" exercise=\"Try\" -->\n```markdown\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ),
        single(
            "ExerciseUnknownStep",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" exercise=\"Try\" step=\"ghost\" -->\n```markdown\nx\n```\n",
        ),
        single(
            "ExerciseDuplicate",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"one\" exercise=\"First\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" exercise=\"Second\" -->\n```markdown\nx\n```\n<!-- bower repo=\"failers\" file=\"b.rs\" -->\n```rust\ny\n```\n",
        ),
        single(
            "ExerciseWithoutAnswer",
            "<!-- bower repo=\"failers\" file=\"a.rs\" exercise=\"Try\" -->\n```rust\nx\n```\n",
        ),
        single(
            "ExerciseConflictingKeys",
            "<!-- bower repo=\"failers\" notebook=\"play\" exercise=\"Try\" -->\n```python\nx\n```\n",
        ),
    ]
}

/// Errors raised by output blocks: binding, a key of another kind of block, a
/// second block for one capture, and a capture the verifier never runs.
fn broken_outputs() -> Vec<(&'static str, Fixture)> {
    vec![
        single(
            "OutputUnbound",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ),
        single(
            "OutputUnknownStep",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"check\" step=\"ghost\" -->\n```text\n```\n",
        ),
        single(
            "OutputConflictingKeys",
            "<!-- bower repo=\"failers\" output=\"check\" file=\"a.rs\" -->\n```text\n```\n",
        ),
        single(
            "OutputDuplicate",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
        ),
        single(
            "OutputNeverRuns",
            "<!-- bower repo=\"failers\" file=\"a.rs\" expect=\"compile_fail\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"verify\" -->\n```text\n```\n",
        ),
    ]
}

/// Broken books, one per error family. The paired name states the
/// [`BowerError`] variant the fixture must produce.
#[must_use]
pub fn broken() -> Vec<(&'static str, Fixture)> {
    let mut cases = broken_source();
    cases.extend(broken_tree());
    cases.extend(broken_display());
    cases.extend(broken_exercises());
    cases.extend(broken_outputs());

    let mut malformed_include = BookSource::from_chapters(vec![chapter(
        "bad.md",
        "<!-- bower include=\"blocks/empty.md\" -->\n",
    )]);
    malformed_include
        .library
        .insert("blocks/empty.md".to_string(), "no fence here\n".to_string());
    cases.push((
        "IncludeMalformed",
        Fixture::new("IncludeMalformed", malformed_include, failers()),
    ));

    cases
}
