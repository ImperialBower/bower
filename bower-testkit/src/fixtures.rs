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
    ]
}

/// Broken books, one per error family. The paired name states the
/// [`BowerError`] variant the fixture must produce.
#[must_use]
pub fn broken() -> Vec<(&'static str, Fixture)> {
    let single = |name: &'static str, text: &str| {
        (
            name,
            Fixture::new(
                name,
                BookSource::from_chapters(vec![chapter("bad.md", text)]),
                failers(),
            ),
        )
    };
    let mut cases = vec![
        single("UnknownRepo", "<!-- bower repo=\"typo\" file=\"a.rs\" -->\n```rust\nx\n```\n"),
        single("MissingKey", "<!-- bower repo=\"failers\" -->\n```rust\nx\n```\n"),
        single("UnknownKey", "<!-- bower repo=\"failers\" file=\"a.rs\" flavor=\"salt\" -->\n```rust\nx\n```\n"),
        single("BadValue", "<!-- bower repo=\"failers\" file=\"a.rs\" op=\"explode\" -->\n```rust\nx\n```\n"),
        single("DirectiveParse", "<!-- bower repo=\"failers file=\"a.rs\" -->\n```rust\nx\n```\n"),
        single("DirectiveWithoutBlock", "<!-- bower repo=\"failers\" file=\"a.rs\" -->\nprose instead\n"),
        single("UnclosedFence", "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nnever closes\n"),
        single("IncludeMissing", "<!-- bower include=\"blocks/ghost.md\" -->\n"),
        single(
            "FileAlreadyExists",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\ny\n```\n",
        ),
        single("FileNotCreated", "<!-- bower repo=\"failers\" file=\"a.rs\" op=\"append\" -->\n```rust\nx\n```\n"),
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
        single("OrphanAfter", "<!-- bower repo=\"failers\" file=\"a.rs\" after=\"ghost\" -->\n```rust\nx\n```\n"),
        single(
            "OrderingCycle",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"a\" after=\"b\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" file=\"b.rs\" step=\"b\" after=\"a\" -->\n```rust\ny\n```\n",
        ),
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
    ];

    cases.push(single(
        "PlayCellUnbound",
        "<!-- bower repo=\"failers\" notebook=\"play\" -->\n```python\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
    ));
    cases.push(single(
        "PlayCellUnknownStep",
        "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" notebook=\"play\" step=\"ghost\" -->\n```python\nx\n```\n",
    ));
    cases.push(single(
        "PlayCellConflictingKeys",
        "<!-- bower repo=\"failers\" notebook=\"play\" file=\"a.rs\" -->\n```python\nx\n```\n",
    ));

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
