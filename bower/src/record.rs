//! `bower verify --record`: writing what the compiler said back into the book.
//!
//! The only code path that edits a chapter. It rewrites fence bodies and
//! nothing else (`bower_core::prelude::rewrite`), and only the fences whose
//! output is missing or has drifted: a fence that still matches is left alone,
//! so an author's `[...]` trim survives every re-record while it holds
//! (EPIC-11 Decision 5). A broken step's output is never written (Decision 6).

use std::collections::BTreeMap;
use std::path::Path;

use bower_core::prelude::rewrite;

use crate::materialize::check_path;
use crate::verify::{OutputResult, VerifyReport};

/// One fence to rewrite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recording {
    /// Book-root-relative, as the loader names chapters: `src/ch04-….md`.
    pub chapter: String,
    /// The output directive's 1-based line.
    pub line: usize,
    /// The normalized live output to write.
    pub lines: Vec<String>,
}

/// Which fences `--record` rewrites: every output of an upheld step that is
/// not recorded yet or has drifted. Nothing else.
#[must_use]
pub fn recordings(reports: &[VerifyReport]) -> Vec<Recording> {
    reports
        .iter()
        .flat_map(VerifyReport::outputs)
        .filter(|(_, o)| {
            matches!(
                o.result,
                OutputResult::NotRecorded | OutputResult::Drifted(_)
            )
        })
        .map(|(_, o)| Recording {
            chapter: o.loc.chapter.clone(),
            line: o.loc.line,
            lines: o.live.clone(),
        })
        .collect()
}

/// Fold the recordings into the chapter texts. Bottom-up within a chapter: a
/// fence that grows moves every line below it, and none above. Returns only
/// the chapters whose text changed.
#[must_use]
pub fn apply(texts: &BTreeMap<String, String>, recs: &[Recording]) -> BTreeMap<String, String> {
    let mut by_chapter: BTreeMap<&str, Vec<&Recording>> = BTreeMap::new();
    for r in recs {
        by_chapter.entry(r.chapter.as_str()).or_default().push(r);
    }
    let mut changed = BTreeMap::new();
    for (chapter, mut list) in by_chapter {
        let Some(original) = texts.get(chapter) else {
            continue;
        };
        list.sort_by_key(|r| std::cmp::Reverse(r.line));
        let text = list
            .iter()
            .fold(original.clone(), |t, r| rewrite(&t, r.line, &r.lines));
        if &text != original {
            changed.insert(chapter.to_string(), text);
        }
    }
    changed
}

/// Write each changed chapter under `book_root`. Every path is checked before
/// anything is written: chapter paths come from the book, which is not trusted
/// to name only its own files (`docs/DEFECT_Path_Traversal.md`).
///
/// # Errors
///
/// An unsafe path (`InvalidInput`), or a failed write.
pub fn write_chapters(book_root: &Path, changed: &BTreeMap<String, String>) -> std::io::Result<()> {
    for path in changed.keys() {
        check_path(path)?;
    }
    for (path, text) in changed {
        std::fs::write(book_root.join(path), text)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod record_tests {
    use super::*;
    use crate::verify::{OutputVerdict, StepVerdict, Verdict};
    use bower_core::prelude::{Capture, Drift, Expect, Location, StepId};

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    fn report(outputs: Vec<(usize, OutputResult, &[&str])>) -> VerifyReport {
        VerifyReport {
            repo: "r".to_string(),
            verdicts: vec![StepVerdict {
                seq: 1,
                id: StepId("s".to_string()),
                anchor: Location::new("src/ch.md", 1),
                expect: Expect::CompileFail,
                verdict: Verdict::Upheld,
                outputs: outputs
                    .into_iter()
                    .map(|(line, result, live)| OutputVerdict {
                        loc: Location::new("src/ch.md", line),
                        capture: Capture::Check,
                        live: v(live),
                        result,
                    })
                    .collect(),
            }],
            unpinned_step: None,
        }
    }

    const CH: &str = concat!(
        "<!-- bower repo=\"r\" output=\"check\" -->\n", // 1
        "```text\n[...]\nerror: kept\n[...]\n```\n",    // 2-6
        "<!-- bower repo=\"r\" output=\"check\" -->\n", // 7
        "```text\nerror: old\n```\n",                   // 8-10
    );

    fn texts() -> BTreeMap<String, String> {
        BTreeMap::from([("src/ch.md".to_string(), CH.to_string())])
    }

    #[test]
    fn record__keeps_a_trim_that_still_matches() {
        let recs = recordings(&[report(vec![(
            1,
            OutputResult::Matches,
            &["a", "error: kept", "b"],
        )])]);
        assert!(recs.is_empty());
        assert!(
            apply(&texts(), &recs).is_empty(),
            "nothing changed, nothing written"
        );
    }

    #[test]
    fn record__replaces_a_drifted_trim_with_the_full_output() {
        let drifted = OutputResult::Drifted(Drift {
            line: 2,
            recorded: Some("error: kept".to_string()),
            live: None,
        });
        let recs = recordings(&[report(vec![(1, drifted, &["a", "error: new", "b"])])]);
        let changed = apply(&texts(), &recs);
        assert!(
            changed["src/ch.md"].starts_with(
                "<!-- bower repo=\"r\" output=\"check\" -->\n```text\na\nerror: new\nb\n```\n"
            ),
            "{changed:?}"
        );
    }

    #[test]
    fn record__never_writes_an_unjudged_output() {
        assert!(recordings(&[report(vec![(7, OutputResult::Unjudged, &[])])]).is_empty());
    }

    #[test]
    fn record__rewrites_bottom_up_so_both_fences_land() {
        let recs = recordings(&[report(vec![
            (1, OutputResult::NotRecorded, &["one", "two", "three"]),
            (7, OutputResult::NotRecorded, &["four"]),
        ])]);
        let changed = apply(&texts(), &recs);
        assert_eq!(
            changed["src/ch.md"],
            concat!(
                "<!-- bower repo=\"r\" output=\"check\" -->\n```text\none\ntwo\nthree\n```\n",
                "<!-- bower repo=\"r\" output=\"check\" -->\n```text\nfour\n```\n",
            )
        );
    }

    #[test]
    fn record__refuses_a_chapter_path_that_climbs_out() {
        let dir = std::env::temp_dir()
            .join("bower-record-escape")
            .join("book");
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        let changed = BTreeMap::from([("../ESCAPED.md".to_string(), "no".to_string())]);
        let err = write_chapters(&dir, &changed).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(!dir.parent().unwrap().join("ESCAPED.md").exists());
    }
}
