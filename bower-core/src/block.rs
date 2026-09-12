//! Block extraction: walking chapter markdown, pairing each directive with
//! its fenced code block, resolving `include=` against the block library,
//! and validating required keys against the repo catalog.
//!
//! The scanner is fence-aware: a line that *looks* like a directive inside
//! someone else's code block (say, a chapter of the book showing Bower's
//! own syntax) is never parsed as one.

use crate::directive::{Capture, Directive, Expect, Op};
use crate::source::{BookSource, Chapter, Location, RepoCatalog, RepoName};
use crate::{BowerError, Errors};

/// The raw fenced block a directive captured: its info string (`rust`,
/// `rust,ignore`, …) and its lines, fence delimiters excluded.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlockContent {
    pub info: String,
    pub lines: Vec<String>,
}

/// One fully-resolved annotated block: directive defaults applied, include
/// resolved, keys validated. This is the unit [`crate::step`] groups into
/// steps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub loc: Location,
    pub repo: RepoName,
    pub op: Op,
    pub expect: Option<Expect>,
    pub hidden: bool,
    pub file: Option<String>,
    pub paths: Vec<String>,
    pub region: Option<String>,
    pub src: Option<String>,
    pub show: Option<String>,
    pub step: Option<String>,
    pub msg: Option<String>,
    pub after: Option<String>,
    /// Nearest preceding heading text, used to derive commit subjects.
    pub heading: Option<String>,
    pub chapter_stem: String,
    pub content: BlockContent,
    /// Global document order across the whole book, 0-based.
    pub seq_in_book: usize,
    /// `notebook="play"`: a live notebook cell (§ 15). Play blocks never
    /// join steps or touch trees; [`crate::plan`] binds them to steps.
    pub play: bool,
    /// `exercise="…"`: the task text, on either form.
    pub exercise: Option<String>,
    /// The block form: an `exercise` directive with no tree keys, whose fence
    /// is the exercise's detail. Like a play cell it never joins a step or
    /// touches a tree; [`crate::plan`] binds it.
    pub exercise_block: bool,
    /// `output="…"`: the fence records what that command printed at the bound
    /// step. Like a play cell it never joins a step or touches a tree;
    /// [`crate::plan`] binds it (EPIC-11).
    pub output: Option<Capture>,
}

/// Extract every annotated block from the book, in document order.
/// Bad directives are reported and skipped; the scan always finishes.
#[must_use]
pub fn extract(book: &BookSource, catalog: &RepoCatalog) -> (Vec<Block>, Errors) {
    let mut blocks = Vec::new();
    let mut errors = Errors::default();
    let mut seq = 0_usize;

    for chapter in &book.chapters {
        scan_chapter(chapter, book, catalog, &mut blocks, &mut errors, &mut seq);
    }
    (blocks, errors)
}

