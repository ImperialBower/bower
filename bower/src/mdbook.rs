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

/// What mdBook calls the book's top-level item array — which is two things.
///
/// mdBook 0.4 sends `sections`; 0.5 renamed it to `items`. Reading only one of
/// the two fails silently, which is the worst way for this to fail: the
/// preprocessor finds no chapters, hands the book back untouched, and mdBook
/// reports a successful build of a book with every directive still on the
/// page. Both names are read, newest first, so one binary works against either
/// mdBook.
const ITEM_KEYS: [&str; 2] = ["items", "sections"];

/// The book's item array, under whichever name this mdBook used.
fn book_items(book: &Value) -> Option<&Value> {
    ITEM_KEYS.iter().find_map(|key| book.get(key))
}

/// The same array, to write through. Split from [`book_items`] because the key
/// has to be chosen with the book borrowed immutably before it can be borrowed
/// mutably to reach the array.
fn book_items_mut(book: &mut Value) -> Option<&mut Value> {
    let key = *ITEM_KEYS.iter().find(|key| book.get(*key).is_some())?;
    book.get_mut(key)
}

/// Every chapter in the book, in reading order.
///
/// A book is a tree: the item array holds items, a `Chapter` item may hold
/// `sub_items`, and only `Chapter` items carry content. Separators and parts
/// carry none and are skipped.
#[must_use]
pub fn chapters(book: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    collect(book_items(book), &mut out);
    out
}

fn collect<'a>(array: Option<&'a Value>, out: &mut Vec<&'a Value>) {
    let Some(Value::Array(items)) = array else {
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
    if let Some(Value::Array(items)) = book_items_mut(book) {
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

    /// The same book as mdBook 0.5 sends it: one key renamed, nothing else.
    fn book_0_5() -> Value {
        let items = book().get("sections").unwrap().clone();
        serde_json::json!({ "items": items })
    }

    #[test]
    fn mdbook__chapters_are_found_under_either_mdbook_s_key() {
        // 0.4 says `sections`, 0.5 says `items`. Reading only one of them is
        // silent — no chapters, book returned untouched, build "successful",
        // directives on the page.
        let paths = |b: &Value| -> Vec<String> {
            chapters(b).iter().filter_map(|c| chapter_path(c)).collect()
        };
        assert_eq!(paths(&book_0_5()), paths(&book()));
        assert_eq!(chapters(&book_0_5()).len(), 3);
    }

    #[test]
    fn mdbook__map_chapters_rewrites_under_either_mdbook_s_key() {
        let mut b = book_0_5();
        map_chapters(&mut b, |path, text| format!("{path}|{text}"));
        assert_eq!(
            chapter_content(chapters(&b)[0]),
            Some("src/ch01.md|# One\n")
        );
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
