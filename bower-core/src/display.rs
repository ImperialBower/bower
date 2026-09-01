//! The display engine (spec § 3.4): which part of a block the rendered
//! book shows, and where exactly those lines live in the materialized
//! tree — the data behind line-anchored source links.
//!
//! Display is orthogonal to assembly: [`crate::tree::assemble`] decides
//! what is in the repo file; this module decides what is on the page.

use crate::block::Block;
use crate::source::Location;
use crate::tree::{self, Hidden, ShowMark, TreeState};
use crate::{BowerError, Errors};

/// One span of a block the book renders. `repo_lines` are the span's lines
/// as they appear in the repo file (the basis for line mapping);
/// `visible_lines` are the ones actually printed (hidden lines inside a
/// span stay behind the toggle).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DisplaySpan {
    pub name: Option<String>,
    pub repo_lines: Vec<String>,
    pub visible_lines: Vec<String>,
}

impl DisplaySpan {
    /// The span's lines as they appear in the *materialized* tree — the
    /// exact text a resolved [`LineRange`] covers. When the repo strips
    /// region markers, marker lines inside the span are excluded here too.
    #[must_use]
    pub fn mapped_lines(&self, keep_region_markers: bool) -> Vec<&str> {
        self.repo_lines
            .iter()
            .map(String::as_str)
            .filter(|l| keep_region_markers || tree::region_marker(l).is_none())
            .collect()
    }
}

/// A 1-based, inclusive line range in a materialized file — the anchor for
/// `…/blob/{tag}/{path}#L{start}-L{end}` links.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LineRange {
    pub file: String,
    pub start: usize,
    pub end: usize,
}

/// The display outcome for one block: the spans that render, how much was
/// elided, and (after [`resolve_ranges`]) where each span sits in the tree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlockDisplay {
    pub loc: Location,
    pub file: Option<String>,
    pub spans: Vec<DisplaySpan>,
    /// Repo lines the page does not show.
    pub elided_lines: usize,
    /// Parallel to `spans`; `None` for spans with nothing mappable
    /// (e.g. an all-hidden span in a `hidden="false"` block).
    pub ranges: Vec<Option<LineRange>>,
}

struct Record {
    /// The line as it appears in the repo file, if it appears at all.
    repo: Option<String>,
    /// Whether the page prints it.
    visible: bool,
}

/// Compute the spans a block displays. Marker errors (unclosed, nested,
/// orphan end, unknown `show=` name) are collected; the analysis still
/// returns its best effort so one bad marker doesn't hide the rest.
#[must_use]
pub fn analyze(block: &Block, errors: &mut Errors) -> BlockDisplay {
    let rustish = block.content.info.split([',', ' ']).next() == Some("rust");

    let mut all: Vec<DisplaySpan> = Vec::new();
    let mut open: Option<DisplaySpan> = None;
    let mut outside: Vec<Record> = Vec::new();
    let mut saw_marker = false;

    for raw in &block.content.lines {
        // Hidden-line processing first, mirroring assembly.
        let (repo, visible) = if rustish {
            match tree::hidden_line(raw) {
                Hidden::No => (Some(raw.clone()), true),
                Hidden::Yes(content) => {
                    if block.hidden {
                        (Some(content), false)
                    } else {
                        (None, false)
                    }
                }
            }
        } else {
            (Some(raw.clone()), true)
        };

        let marker_probe = repo.as_deref().unwrap_or(raw.as_str());
        if let Some(mark) = tree::show_marker(marker_probe) {
            saw_marker = true;
            match mark {
                ShowMark::Begin(name) => {
                    if open.is_some() {
                        errors.push(BowerError::NestedShowSpan {
                            loc: block.loc.clone(),
                        });
                    } else {
                        open = Some(DisplaySpan {
                            name,
                            ..DisplaySpan::default()
                        });
                    }
                }
                ShowMark::End => match open.take() {
                    Some(span) => all.push(span),
                    None => errors.push(BowerError::OrphanShowEnd {
                        loc: block.loc.clone(),
                    }),
                },
            }
            continue;
        }

        let record = Record { repo, visible };
        if let Some(span) = &mut open {
            if let Some(line) = &record.repo {
                span.repo_lines.push(line.clone());
                if record.visible {
                    span.visible_lines.push(line.clone());
                }
            }
        } else {
            outside.push(record);
        }
    }

    if open.is_some() {
        errors.push(BowerError::UnclosedShowSpan {
            loc: block.loc.clone(),
        });
    }

    let mut display = BlockDisplay {
        loc: block.loc.clone(),
        file: block.file.clone(),
        ..BlockDisplay::default()
    };

    if saw_marker {
        display.elided_lines = outside.iter().filter(|r| r.repo.is_some()).count();
        display.spans = filter_by_show(block, all, &mut display.elided_lines, errors);
    } else {
        // No markers: the whole block is one span. Small examples pay no tax.
        let mut span = DisplaySpan::default();
        for r in outside {
            if let Some(line) = r.repo {
                if r.visible {
                    span.visible_lines.push(line.clone());
                }
                span.repo_lines.push(line);
            }
        }
        display.spans = vec![span];
    }

    display.ranges = vec![None; display.spans.len()];
    display
}

