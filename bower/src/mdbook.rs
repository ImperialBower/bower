//! mdBook's preprocessor protocol, modelled as little as possible.
//!
//! mdBook sends `[context, book]` as JSON on stdin and expects the transformed
//! book back on stdout. Only the handful of fields this crate reads are typed;
//! everything else round-trips through `serde_json::Value` untouched.
//!
//! That choice is deliberate. A typed mirror of mdBook's `Book` has to be kept
//! in step with every mdBook release, and every field it forgets is a field it
//! silently drops from somebody's book.

use std::path::PathBuf;

use bower_core::prelude::{BookSource, Chapter};
use serde::Deserialize;
use serde_json::Value;

/// The half of the envelope this crate reads.
#[derive(Debug, Deserialize)]
pub struct Context {
    /// The book's root directory — where `bower.toml` lives.
    pub root: PathBuf,
}

/// Split the `[context, book]` envelope.
///
/// # Errors
///
/// Returns a message naming what was wrong if the value is not a pair, or the
/// context does not carry the fields this crate needs.
pub fn split(envelope: Value) -> Result<(Context, Value), String> {
    let Value::Array(mut items) = envelope else {
        return Err("expected a [context, book] pair on stdin".to_string());
    };
    if items.len() != 2 {
        return Err(format!(
            "expected a [context, book] pair on stdin, got {} element(s)",
            items.len()
        ));
    }
    let book = items.pop().unwrap_or(Value::Null);
    let context = items.pop().unwrap_or(Value::Null);
    let context: Context = serde_json::from_value(context)
        .map_err(|e| format!("unusable preprocessor context: {e}"))?;
    Ok((context, book))
}

/// Every chapter in the book, in reading order.
///
/// A book is a tree: `sections` holds items, a `Chapter` item may hold
/// `sub_items`, and only `Chapter` items carry content. Separators and parts
/// carry none and are skipped.
#[must_use]
pub fn chapters(book: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    collect(book.get("sections"), &mut out);
    out
}

fn collect<'a>(sections: Option<&'a Value>, out: &mut Vec<&'a Value>) {
    let Some(Value::Array(items)) = sections else {
        return;
    };
    for item in items {
        if let Some(chapter) = item.get("Chapter") {
            out.push(chapter);
            collect(chapter.get("sub_items"), out);
        }
    }
}

/// A chapter's book-relative path, `src/`-prefixed to match the loader's
/// convention (`bower/src/loader.rs`). A `Book-Source` trailer and a
/// preprocessor anchor must name the same chapter, or the two directions of the
/// link disagree.
#[must_use]
pub fn chapter_path(chapter: &Value) -> Option<String> {
    let path = chapter.get("path")?.as_str()?;
    Some(format!("src/{path}"))
}

/// A chapter's markdown.
#[must_use]
pub fn chapter_content(chapter: &Value) -> Option<&str> {
    chapter.get("content")?.as_str()
}

/// Rewrite every chapter's content in place, in reading order.
///
/// `f` receives the chapter's `src/`-prefixed path and its current markdown,
/// and returns the replacement.
pub fn map_chapters<F>(book: &mut Value, mut f: F)
where
    F: FnMut(&str, &str) -> String,
{
    if let Some(Value::Array(items)) = book.get_mut("sections") {
        walk(items, &mut f);
    }
}

fn walk<F: FnMut(&str, &str) -> String>(items: &mut [Value], f: &mut F) {
    for item in items {
        let Some(chapter) = item.get_mut("Chapter") else {
            continue;
        };
        let path = chapter
            .get("path")
            .and_then(Value::as_str)
            .map(|p| format!("src/{p}"));
        if let (Some(path), Some(content)) = (path, chapter.get("content").and_then(Value::as_str))
        {
            let rewritten = f(&path, content);
            if let Some(slot) = chapter.get_mut("content") {
                *slot = Value::String(rewritten);
            }
        }
        if let Some(Value::Array(subs)) = chapter.get_mut("sub_items") {
            walk(subs, f);
        }
    }
}

/// The book as the kernel consumes it: chapters in reading order, as text.
#[must_use]
pub fn book_source(book: &Value) -> BookSource {
    BookSource::from_chapters(
        chapters(book)
            .into_iter()
            .filter_map(|c| Some(Chapter::new(&chapter_path(c)?, chapter_content(c)?)))
            .collect(),
    )
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod mdbook_tests {
    use super::*;

    fn chapter(name: &str, path: &str, content: &str, subs: &[Value]) -> Value {
        serde_json::json!({
            "Chapter": {
                "name": name,
                "content": content,
                "path": path,
                "sub_items": subs,
            }
        })
    }

    fn book() -> Value {
        serde_json::json!({
            "sections": [
                chapter("One", "ch01.md", "# One\n", &[]),
                { "Separator": null },
                chapter(
                    "Two",
                    "ch02.md",
                    "# Two\n",
                    &[chapter("Two A", "ch02a.md", "# Two A\n", &[])],
                ),
            ]
        })
    }

    #[test]
    fn mdbook__chapters_come_back_in_reading_order() {
        let b = book();
        let paths: Vec<String> = chapters(&b)
            .iter()
            .filter_map(|c| chapter_path(c))
            .collect();
        assert_eq!(paths, vec!["src/ch01.md", "src/ch02.md", "src/ch02a.md"]);
    }

    #[test]
    fn mdbook__nested_sections_are_found() {
        // A sub-chapter is a chapter. Missing one would silently drop every
        // step it declares.
        let b = book();
        assert_eq!(chapters(&b).len(), 3);
    }

    #[test]
    fn mdbook__separators_carry_no_content_and_are_skipped() {
        let b = book();
        assert!(chapters(&b).iter().all(|c| chapter_content(c).is_some()));
    }

    #[test]
    fn mdbook__book_source_matches_the_loader_convention() {
        let src = book_source(&book());
        assert_eq!(src.chapters[0].path, "src/ch01.md");
        assert_eq!(src.chapters[2].text, "# Two A\n");
    }

    #[test]
    fn mdbook__map_chapters_rewrites_nested_chapters_too() {
        let mut b = book();
        map_chapters(&mut b, |path, text| format!("{path}|{text}"));
        let paths: Vec<String> = chapters(&b)
            .iter()
            .filter_map(|c| chapter_content(c).map(str::to_string))
            .collect();
        assert_eq!(paths[0], "src/ch01.md|# One\n");
        assert_eq!(paths[2], "src/ch02a.md|# Two A\n");
    }

    #[test]
    fn split__rejects_anything_that_is_not_a_pair() {
        assert!(split(serde_json::json!({})).is_err());
        assert!(split(serde_json::json!([1, 2, 3])).is_err());
        let ok = split(serde_json::json!([{"root": "."}, {"sections": []}]));
        assert_eq!(ok.unwrap().0.root, PathBuf::from("."));
    }
}
