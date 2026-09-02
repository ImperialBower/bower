//! Rewriting a chapter for the rendered book (spec § 5.4).
//!
//! Three jobs: remove the `<!-- bower … -->` directives, apply the display
//! markers so a block prints only what it marks, and inject a `#step-<id>`
//! anchor so every commit trailer's `Book-Source` link lands on the right
//! paragraph.
//!
//! The marker grammar is the kernel's (`bower_core::prelude::show_marker`), not
//! a second copy. Fence syntax is `CommonMark`'s and is recognized locally.

use std::collections::BTreeMap;

use bower_core::prelude::{
    show_marker, BlockDisplay, BookPlan, Directive, LineRange, PlannedStep, ShowMark,
};

use crate::config::LinkTemplates;

/// Rewrite one chapter's markdown.
#[must_use]
pub fn chapter(
    text: &str,
    chapter_path: &str,
    plan: &BookPlan,
    forge: &BTreeMap<String, LinkTemplates>,
) -> String {
    let anchors = anchors_by_line(plan, chapter_path);
    let blocks = blocks_by_line(plan, chapter_path);
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // A fence not opened by a directive is ordinary content — including a
        // fence whose body *shows* a directive, which a book about Bower is
        // full of. Copy it through untouched.
        if let Some(width) = fence_width(line) {
            out.push((*line).to_string());
            i += 1;
            while i < lines.len() {
                let inner = lines[i];
                out.push((*inner).to_string());
                i += 1;
                if fence_width(inner).is_some_and(|w| w >= width) {
                    break;
                }
            }
            continue;
        }

        if !Directive::is_directive_line(line) {
            out.push((*line).to_string());
            i += 1;
            continue;
        }

        // An anchor for the step this directive opens, so `#step-<id>` in a
        // commit trailer resolves on the rendered page. Steps whose anchor is
        // some other block get none — one step, one anchor.
        // Fixed before `i` moves: every later lookup uses this, not arithmetic
        // on a cursor that has since advanced.
        let directive_line = i + 1;

        if let Some(id) = anchors.get(&directive_line) {
            out.push(format!("<a id=\"step-{id}\"></a>"));
            // Only add a separator if the book has not already left one.
            if !lines.get(i + 1).is_some_and(|l| l.trim().is_empty()) {
                out.push(String::new());
            }
        }
        i += 1; // the directive itself never reaches the page

        // A fence may follow, after blank lines. `op="none"` and `op="delete"`
        // directives have none, and prose simply resumes.
        let mut j = i;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        let Some(width) = fence_width(lines.get(j).copied().unwrap_or_default()) else {
            continue;
        };

        for line in lines.iter().take(j).skip(i) {
            out.push((*line).to_string());
        }
        let info = fence_info(lines[j]);
        out.push(lines[j].to_string());

        let mut k = j + 1;
        let mut body: Vec<String> = Vec::new();
        while k < lines.len() && fence_width(lines[k]).is_none_or(|w| w < width) {
            body.push(lines[k].to_string());
            k += 1;
        }
        out.extend(body_lines(&info, &body));
        if k < lines.len() {
            out.push(lines[k].to_string());
        }
        // The footer belongs after the closing fence: it describes the block,
        // and inside the fence it would be code.
        if let Some((repo, step, display)) = blocks.get(&directive_line) {
            // A chapter may feed several repos, and each declares its own forge
            // templates. A repo with none gets a footer with plain text where
            // the links would be, never someone else's URLs.
            let empty = LinkTemplates::default();
            let repo_links = forge.get(repo).unwrap_or(&empty);
            if let Some(line) = footer(step, display, repo, repo_links) {
                out.push(String::new());
                out.push(line);
            }
        }
        i = k + 1;
    }

    let mut rendered = out.join("\n");
    rendered.push('\n');
    rendered
}