/// Apply a `show="a,b"` filter: keep only the named spans, elide the rest,
/// and report names that match nothing.
fn filter_by_show(
    block: &Block,
    spans: Vec<DisplaySpan>,
    elided: &mut usize,
    errors: &mut Errors,
) -> Vec<DisplaySpan> {
    let Some(show) = &block.show else {
        return spans;
    };
    let wanted: Vec<&str> = show
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    for name in &wanted {
        if !spans.iter().any(|s| s.name.as_deref() == Some(*name)) {
            errors.push(BowerError::UnknownShowSpan {
                loc: block.loc.clone(),
                name: (*name).to_string(),
            });
        }
    }
    let (kept, dropped): (Vec<_>, Vec<_>) = spans
        .into_iter()
        .partition(|s| s.name.as_deref().is_some_and(|n| wanted.contains(&n)));
    *elided += dropped.iter().map(|s| s.repo_lines.len()).sum::<usize>();
    kept
}

/// Locate each span in the materialized tree, filling `ranges`. A span
/// that cannot be found is a kernel invariant violation and is reported,
/// never invented.
pub fn resolve_ranges(
    display: &mut BlockDisplay,
    tree: &TreeState,
    keep_region_markers: bool,
    step: &str,
    errors: &mut Errors,
) {
    let Some(file) = display.file.clone() else {
        return;
    };
    let Some(text) = tree.text(&file) else {
        return; // deleted later in the step, or binary; nothing to anchor
    };
    let file_lines: Vec<&str> = text.lines().collect();

    for (i, span) in display.spans.iter().enumerate() {
        let needle = span.mapped_lines(keep_region_markers);
        if needle.is_empty() {
            continue;
        }
        match find_window(&file_lines, &needle) {
            Some(start) => {
                display.ranges[i] = Some(LineRange {
                    file: file.clone(),
                    start: start + 1,
                    end: start + needle.len(),
                });
            }
            None => errors.push(BowerError::SpanNotInTree {
                loc: display.loc.clone(),
                step: step.to_string(),
                file: file.clone(),
            }),
        }
    }
}

