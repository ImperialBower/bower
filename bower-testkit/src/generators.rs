//! Proptest strategies over the kernel's input space: arbitrary *valid*
//! books. Properties proven over these hold for any book an author can
//! legally write — the generators are the state-space walker, the
//! properties in `tests/` are the invariants.

use bower_core::prelude::*;
use proptest::prelude::*;

/// One generated modification to a file.
#[derive(Clone, Debug)]
pub struct GenMod {
    /// `true` → `op="replace"`, `false` → `op="append"`.
    pub replace: bool,
    pub lines: Vec<String>,
    /// Wrap the content in a `bower:show` span.
    pub marked: bool,
}

/// One generated file: a create block plus follow-up modifications.
#[derive(Clone, Debug)]
pub struct GenFile {
    pub create: Vec<String>,
    pub mods: Vec<GenMod>,
}

fn line() -> impl Strategy<Value = String> {
    // Lowercase words: can never collide with directives, fences, markers,
    // hidden-line prefixes, or headings.
    "[a-z]{1,8}"
}

fn lines(max: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(line(), 1..=max)
}

fn arb_mod() -> impl Strategy<Value = GenMod> {
    (any::<bool>(), lines(3), any::<bool>()).prop_map(|(replace, lines, marked)| GenMod {
        replace,
        lines,
        marked,
    })
}

fn arb_file() -> impl Strategy<Value = GenFile> {
    (lines(4), prop::collection::vec(arb_mod(), 0..=2))
        .prop_map(|(create, mods)| GenFile { create, mods })
}

/// An arbitrary valid book against a one-repo catalog. Every generated
/// book must plan cleanly; a generated book that errors is itself a bug —
/// in the generator or in the kernel — and the properties treat it as one.
pub fn arb_book() -> impl Strategy<Value = (BookSource, RepoCatalog)> {
    prop::collection::vec(arb_file(), 1..=3).prop_map(|files| build(&files))
}

fn build(files: &[GenFile]) -> (BookSource, RepoCatalog) {
    let mut text = String::from("# Generated\n\n");

    for (i, f) in files.iter().enumerate() {
        text.push_str(&directive_line(i, "create", None));
        text.push_str("```rust\n");
        for l in &f.create {
            text.push_str(l);
            text.push('\n');
        }
        text.push_str("```\n\n");
    }
    for (i, f) in files.iter().enumerate() {
        for m in &f.mods {
            let op = if m.replace { "replace" } else { "append" };
            text.push_str(&directive_line(i, op, None));
            text.push_str("```rust\n");
            if m.marked {
                text.push_str("// bower:show\n");
            }
            for l in &m.lines {
                text.push_str(l);
                text.push('\n');
            }
            if m.marked {
                text.push_str("// bower:show end\n");
            }
            text.push_str("```\n\n");
        }
    }

    (
        BookSource::from_chapters(vec![Chapter::new("generated.md", &text)]),
        RepoCatalog::from_names(&["gen"]),
    )
}

fn directive_line(file_idx: usize, op: &str, step: Option<&str>) -> String {
    let step_key = step.map(|s| format!(" step=\"{s}\"")).unwrap_or_default();
    format!("<!-- bower repo=\"gen\" file=\"f{file_idx}.rs\" op=\"{op}\"{step_key} -->\n")
}