/// The lines a block contributes to the page.
///
/// With no display markers the block is returned untouched — the no-tax path
/// of spec § 3.4, and the case most blocks are in.
///
/// With markers, marked spans print verbatim and the rest is elided. *How* it
/// is elided depends on the language, because mdBook's hidden-line toggle only
/// exists for Rust: a `rust` fence gets `# `-prefixed lines the reader can
/// expand in place, and every other language gets a single comment line saying
/// how much was left out. Emitting `# ` into a Makefile would not hide
/// anything; it would corrupt it.
#[must_use]
pub fn body_lines(info: &str, raw: &[String]) -> Vec<String> {
    if !raw.iter().any(|l| show_marker(l).is_some()) {
        return raw.to_vec();
    }

    let rustish = info.split([',', ' ']).next() == Some("rust");
    let comment = comment_token(info);

    let mut out = Vec::with_capacity(raw.len());
    let mut showing = false;
    let mut elided = 0_usize;

    let flush = |out: &mut Vec<String>, elided: &mut usize| {
        if *elided > 0 && !rustish {
            let plural = if *elided == 1 { "line" } else { "lines" };
            out.push(format!("{comment} ⋯ {elided} {plural} elided"));
        }
        *elided = 0;
    };

    for line in raw {
        match show_marker(line) {
            Some(ShowMark::Begin(_)) => {
                flush(&mut out, &mut elided);
                showing = true;
            }
            Some(ShowMark::End) => {
                showing = false;
            }
            None => {
                if showing {
                    flush(&mut out, &mut elided);
                    out.push(line.clone());
                } else if rustish {
                    out.push(hide(line));
                } else {
                    elided += 1;
                }
            }
        }
    }
    flush(&mut out, &mut elided);
    out
}

/// mdBook's hidden-line form: `# ` before the content, indentation preserved.
fn hide(line: &str) -> String {
    if line.trim().is_empty() {
        return "#".to_string();
    }
    let indent = line.len() - line.trim_start().len();
    format!("{}# {}", &line[..indent], &line[indent..])
}

/// The line-comment token for a fence's language. `#` is the default because
/// it covers the shell, Make, YAML, and TOML blocks these books are full of.
fn comment_token(info: &str) -> &'static str {
    match info.split([',', ' ']).next().unwrap_or_default() {
        "rust" | "c" | "cpp" | "go" | "java" | "js" | "ts" | "json5" => "//",
        "sql" | "lua" | "haskell" => "--",
        _ => "#",
    }
}

/// A fence opener's backtick count, if this line opens or closes one.
fn fence_width(line: &str) -> Option<usize> {
    let ticks = line.trim_start().chars().take_while(|c| *c == '`').count();
    (ticks >= 3).then_some(ticks)
}

/// A fence opener's info string (`rust`, `makefile`, `yaml`, …).
fn fence_info(line: &str) -> String {
    line.trim_start().trim_start_matches('`').trim().to_string()
}

/// The source-link footer for one block.
///
/// Returns `None` when nothing in the block resolved to a line range — a
/// `delete`, a prose step, or a block that is entirely elided. A footer that
/// names no code is furniture.
#[must_use]
pub fn footer(
    step: &PlannedStep,
    display: &BlockDisplay,
    repo: &str,
    links: &LinkTemplates,
) -> Option<String> {
    let ranges: Vec<&LineRange> = display.ranges.iter().flatten().collect();
    let first = ranges.first()?;
    let tag = step.tag();

    let mut parts = vec![format!("`{}`", first.file)];

    // One link per shown span, because a block may show two slices of a file
    // and a reader following the link deserves the one they just read.
    for range in &ranges {
        let label = format!("L{}–L{}", range.start, range.end);
        parts.push(match subst(links.blob.as_deref(), &tag, Some(range)) {
            Some(url) => format!("[{label}]({url})"),
            None => label,
        });
    }

    parts.push(format!("step {:03} of {repo}", step.seq));

    if let Some(url) = subst(links.commit.as_deref(), &tag, None) {
        parts.push(format!("[diff]({url})"));
    }
    if let Some(url) = subst(links.tree.as_deref(), &tag, None) {
        parts.push(format!("[browse]({url})"));
    }

    Some(format!("<sub>{}</sub>", parts.join(" · ")))
}