/// First index where `needle` appears as consecutive lines of `hay`.
fn find_window(hay: &[&str], needle: &[&str]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod display_tests {
    use super::*;
    use crate::block::BlockContent;
    use crate::directive::Op;
    use crate::source::RepoName;

    fn block(lines: &[&str]) -> Block {
        Block {
            loc: Location::new("ch01.md", 1),
            repo: RepoName::new("failers"),
            op: Op::Create,
            expect: None,
            hidden: true,
            file: Some("src/rank.rs".to_string()),
            paths: vec![],
            region: None,
            src: None,
            show: None,
            step: None,
            msg: None,
            after: None,
            heading: None,
            chapter_stem: "ch01".to_string(),
            content: BlockContent {
                info: "rust".to_string(),
                lines: lines.iter().map(ToString::to_string).collect(),
            },
            seq_in_book: 0,
            play: false,
        }
    }

    #[test]
    fn analyze__no_markers_whole_block_renders() {
        let mut errors = Errors::default();
        let d = analyze(&block(&["a", "b"]), &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(d.spans.len(), 1);
        assert_eq!(
            d.spans[0].visible_lines,
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(d.elided_lines, 0);
    }

    #[test]
    fn analyze__markers_elide_the_unmarked() {
        let mut errors = Errors::default();
        let d = analyze(
            &block(&[
                "hidden_from_page",
                "// bower:show",
                "shown",
                "// bower:show end",
                "also_elided",
            ]),
            &mut errors,
        );
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(d.spans.len(), 1);
        assert_eq!(d.spans[0].visible_lines, vec!["shown".to_string()]);
        assert_eq!(d.elided_lines, 2);
    }

    #[test]
    fn analyze__hidden_lines_inside_span_stay_untoggled_but_map() {
        let mut errors = Errors::default();
        let d = analyze(
            &block(&[
                "// bower:show",
                "# use std::fmt;",
                "shown",
                "// bower:show end",
            ]),
            &mut errors,
        );
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(
            d.spans[0].repo_lines,
            vec!["use std::fmt;".to_string(), "shown".to_string()]
        );
        assert_eq!(d.spans[0].visible_lines, vec!["shown".to_string()]);
    }

    #[test]
    fn analyze__named_spans_filtered_by_show_key() {
        let mut errors = Errors::default();
        let mut b = block(&[
            "// bower:show begin one",
            "first",
            "// bower:show end",
            "// bower:show begin two",
            "second",
            "// bower:show end",
        ]);
        b.show = Some("two".to_string());
        let d = analyze(&b, &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(d.spans.len(), 1);
        assert_eq!(d.spans[0].name.as_deref(), Some("two"));
        assert_eq!(d.elided_lines, 1);
    }

    #[test]
    fn analyze__unknown_show_name_is_an_error() {
        let mut errors = Errors::default();
        let mut b = block(&["// bower:show begin one", "x", "// bower:show end"]);
        b.show = Some("ghost".to_string());
        let _ = analyze(&b, &mut errors);
        assert!(matches!(errors.0[0], BowerError::UnknownShowSpan { .. }));
    }

    #[test]
    fn analyze__unclosed_nested_orphan_all_reported() {
        let mut errors = Errors::default();
        let _ = analyze(&block(&["// bower:show end"]), &mut errors);
        let _ = analyze(&block(&["// bower:show", "// bower:show"]), &mut errors);
        let _ = analyze(&block(&["// bower:show", "x"]), &mut errors);
        assert!(errors
            .0
            .iter()
            .any(|e| matches!(e, BowerError::OrphanShowEnd { .. })));
        assert!(errors
            .0
            .iter()
            .any(|e| matches!(e, BowerError::NestedShowSpan { .. })));
        assert!(errors
            .0
            .iter()
            .any(|e| matches!(e, BowerError::UnclosedShowSpan { .. })));
    }

    #[test]
    fn resolve_ranges__finds_one_based_inclusive_range() {
        let mut errors = Errors::default();
        let b = block(&["header", "// bower:show", "target", "// bower:show end"]);
        let mut d = analyze(&b, &mut errors);
        let mut tree = TreeState::default();
        tree.0.insert(
            "src/rank.rs".to_string(),
            crate::tree::FileBody::Text("header\ntarget\n".to_string()),
        );
        resolve_ranges(&mut d, &tree, false, "s", &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(
            d.ranges[0],
            Some(LineRange {
                file: "src/rank.rs".to_string(),
                start: 2,
                end: 2
            })
        );
    }

    #[test]
    fn resolve_ranges__missing_span_is_reported_not_invented() {
        let mut errors = Errors::default();
        let b = block(&["// bower:show", "not_in_tree", "// bower:show end"]);
        let mut d = analyze(&b, &mut errors);
        let mut tree = TreeState::default();
        tree.0.insert(
            "src/rank.rs".to_string(),
            crate::tree::FileBody::Text("something else\n".to_string()),
        );
        resolve_ranges(&mut d, &tree, false, "s", &mut errors);
        assert!(matches!(errors.0[0], BowerError::SpanNotInTree { .. }));
    }
}
