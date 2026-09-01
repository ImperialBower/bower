//! `mdbook-bower` — the mdBook preprocessor (spec § 5.4).
//!
//! mdBook speaks to a preprocessor over stdin and stdout in JSON. Two calls:
//!
//! * `mdbook-bower supports <renderer>` — exit `0` if this renderer is
//!   supported, non-zero otherwise. mdBook drops the preprocessor for renderers
//!   that say no.
//! * `mdbook-bower` with no arguments — read `[context, book]` from stdin,
//!   write the transformed book to stdout.
//!
//! As of EPIC-03 Phase 2 the chapters are rewritten: directives are stripped,
//! display markers applied, and a `#step-<id>` anchor injected before each
//! block. Annotation rot fails `mdbook build` rather than reaching a reader.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::process::ExitCode;

use bower::config::BookConfig;
use bower::render;
use bower::mdbook;
use bower_core::prelude::plan;

/// The renderers this preprocessor knows how to rewrite for. HTML only: the
/// elision rendering spec § 3.4 describes for epub is a different shape, and
/// claiming support for a renderer we do not handle would silently produce
/// wrong pages.
const SUPPORTED: &[&str] = &["html"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("supports") {
        let renderer = args.get(1).map(String::as_str).unwrap_or_default();
        return if SUPPORTED.contains(&renderer) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    match preprocess() {
        Ok(out) => {
            let mut stdout = std::io::stdout();
            if let Err(e) = stdout.write_all(out.as_bytes()) {
                eprintln!("mdbook-bower: cannot write output: {e}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("mdbook-bower: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Read the `[context, book]` envelope, resolve the book through the kernel,
/// and return the book to hand back.
fn preprocess() -> Result<String, String> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("cannot read stdin: {e}"))?;

    let envelope: serde_json::Value =
        serde_json::from_str(&input).map_err(|e| format!("stdin is not valid JSON: {e}"))?;

    let (context, book) = mdbook::split(envelope)?;

    let config = BookConfig::load(&context.root)
        .map_err(|e| format!("cannot load {}/bower.toml: {e}", context.root.display()))?;

    // Resolving is not optional even though nothing is rewritten yet: a book
    // that does not plan is a book with broken annotations, and publishing it
    // would hand a reader links to states that were never computed.
    let resolved = plan(&mdbook::book_source(&book), &config.catalog())
        .map_err(|errs| format!("the book does not resolve:\n{errs}"))?;

    let mut book = book;
    mdbook::map_chapters(&mut book, |path, text| render::chapter(text, path, &resolved));

    serde_json::to_string(&book).map_err(|e| format!("cannot serialize the book: {e}"))
}