/// Fill `{tag}`, `{path}`, `{start}`, `{end}` in a template.
///
/// Literal replacement, not a template engine: four placeholders do not justify
/// a dependency, and a template that can compute is one that can fail at render
/// time. An absent template yields no link at all rather than a plausible
/// broken one — the rule `Book-Url` already follows.
fn subst(template: Option<&str>, tag: &str, range: Option<&LineRange>) -> Option<String> {
    let mut url = template?.replace("{tag}", tag);
    if let Some(r) = range {
        url = url
            .replace("{path}", &r.file)
            .replace("{start}", &r.start.to_string())
            .replace("{end}", &r.end.to_string());
    }
    Some(url)
}

/// Line number → the block that directive introduces, with its step and repo.
///
/// Keyed by the *block's* location rather than the step's: a step may hold
/// several blocks, and each gets its own footer naming its own file.
fn blocks_by_line<'a>(
    plan: &'a BookPlan,
    chapter_path: &str,
) -> BTreeMap<usize, (String, &'a PlannedStep, &'a BlockDisplay)> {
    let mut out = BTreeMap::new();
    for repo in &plan.repos {
        for step in &repo.steps {
            for display in &step.displays {
                if display.loc.chapter == chapter_path {
                    out.insert(display.loc.line, (repo.repo.0.clone(), step, display));
                }
            }
        }
    }
    out
}