fn scan_chapter(
    chapter: &Chapter,
    book: &BookSource,
    catalog: &RepoCatalog,
    blocks: &mut Vec<Block>,
    errors: &mut Errors,
    seq: &mut usize,
) {
    let lines: Vec<&str> = chapter.text.lines().collect();
    let mut heading: Option<String> = None;
    let mut i = 0_usize;

    while i < lines.len() {
        let line = lines[i];

        if let Some(width) = fence_width(line) {
            // A fence not owned by a directive: skip its body entirely.
            i = skip_fence(&lines, i, width);
            continue;
        }

        if let Some(h) = heading_text(line) {
            heading = Some(h);
            i += 1;
            continue;
        }

        if Directive::is_directive_line(line) {
            let loc = Location::new(&chapter.path, i + 1);
            match Directive::parse(line, &loc) {
                Err(errs) => errors.extend(errs),
                Ok(directive) => {
                    let (content, next) = capture_block(&lines, i + 1, &loc, errors);
                    if let Some(block) = resolve(
                        directive,
                        content,
                        &loc,
                        chapter,
                        book,
                        catalog,
                        heading.clone(),
                        *seq,
                        errors,
                    ) {
                        blocks.push(block);
                        *seq += 1;
                    }
                    i = next;
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// If the line opens a fence, return its backtick count.
fn fence_width(line: &str) -> Option<usize> {
    let t = line.trim_start();
    let count = t.chars().take_while(|&c| c == '`').count();
    (count >= 3).then_some(count)
}

/// Given the index of an opening fence, return the index just past its
/// closing fence (`CommonMark`: closer must be at least as wide).
fn skip_fence(lines: &[&str], open: usize, width: usize) -> usize {
    let mut j = open + 1;
    while j < lines.len() {
        if fence_width(lines[j]).is_some_and(|w| w >= width) {
            return j + 1;
        }
        j += 1;
    }
    lines.len()
}

fn heading_text(line: &str) -> Option<String> {
    let t = line.trim_start();
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &t[hashes..];
    rest.strip_prefix(' ').map(|r| r.trim().to_string())
}

/// Capture the fenced block following a directive, skipping blank lines
/// between directive and fence. Returns the content (None if there is no
/// fence) and the index to resume scanning at.
fn capture_block(
    lines: &[&str],
    from: usize,
    loc: &Location,
    errors: &mut Errors,
) -> (Option<BlockContent>, usize) {
    let mut j = from;
    while j < lines.len() && lines[j].trim().is_empty() {
        j += 1;
    }
    let Some(&opener) = lines.get(j) else {
        return (None, from);
    };
    let Some(width) = fence_width(opener) else {
        return (None, from);
    };
    let info = opener
        .trim_start()
        .trim_start_matches('`')
        .trim()
        .to_string();
    let mut body = Vec::new();
    let mut k = j + 1;
    while k < lines.len() {
        if fence_width(lines[k]).is_some_and(|w| w >= width) {
            return (Some(BlockContent { info, lines: body }), k + 1);
        }
        body.push(lines[k].to_string());
        k += 1;
    }
    errors.push(BowerError::UnclosedFence { loc: loc.clone() });
    (None, lines.len())
}

/// Does the directive say anything about a repo tree? Play cells and
/// block-form exercises must not; every other directive must.
fn carries_tree_keys(d: &Directive) -> bool {
    d.op.is_some()
        || d.file.is_some()
        || d.region.is_some()
        || d.src.is_some()
        || !d.paths.is_empty()
}

/// A tree block's op decides which other keys it needs: a file (or paths),
/// a region for `region`, a source for `copy`, and a fence for the ops that
/// write text.
fn require_op_keys(
    directive: &Directive,
    op: Op,
    has_content: bool,
    loc: &Location,
    errors: &mut Errors,
) {
    if op.needs_file() && directive.file.is_none() && directive.paths.is_empty() {
        errors.push(BowerError::MissingKey {
            loc: loc.clone(),
            key: "file".to_string(),
        });
    }
    if op == Op::Region && directive.region.is_none() {
        errors.push(BowerError::MissingKey {
            loc: loc.clone(),
            key: "region".to_string(),
        });
    }
    if op == Op::Copy && directive.src.is_none() {
        errors.push(BowerError::MissingKey {
            loc: loc.clone(),
            key: "src".to_string(),
        });
    }
    if op.needs_block() && !has_content {
        errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
    }
}

/// Merge include defaults, validate keys against the catalog and the op's
/// requirements, and produce a [`Block`] — or report why not.
#[allow(clippy::too_many_arguments)] // internal seam; the tuple would be worse
fn resolve(
    mut directive: Directive,
    mut content: Option<BlockContent>,
    loc: &Location,
    chapter: &Chapter,
    book: &BookSource,
    catalog: &RepoCatalog,
    heading: Option<String>,
    seq_in_book: usize,
    errors: &mut Errors,
) -> Option<Block> {
    // Read before the include is resolved: the overlay clears it.
    let included = directive.include.is_some();
    if let Some(path) = directive.include.clone() {
        match resolve_include(&path, loc, book, errors) {
            Some((defaults, lib_content)) => {
                directive = directive.overlaid_on(&defaults);
                if content.is_none() {
                    content = Some(lib_content);
                }
            }
            None => return None,
        }
    }

    let before = errors.len();

    let repo = match &directive.repo {
        Some(r) if catalog.contains(r) => RepoName::new(r),
        Some(r) => {
            errors.push(BowerError::UnknownRepo {
                loc: loc.clone(),
                repo: r.clone(),
            });
            RepoName::new(r)
        }
        None => {
            errors.push(BowerError::MissingKey {
                loc: loc.clone(),
                key: "repo".to_string(),
            });
            RepoName::new("")
        }
    };

    let play = directive.notebook.is_some();
    let tree_keys = carries_tree_keys(&directive);
    let output = directive.output;
    let exercise_block = directive.exercise.is_some() && !tree_keys && !play && output.is_none();
    let op = directive.op.unwrap_or_default();

    if output.is_some() {
        // An output block quotes what a command printed at a step. It names
        // its step and nothing else: not a tree, a cell, an exercise, or a
        // claim — and not an include, because `--record` writes into the
        // chapter and a library entry is not one.
        if tree_keys
            || play
            || directive.exercise.is_some()
            || directive.expect.is_some()
            || included
        {
            errors.push(BowerError::OutputConflictingKeys { loc: loc.clone() });
        }
        if content.is_none() {
            errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
        }
    } else if play {
        // A play cell is a notebook concern: it needs its code block and
        // must not carry anything that would touch a repo tree — nor be an
        // exercise, which is a different kind of aside.
        if tree_keys {
            errors.push(BowerError::PlayCellConflictingKeys { loc: loc.clone() });
        }
        if directive.exercise.is_some() {
            errors.push(BowerError::ExerciseConflictingKeys { loc: loc.clone() });
        }
        if content.is_none() {
            errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
        }
    } else if exercise_block {
        // The fence *is* the exercise's detail. Without one there is nothing
        // the key form would not have said shorter.
        if content.is_none() {
            errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
        }
    } else {
        require_op_keys(&directive, op, content.is_some(), loc, errors);
    }

    if errors.len() > before {
        return None;
    }

    Some(Block {
        loc: loc.clone(),
        repo,
        // Neither a play cell, a block-form exercise, nor an output applies
        // to a tree; Prose is the inert op.
        op: if play || exercise_block || output.is_some() {
            Op::Prose
        } else {
            op
        },
        expect: directive.expect,
        hidden: directive.hidden.unwrap_or(true),
        file: directive.file,
        paths: directive.paths,
        region: directive.region,
        src: directive.src,
        show: directive.show,
        step: directive.step,
        msg: directive.msg,
        after: directive.after,
        heading,
        chapter_stem: chapter.stem().to_string(),
        content: content.unwrap_or_default(),
        seq_in_book,
        play,
        exercise: directive.exercise,
        exercise_block,
        output,
    })
}

/// A library entry is a mini-document: an optional directive (defaults)
/// followed by exactly one fenced block.
fn resolve_include(
    path: &str,
    loc: &Location,
    book: &BookSource,
    errors: &mut Errors,
) -> Option<(Directive, BlockContent)> {
    let Some(text) = book.library.get(path) else {
        errors.push(BowerError::IncludeMissing {
            loc: loc.clone(),
            path: path.to_string(),
        });
        return None;
    };
    let lines: Vec<&str> = text.lines().collect();
    let mut defaults = Directive::default();
    let mut i = 0_usize;
    while i < lines.len() {
        let line = lines[i];
        if Directive::is_directive_line(line) {
            match Directive::parse(line, loc) {
                Ok(d) => defaults = d,
                Err(errs) => {
                    errors.extend(errs);
                    return None;
                }
            }
            i += 1;
            continue;
        }
        if fence_width(line).is_some() {
            let mut scratch = Errors::default();
            let (content, _next) = capture_block(&lines, i, loc, &mut scratch);
            errors.extend(scratch);
            if let Some(c) = content {
                return Some((defaults, c));
            }
            errors.push(BowerError::IncludeMalformed {
                loc: loc.clone(),
                path: path.to_string(),
                reason: "fence never closes".to_string(),
            });
            return None;
        }
        i += 1;
    }
    errors.push(BowerError::IncludeMalformed {
        loc: loc.clone(),
        path: path.to_string(),
        reason: "no fenced code block found".to_string(),
    });
    None
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod block_tests {
    use super::*;

    fn catalog() -> RepoCatalog {
        RepoCatalog::from_names(&["failers"])
    }

    fn book(text: &str) -> BookSource {
        BookSource::from_chapters(vec![Chapter::new("ch01.md", text)])
    }

    #[test]
    fn extract__pairs_directive_with_block_and_heading() {
        let src = book(
            "# Ranks\n\n<!-- bower repo=\"failers\" file=\"src/rank.rs\" -->\n```rust\npub enum Rank {}\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(b.heading.as_deref(), Some("Ranks"));
        assert_eq!(b.op, Op::Create);
        assert_eq!(b.content.lines, vec!["pub enum Rank {}".to_string()]);
        assert_eq!(b.loc.line, 3);
    }

    #[test]
    fn extract__directive_inside_foreign_fence_is_ignored() {
        let src = book("```markdown\n<!-- bower repo=\"failers\" file=\"x\" -->\n```\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        assert!(blocks.is_empty());
    }

    #[test]
    fn extract__unknown_repo_reported_block_skipped() {
        let src = book("<!-- bower repo=\"typo\" file=\"x\" -->\n```rust\n```\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(matches!(errors.0[0], BowerError::UnknownRepo { .. }));
    }

    #[test]
    fn extract__op_needing_block_without_one_errors() {
        let src = book("<!-- bower repo=\"failers\" file=\"x\" -->\nno fence here\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(matches!(
            errors.0[0],
            BowerError::DirectiveWithoutBlock { .. }
        ));
    }

    #[test]
    fn extract__prose_op_needs_no_block_or_file() {
        let src = book("<!-- bower repo=\"failers\" op=\"none\" msg=\"narrated refactor\" -->\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(blocks[0].op, Op::Prose);
    }

    #[test]
    fn extract__exercise_key_rides_on_a_tree_block() {
        let src = book(
            "<!-- bower repo=\"failers\" file=\"a.rs\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n```rust\nx\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        let b = &blocks[0];
        assert_eq!(b.exercise.as_deref(), Some("Make this compile"));
        assert!(!b.exercise_block);
        assert_eq!(b.op, Op::Create);
    }

    #[test]
    fn extract__exercise_without_tree_keys_is_the_block_form() {
        let src = book(
            "<!-- bower repo=\"failers\" exercise=\"Try a lookup table\" -->\n```markdown\n- one idea\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        let b = &blocks[0];
        assert!(b.exercise_block);
        assert_eq!(b.exercise.as_deref(), Some("Try a lookup table"));
        assert_eq!(b.op, Op::Prose);
        assert_eq!(b.content.lines, vec!["- one idea".to_string()]);
    }

    #[test]
    fn extract__block_form_needs_its_fence() {
        let src = book("<!-- bower repo=\"failers\" exercise=\"Try it\" -->\nprose instead\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(matches!(
            errors.0[0],
            BowerError::DirectiveWithoutBlock { .. }
        ));
    }

    #[test]
    fn extract__a_play_cell_cannot_also_be_an_exercise() {
        let src = book(
            "<!-- bower repo=\"failers\" notebook=\"play\" exercise=\"Try it\" -->\n```python\nx\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(matches!(
            errors.0[0],
            BowerError::ExerciseConflictingKeys { .. }
        ));
    }

    #[test]
    fn extract__include_pulls_content_and_defaults() {
        let mut src = book("<!-- bower include=\"blocks/rank.md\" msg=\"override\" -->\n");
        src.library.insert(
            "blocks/rank.md".to_string(),
            "<!-- bower repo=\"failers\" file=\"src/rank.rs\" msg=\"default\" -->\n```rust\npub enum Rank {}\n```\n"
                .to_string(),
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        let b = &blocks[0];
        assert_eq!(b.file.as_deref(), Some("src/rank.rs"));
        assert_eq!(b.msg.as_deref(), Some("override"));
        assert_eq!(b.content.lines, vec!["pub enum Rank {}".to_string()]);
    }

    #[test]
    fn extract__include_missing_is_an_error() {
        let src = book("<!-- bower include=\"blocks/nope.md\" -->\n");
        let (_, errors) = extract(&src, &catalog());
        assert!(matches!(errors.0[0], BowerError::IncludeMissing { .. }));
    }

    #[test]
    fn extract__wide_fence_can_hold_narrow_fences() {
        let src = book(
            "<!-- bower repo=\"failers\" file=\"README.md\" -->\n````markdown\n```rust\ninner\n```\n````\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(blocks[0].content.lines.len(), 3);
    }

    #[test]
    fn extract__unclosed_fence_reported() {
        let src = book("<!-- bower repo=\"failers\" file=\"x\" -->\n```rust\nnever closes\n");
        let (_, errors) = extract(&src, &catalog());
        assert!(matches!(errors.0[0], BowerError::UnclosedFence { .. }));
    }

    #[test]
    fn extract__output_block_is_inert() {
        let src = book(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" output=\"check\" -->\n```text\nerror: e\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        let b = &blocks[1];
        assert_eq!(b.output, Some(Capture::Check));
        assert_eq!(b.op, Op::Prose);
        assert_eq!(b.content.lines, vec!["error: e".to_string()]);
    }

    #[test]
    fn extract__output_needs_its_fence() {
        let src = book("<!-- bower repo=\"failers\" output=\"check\" -->\nprose instead\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(
            matches!(errors.0[0], BowerError::DirectiveWithoutBlock { .. }),
            "{errors}"
        );
    }

    #[test]
    fn extract__output_with_tree_keys_is_refused() {
        for keys in [
            "file=\"a.rs\"",
            "op=\"none\"",
            "expect=\"pass\"",
            "notebook=\"play\"",
            "exercise=\"Try\"",
        ] {
            let src = book(&format!(
                "<!-- bower repo=\"failers\" output=\"check\" {keys} -->\n```text\nx\n```\n"
            ));
            let (blocks, errors) = extract(&src, &catalog());
            assert!(blocks.is_empty(), "{keys}");
            assert!(
                errors
                    .0
                    .iter()
                    .any(|e| matches!(e, BowerError::OutputConflictingKeys { .. })),
                "{keys}: {errors}"
            );
        }
    }

    #[test]
    fn extract__output_with_include_is_refused() {
        // `--record` writes into the chapter, and a library entry is not one.
        let mut src = book("<!-- bower include=\"blocks/out.md\" output=\"check\" -->\n");
        src.library.insert(
            "blocks/out.md".to_string(),
            "<!-- bower repo=\"failers\" -->\n```text\nx\n```\n".to_string(),
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(
            matches!(errors.0[0], BowerError::OutputConflictingKeys { .. }),
            "{errors}"
        );
    }
}