/// Line number → step id, for every step anchored in this chapter.
fn anchors_by_line(plan: &BookPlan, chapter_path: &str) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    for repo in &plan.repos {
        for step in &repo.steps {
            if step.anchor.chapter == chapter_path {
                out.insert(step.anchor.line, step.id.0.clone());
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod render_tests {
    use super::*;
    use bower_core::prelude::{plan, BookSource, Chapter, RepoCatalog};

    fn no_links() -> BTreeMap<String, LinkTemplates> {
        BTreeMap::new()
    }

    fn github_links() -> BTreeMap<String, LinkTemplates> {
        let mut m = BTreeMap::new();
        m.insert(
            "r".to_string(),
            LinkTemplates {
                blob: Some("https://x.invalid/blob/{tag}/{path}#L{start}-L{end}".to_string()),
                tree: Some("https://x.invalid/tree/{tag}".to_string()),
                commit: Some("https://x.invalid/commit/{tag}".to_string()),
            },
        );
        m
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn render__unmarked_block_is_untouched() {
        let body = lines("fn main() {}\nlet x = 1;");
        assert_eq!(body_lines("rust", &body), body);
    }

    #[test]
    fn render__marked_rust_block_hides_the_rest() {
        let body = lines(
            "mod tests {\n    // bower:show\n    fn new() {}\n    // bower:show end\n    fn old() {}\n}",
        );
        assert_eq!(
            body_lines("rust", &body),
            vec![
                "# mod tests {",
                "    fn new() {}",
                "    # fn old() {}",
                "# }",
            ]
        );
    }

    #[test]
    fn render__marked_yaml_block_elides_with_a_comment() {
        // `# ` is not a hidden-line marker outside Rust, so eliding by
        // prefixing would corrupt the file rather than hide it.
        let body =
            lines("jobs:\n  # bower:show\n  - run: make ayce\n  # bower:show end\n  extra: 1");
        assert_eq!(
            body_lines("yaml", &body),
            vec![
                "# ⋯ 1 line elided",
                "  - run: make ayce",
                "# ⋯ 1 line elided"
            ]
        );
    }

    #[test]
    fn hide__preserves_indentation_and_blanks() {
        assert_eq!(hide("    x"), "    # x");
        assert_eq!(hide(""), "#");
    }

    fn tiny_plan(text: &str) -> BookPlan {
        let book = BookSource::from_chapters(vec![Chapter::new("src/ch01.md", text)]);
        plan(&book, &RepoCatalog::from_names(&["r"])).unwrap()
    }

    const CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"first\" file=\"src/lib.rs\" -->\n\n",
        "```rust\n",
        "// bower:show\n",
        "pub fn shown() {}\n",
        "// bower:show end\n",
        "fn hidden() {}\n",
        "```\n\n",
        "Prose after.\n",
    );

    #[test]
    fn render__directive_comments_do_not_survive() {
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links());
        assert!(!out.contains("<!-- bower"), "{out}");
    }

    #[test]
    fn render__anchor_precedes_the_block() {
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links());
        let anchor = out
            .find("<a id=\"step-first\"></a>")
            .expect("anchor missing");
        let fence = out.find("```rust").expect("fence missing");
        assert!(anchor < fence, "{out}");
    }

    #[test]
    fn render__applies_display_markers_and_keeps_prose() {
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links());
        assert!(out.contains("pub fn shown() {}"), "{out}");
        assert!(out.contains("# fn hidden() {}"), "{out}");
        assert!(!out.contains("bower:show"), "{out}");
        assert!(out.contains("Prose after."), "{out}");
    }

    #[test]
    fn render__a_directive_shown_inside_a_fence_survives() {
        // A book about Bower quotes directives. Stripping one out of an
        // example fence would erase the very thing the page is teaching.
        let text = concat!(
            "# Showing a directive\n\n",
            "````markdown\n",
            "<!-- bower repo=\"r\" file=\"src/lib.rs\" -->\n",
            "````\n",
        );
        let out = chapter(text, "src/ch01.md", &tiny_plan(CH), &no_links());
        assert!(out.contains("<!-- bower repo="), "{out}");
    }

    #[test]
    fn render__a_chapter_with_no_directives_is_unchanged() {
        let text = "# Plain\n\nJust prose.\n";
        assert_eq!(
            chapter(text, "src/ch01.md", &tiny_plan(CH), &no_links()),
            text
        );
    }

    #[test]
    fn footer__names_the_file_and_line_range() {
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &github_links());
        assert!(out.contains("<sub>`src/lib.rs`"), "{out}");
        assert!(out.contains("step 001 of r"), "{out}");
        assert!(
            out.contains("[L1–L1](https://x.invalid/blob/step-001-first/src/lib.rs#L1-L1)"),
            "{out}"
        );
        assert!(
            out.contains("[diff](https://x.invalid/commit/step-001-first)"),
            "{out}"
        );
        assert!(
            out.contains("[browse](https://x.invalid/tree/step-001-first)"),
            "{out}"
        );
    }

    #[test]
    fn footer__omits_links_without_templates() {
        // A plausible-looking broken link is worse than plain text.
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links());
        assert!(out.contains("<sub>`src/lib.rs`"), "{out}");
        assert!(out.contains("L1–L1"), "{out}");
        assert!(!out.contains("]("), "no links may be invented: {out}");
        assert!(!out.contains("diff"), "{out}");
    }

    #[test]
    fn footer__comes_after_the_closing_fence() {
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &github_links());
        let fence_end = out.rfind("```").expect("closing fence");
        let footer = out.find("<sub>").expect("footer");
        assert!(
            footer > fence_end,
            "a footer inside the fence is code: {out}"
        );
    }

    const PROSE_CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"note\" op=\"none\" -->\n\n",
        "Just narrative, no code.\n",
    );

    #[test]
    fn footer__absent_when_a_block_has_no_range() {
        // A prose step names no file. A footer here would be furniture.
        let plan = {
            let book = bower_core::prelude::BookSource::from_chapters(vec![Chapter::new(
                "src/ch01.md",
                PROSE_CH,
            )]);
            bower_core::prelude::plan(&book, &RepoCatalog::from_names(&["r"])).unwrap()
        };
        let out = chapter(PROSE_CH, "src/ch01.md", &plan, &github_links());
        assert!(!out.contains("<sub>"), "{out}");
        assert!(out.contains("<a id=\"step-note\"></a>"), "{out}");
        assert!(out.contains("Just narrative"), "{out}");
    }
}
