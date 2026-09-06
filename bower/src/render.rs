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
    BlockDisplay, BookPlan, Directive, Exercise, ExerciseForm, Expect, LineRange, PlannedStep,
    ShowMark, StepId, show_marker,
};

use crate::config::LinkTemplates;
use crate::publish::Target;

/// Rewrite one chapter's markdown.
#[must_use]
pub fn chapter(
    text: &str,
    chapter_path: &str,
    plan: &BookPlan,
    forge: &BTreeMap<String, LinkTemplates>,
    target: Target,
) -> String {
    let anchors = anchors_by_line(plan, chapter_path);
    let blocks = blocks_by_line(plan, chapter_path);
    let exercises = exercises_by_line(plan, chapter_path);
    let folds = folds_by_line(plan, chapter_path);
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

        let entry = exercises.get(&directive_line);
        if let Some(next) = push_block_box(&mut out, entry, forge, target, &lines, i) {
            i = next;
            continue;
        }

        // A fence may follow, after blank lines. `op="none"` and `op="delete"`
        // directives have none, and prose simply resumes.
        let mut j = i;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        let Some(width) = fence_width(lines.get(j).copied().unwrap_or_default()) else {
            push_key_box(&mut out, entry, forge, target);
            continue;
        };

        let fold = fold_label(&folds, directive_line, target);
        open_fold(&mut out, fold);

        // The header belongs right above the fence: a reader deciding whether
        // a block is worth reading wants the file and step before the code,
        // not after it.
        if let Some((repo, step, display)) = blocks.get(&directive_line) {
            // A chapter may feed several repos, and each declares its own forge
            // templates. A repo with none gets a header with plain text where
            // the links would be, never someone else's URLs.
            let empty = LinkTemplates::default();
            let repo_links = forge.get(repo).unwrap_or(&empty);
            if let Some(line) = footer(step, display, repo, repo_links) {
                out.push(line);
                out.push(String::new());
            }
        }

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
        let block = blocks.get(&directive_line);
        // The link spec § 3.4 wants beside an elision: the whole file, not the
        // span. Built here because only `chapter` knows the block's step and
        // its repo's templates.
        let full_file = block.and_then(|(repo, step, display)| {
            let template = forge.get(repo)?.blob.as_deref()?;
            let file = display.file.as_deref()?;
            Some(full_file_url(template, &step.tag(), file))
        });
        out.extend(body_lines(
            &info,
            &body,
            block.map(|(_, _, d)| *d),
            target,
            full_file.as_deref(),
        ));
        if k < lines.len() {
            out.push(lines[k].to_string());
        }
        // Below the block, not above it: the reader who has just read the code
        // is the one who wants it in front of them locally.
        if let Some((repo, step, _)) = block
            && let Some(line) = checkout_line(step, forge.get(repo))
        {
            out.push(String::new());
            out.push(line);
        }
        close_fold(&mut out, fold);
        push_key_box(&mut out, entry, forge, target);
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
pub fn body_lines(
    info: &str,
    raw: &[String],
    display: Option<&BlockDisplay>,
    target: Target,
    full_file: Option<&str>,
) -> Vec<String> {
    if !raw.iter().any(|l| show_marker(l).is_some()) {
        return raw.to_vec();
    }

    // Which spans actually render is the kernel's answer, not ours: a `show=`
    // key on the directive narrows a block to named spans
    // (`bower-core`'s `display::filter_by_show`). Re-deriving that from the
    // markers alone renders every marked span, so the page would show what the
    // footer does not link.
    let kept: Option<Vec<Option<String>>> =
        display.map(|d| d.spans.iter().map(|s| s.name.clone()).collect());

    // mdBook's hidden-line toggle is a Rust feature of one renderer. In an
    // epub there is no toggle at all, so a `# `-prefixed line would simply
    // vanish with nothing telling the reader it had been there.
    let rustish = target.has_hidden_lines() && info.split([',', ' ']).next() == Some("rust");
    let comment = comment_token(info);

    let mut out = Vec::with_capacity(raw.len());
    let mut showing = false;
    let mut elided = 0_usize;

    let flush = |out: &mut Vec<String>, elided: &mut usize| {
        if *elided > 0 && !rustish {
            let plural = if *elided == 1 { "line" } else { "lines" };
            let link = full_file.map_or_else(String::new, |u| format!(" — full file: {u}"));
            out.push(format!("{comment} ... {elided} {plural} elided{link}"));
        }
        *elided = 0;
    };

    for line in raw {
        match show_marker(line) {
            Some(ShowMark::Begin(name)) => {
                flush(&mut out, &mut elided);
                // A span the kernel dropped is elided like any unmarked code.
                showing = kept.as_ref().is_none_or(|k| k.contains(&name));
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

/// The whole file at a step, for the comment that replaces an elided span.
///
/// The `blob` template addresses a *line range*; a reader following an elision
/// wants the file. Dropping the template's fragment is what turns one into the
/// other, and is why this is a function rather than another `subst` call.
fn full_file_url(template: &str, tag: &str, file: &str) -> String {
    let url = template.replace("{tag}", tag).replace("{path}", file);
    url.split('#').next().unwrap_or(&url).to_string()
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

/// The source-link header for one block, printed above its fence.
///
/// Returns `None` when nothing in the block resolved to a line range — a
/// `delete`, a prose step, or a block that is entirely elided. A header that
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

    let file_label = format!("💾 `{}`", first.file);
    let mut parts = vec![match links.blob.as_deref() {
        Some(template) => {
            let url = full_file_url(template, &tag, &first.file);
            format!("[{file_label}]({url} \"View full file\")")
        }
        None => file_label,
    }];

    // One link per shown span, because a block may show two slices of a file
    // and a reader following the link deserves the one they just read.
    for range in &ranges {
        let label = format!("L{}–L{}", range.start, range.end);
        parts.push(match subst(links.blob.as_deref(), &tag, Some(range)) {
            Some(url) => format!("[{label}]({url} \"View source\")"),
            None => label,
        });
    }

    parts.push(format!("step {:03} of {repo}", step.seq));

    if let Some(url) = subst(links.commit.as_deref(), &tag, None) {
        parts.push(format!("[diff]({url} \"View diff\")"));
    }
    if let Some(url) = subst(links.tree.as_deref(), &tag, None) {
        parts.push(format!("[browse]({url} \"Browse repo\")"));
    }

    Some(format!(
        "<span class=\"step-meta\"><sub>{}</sub></span>",
        parts.join(" · ")
    ))
}

/// The command that puts a local clone at this step, printed below the block.
///
/// Returns `None` when the repo declares no `checkout` template, which is what
/// a repo with no `github` remote gets: a reader cannot check out a repository
/// that was never pushed, and a command that fails is worse than no command.
///
/// The `$>` prompt sits *outside* the code span so that copying the span yields
/// a command that runs, not one that starts with a prompt.
///
/// The `step-checkout` class is the whole of the tool's opinion about looks: a
/// book's own theme decides the rest, exactly as `step-meta` already works.
#[must_use]
pub fn checkout_line(step: &PlannedStep, links: Option<&LinkTemplates>) -> Option<String> {
    let cmd = subst(links?.checkout.as_deref(), &step.tag(), None)?;
    Some(format!("<span class=\"step-checkout\">$> `{cmd}`</span>"))
}

/// The "your turn" box for a step's exercise (spec § 6.1), as markdown lines.
///
/// One text for every target; only the wrapping differs. HTML gets a `<div>`
/// with a class the book's theme styles — the tool's whole opinion about
/// looks, as with `step-meta`. Epub and PDF have no stylesheet of ours, so
/// they get a blockquote, which every renderer draws as an aside.
///
/// The commands need a remote: a repo nobody can clone gets the task, the
/// detail, and the closing line, and no command that would fail.
#[must_use]
pub fn exercise_box(
    step: &PlannedStep,
    exercise: &Exercise,
    links: Option<&LinkTemplates>,
    target: Target,
) -> Vec<String> {
    let mut body: Vec<String> = vec![format!("**Your turn: {}**", exercise.task)];
    if !exercise.detail.is_empty() {
        body.push(String::new());
        body.extend(exercise.detail.iter().cloned());
    }

    if let Some(l) = links
        && let Some(clone) = l.clone.as_deref()
        && let Some(checkout) = subst(l.checkout.as_deref(), &step.tag(), None)
    {
        body.push(String::new());
        body.push(match l.fork.as_deref() {
            Some(url) => format!("Fork [the repository]({url}), then:"),
            None => "Clone the repository, then:".to_string(),
        });
        body.push(String::new());
        body.push("```console".to_string());
        body.push(format!("{clone}   # or your fork"));
        body.push(checkout);
        if let Some(cmd) = reader_command(step.expect, l) {
            body.push(cmd.to_string());
        }
        body.push("```".to_string());
    }

    body.push(String::new());
    body.push(closing_line(step.expect, &exercise.answer, target));
    wrap_box(body, target)
}

/// Which of the repo's two commands a reader runs after this step: `check`
/// after a `compile_fail`, `verify` after anything with tests to run, nothing
/// after a step the verifier skips.
fn reader_command(expect: Expect, links: &LinkTemplates) -> Option<&str> {
    match expect {
        Expect::CompileFail => links.check.as_deref(),
        Expect::TestFail | Expect::Pass => links.verify.as_deref(),
        Expect::Skip => None,
    }
}

/// The last line: what green means, and where the next step is. A broken
/// step has an answer; a green one has one way among many. Only HTML has an
/// anchor to link.
fn closing_line(expect: Expect, answer: &StepId, target: Target) -> String {
    let (lower, upper) = match target {
        Target::Html => (
            format!("[the next step](#step-{answer})"),
            format!("[The next step](#step-{answer})"),
        ),
        Target::Epub | Target::Pdf => ("the next step".to_string(), "The next step".to_string()),
    };
    match expect {
        Expect::CompileFail | Expect::TestFail => {
            format!("Green means you did it. The answer is {lower}.")
        }
        Expect::Pass => format!("Keep it green. {upper} shows one way."),
        Expect::Skip => format!("{upper} shows one way."),
    }
}

/// HTML: a `<div>` the theme styles, blank-line separated so the markdown
/// inside still renders (`CommonMark` ends an HTML block at a blank line).
/// Everything else: a blockquote.
fn wrap_box(body: Vec<String>, target: Target) -> Vec<String> {
    match target {
        Target::Html => {
            let mut out = vec!["<div class=\"step-exercise\">".to_string(), String::new()];
            out.extend(body);
            out.push(String::new());
            out.push("</div>".to_string());
            out
        }
        Target::Epub | Target::Pdf => body
            .into_iter()
            .map(|l| {
                if l.is_empty() {
                    ">".to_string()
                } else {
                    format!("> {l}")
                }
            })
            .collect(),
    }
}

/// The toggle's label over an answer step's blocks: a broken step has an
/// answer, a green one has one way.
fn answer_summary(expect: Expect) -> &'static str {
    match expect {
        Expect::CompileFail | Expect::TestFail => "Show the answer",
        Expect::Pass | Expect::Skip => "Show one way",
    }
}

/// Print a key-form box below whatever the step's last block left on the
/// page — a fence and its checkout line, or nothing at all for a prose step.
fn push_key_box(
    out: &mut Vec<String>,
    entry: Option<&(String, &PlannedStep, &Exercise)>,
    forge: &BTreeMap<String, LinkTemplates>,
    target: Target,
) {
    if let Some((repo, step, x)) = entry
        && x.form == ExerciseForm::Key
    {
        out.push(String::new());
        out.extend(exercise_box(step, x, forge.get(repo), target));
        out.push(String::new());
    }
}

/// A block-form exercise: its fence is the box's detail, not code. Print the
/// box where the author put the directive and return where scanning resumes,
/// past the fence that never renders. `None` for every other directive.
fn push_block_box(
    out: &mut Vec<String>,
    entry: Option<&(String, &PlannedStep, &Exercise)>,
    forge: &BTreeMap<String, LinkTemplates>,
    target: Target,
    lines: &[&str],
    from: usize,
) -> Option<usize> {
    let (repo, step, x) = entry?;
    if x.form != ExerciseForm::Block {
        return None;
    }
    out.extend(exercise_box(step, x, forge.get(repo), target));
    out.push(String::new());
    Some(skip_directive_fence(lines, from))
}

/// An answer step's code folds shut in HTML; the reader opens it after
/// trying. Print has no toggle anywhere, so it prints.
fn fold_label(
    folds: &BTreeMap<usize, &'static str>,
    directive_line: usize,
    target: Target,
) -> Option<&'static str> {
    target
        .has_hidden_lines()
        .then(|| folds.get(&directive_line).copied())
        .flatten()
}

/// Open the fold above a block, when this block is an answer's.
fn open_fold(out: &mut Vec<String>, label: Option<&str>) {
    if let Some(label) = label {
        out.push("<details class=\"step-answer\">".to_string());
        out.push(format!("<summary>{label}</summary>"));
        out.push(String::new());
    }
}

/// Close the fold below a block and its checkout line.
fn close_fold(out: &mut Vec<String>, label: Option<&str>) {
    if label.is_some() {
        out.push(String::new());
        out.push("</details>".to_string());
    }
}

/// From just past a directive, the index just past the fence that follows it
/// (blank lines between allowed). A directive with no fence yields `from`.
fn skip_directive_fence(lines: &[&str], from: usize) -> usize {
    let mut j = from;
    while j < lines.len() && lines[j].trim().is_empty() {
        j += 1;
    }
    let Some(width) = lines.get(j).and_then(|l| fence_width(l)) else {
        return from;
    };
    let mut k = j + 1;
    while k < lines.len() && fence_width(lines[k]).is_none_or(|w| w < width) {
        k += 1;
    }
    (k + 1).min(lines.len())
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

/// Line number → the exercise that renders there, with its step and repo.
/// Keyed by [`Exercise::at`]: the block form's own directive, or the key
/// form's last block.
fn exercises_by_line<'a>(
    plan: &'a BookPlan,
    chapter_path: &str,
) -> BTreeMap<usize, (String, &'a PlannedStep, &'a Exercise)> {
    let mut out = BTreeMap::new();
    for repo in &plan.repos {
        for step in &repo.steps {
            if let Some(x) = &step.exercise
                && x.at.chapter == chapter_path
            {
                out.insert(x.at.line, (repo.repo.0.clone(), step, x));
            }
        }
    }
    out
}

/// Line number → the fold's label, for every block of a step that is some
/// exercise's answer.
fn folds_by_line(plan: &BookPlan, chapter_path: &str) -> BTreeMap<usize, &'static str> {
    let mut out = BTreeMap::new();
    for repo in &plan.repos {
        for step in &repo.steps {
            let Some(x) = &step.exercise else { continue };
            let Some(answer) = repo.steps.iter().find(|s| s.id == x.answer) else {
                continue;
            };
            for display in &answer.displays {
                if display.loc.chapter == chapter_path {
                    out.insert(display.loc.line, answer_summary(step.expect));
                }
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod render_tests {
    use super::*;
    use bower_core::prelude::{BookSource, Chapter, RepoCatalog, plan};

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
                checkout: Some("git checkout {tag}".to_string()),
                ..LinkTemplates::default()
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
        assert_eq!(body_lines("rust", &body, None, Target::Html, None), body);
    }

    #[test]
    fn render__marked_rust_block_hides_the_rest() {
        let body = lines(
            "mod tests {\n    // bower:show\n    fn new() {}\n    // bower:show end\n    fn old() {}\n}",
        );
        assert_eq!(
            body_lines("rust", &body, None, Target::Html, None),
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
            body_lines("yaml", &body, None, Target::Html, None),
            vec![
                "# ... 1 line elided",
                "  - run: make ayce",
                "# ... 1 line elided"
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
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links(), Target::Html);
        assert!(!out.contains("<!-- bower"), "{out}");
    }

    #[test]
    fn render__anchor_precedes_the_block() {
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links(), Target::Html);
        let anchor = out
            .find("<a id=\"step-first\"></a>")
            .expect("anchor missing");
        let fence = out.find("```rust").expect("fence missing");
        assert!(anchor < fence, "{out}");
    }

    #[test]
    fn render__applies_display_markers_and_keeps_prose() {
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links(), Target::Html);
        assert!(out.contains("pub fn shown() {}"), "{out}");
        assert!(out.contains("# fn hidden() {}"), "{out}");
        assert!(!out.contains("bower:show"), "{out}");
        assert!(out.contains("Prose after."), "{out}");
    }

    const SHOW_KEY_CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"first\" file=\"src/lib.rs\" show=\"second\" -->\n\n",
        "```rust\n",
        "// bower:show begin first\n",
        "pub fn one() {}\n",
        "// bower:show end\n",
        "// bower:show begin second\n",
        "pub fn two() {}\n",
        "// bower:show end\n",
        "```\n",
    );

    #[test]
    fn target__html_keeps_the_rust_toggle() {
        let body =
            lines("mod tests {\n    // bower:show\n    fn new() {}\n    // bower:show end\n}");
        let out = body_lines("rust", &body, None, Target::Html, None);
        assert!(
            out.iter()
                .any(|l| l.trim_start().starts_with("# mod tests")),
            "html must keep mdBook's hidden lines: {out:?}"
        );
        assert!(!out.iter().any(|l| l.contains("elided")), "{out:?}");
    }

    #[test]
    fn target__epub_elides_rust_too() {
        // An epub has no toggle anywhere, so hidden lines would simply vanish
        // with nothing telling the reader they existed.
        let body =
            lines("mod tests {\n    // bower:show\n    fn new() {}\n    // bower:show end\n}");
        let out = body_lines("rust", &body, None, Target::Epub, None);
        assert!(
            out.iter().any(|l| l.contains("1 line elided")),
            "epub must say what it left out: {out:?}"
        );
        assert!(
            !out.iter()
                .any(|l| l.trim_start().starts_with("# mod tests")),
            "an epub hidden line is just a comment nobody can expand: {out:?}"
        );
    }

    #[test]
    fn elision__names_the_full_file_when_a_template_exists() {
        // Spec § 3.4 asks for the link; there was no target that needed one.
        let body = lines("a\n// bower:show\nb\n// bower:show end");
        let out = body_lines(
            "rust",
            &body,
            None,
            Target::Epub,
            Some("https://x.invalid/blob/step-001-a/src/lib.rs"),
        );
        assert!(
            out.iter()
                .any(|l| l.contains("full file: https://x.invalid/blob/step-001-a/src/lib.rs")),
            "{out:?}"
        );
    }

    #[test]
    fn elision__is_ascii_so_every_font_can_render_it() {
        // The elision lands inside a code block, where the font is a monospace
        // one the renderer picked. `⋯` (U+22EF) is absent from Latin Modern
        // Mono, so xelatex dropped it silently and the PDF read
        // `# 9 lines elided` with no ellipsis at all. ASCII is what a code
        // comment should be anyway.
        let body = lines("a\n// bower:show\nb\n// bower:show end");
        let out = body_lines("rust", &body, None, Target::Epub, None);
        let elision = out.iter().find(|l| l.contains("elided")).unwrap();
        assert!(
            elision.is_ascii(),
            "a glyph a mono font may lack: {elision:?}"
        );
    }

    #[test]
    fn elision__omits_the_link_without_one() {
        // The rule `Book-Url` and `footer` already follow: no link beats a
        // plausible broken one.
        let body = lines("a\n// bower:show\nb\n// bower:show end");
        let out = body_lines("rust", &body, None, Target::Epub, None);
        assert!(out.iter().any(|l| l.contains("elided")), "{out:?}");
        assert!(!out.iter().any(|l| l.contains("full file")), "{out:?}");
    }

    #[test]
    fn render__honours_the_show_key() {
        // The kernel filters spans by `show=` (`display::filter_by_show`).
        // Walking the raw markers instead renders every marked span, so the
        // page shows two and the footer links one.
        let plan = tiny_plan(SHOW_KEY_CH);
        let out = chapter(SHOW_KEY_CH, "src/ch01.md", &plan, &no_links(), Target::Html);
        assert!(
            out.contains("pub fn two() {}"),
            "the named span must show: {out}"
        );
        assert!(
            out.contains("# pub fn one() {}"),
            "the span `show=` did not name must be hidden: {out}"
        );
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
        let out = chapter(
            text,
            "src/ch01.md",
            &tiny_plan(CH),
            &no_links(),
            Target::Html,
        );
        assert!(out.contains("<!-- bower repo="), "{out}");
    }

    #[test]
    fn render__a_chapter_with_no_directives_is_unchanged() {
        let text = "# Plain\n\nJust prose.\n";
        assert_eq!(
            chapter(
                text,
                "src/ch01.md",
                &tiny_plan(CH),
                &no_links(),
                Target::Html
            ),
            text
        );
    }

    #[test]
    fn footer__names_the_file_and_line_range() {
        let out = chapter(
            CH,
            "src/ch01.md",
            &tiny_plan(CH),
            &github_links(),
            Target::Html,
        );
        assert!(
            out.contains(
                "<span class=\"step-meta\"><sub>[💾 `src/lib.rs`](https://x.invalid/blob/step-001-first/src/lib.rs \"View full file\")"
            ),
            "{out}"
        );
        assert!(out.contains("step 001 of r"), "{out}");
        assert!(
            out.contains(
                "[L1–L1](https://x.invalid/blob/step-001-first/src/lib.rs#L1-L1 \"View source\")"
            ),
            "{out}"
        );
        assert!(
            out.contains("[diff](https://x.invalid/commit/step-001-first \"View diff\")"),
            "{out}"
        );
        assert!(
            out.contains("[browse](https://x.invalid/tree/step-001-first \"Browse repo\")"),
            "{out}"
        );
    }

    #[test]
    fn footer__omits_links_without_templates() {
        // A plausible-looking broken link is worse than plain text.
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links(), Target::Html);
        assert!(
            out.contains("<span class=\"step-meta\"><sub>💾 `src/lib.rs`"),
            "{out}"
        );
        assert!(out.contains("L1–L1"), "{out}");
        assert!(!out.contains("]("), "no links may be invented: {out}");
        assert!(!out.contains("diff"), "{out}");
    }

    #[test]
    fn footer__comes_before_the_opening_fence() {
        let out = chapter(
            CH,
            "src/ch01.md",
            &tiny_plan(CH),
            &github_links(),
            Target::Html,
        );
        let fence_start = out.find("```").expect("opening fence");
        let header = out.find("<sub>").expect("header");
        assert!(
            header < fence_start,
            "a header inside the fence is code: {out}"
        );
    }

    #[test]
    fn checkout__comes_after_the_closing_fence() {
        // A reader who has just read the code is the one who wants it locally.
        let out = chapter(
            CH,
            "src/ch01.md",
            &tiny_plan(CH),
            &github_links(),
            Target::Html,
        );
        let cmd = out
            .find("$> `git checkout step-001-first`")
            .unwrap_or_else(|| panic!("no checkout line: {out}"));
        let closing_fence = out.rfind("```").expect("closing fence");
        assert!(
            cmd > closing_fence,
            "the command must follow the block: {out}"
        );
    }

    #[test]
    fn checkout__carries_a_class_a_book_can_style() {
        // The tool names the line; the book's theme decides how it looks.
        let out = chapter(
            CH,
            "src/ch01.md",
            &tiny_plan(CH),
            &github_links(),
            Target::Html,
        );
        assert!(
            out.contains("<span class=\"step-checkout\">$> `git checkout step-001-first`</span>"),
            "{out}"
        );
    }

    #[test]
    fn checkout__prompt_sits_outside_the_command() {
        // `$>` inside the span would be copied along with the command.
        let out = chapter(
            CH,
            "src/ch01.md",
            &tiny_plan(CH),
            &github_links(),
            Target::Html,
        );
        assert!(
            !out.contains("`$>"),
            "the prompt must not be copyable: {out}"
        );
    }

    #[test]
    fn checkout__omitted_without_a_template() {
        // A book with no remote has no repository a reader can check out.
        let out = chapter(CH, "src/ch01.md", &tiny_plan(CH), &no_links(), Target::Html);
        assert!(!out.contains("git checkout"), "{out}");
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
        let out = chapter(
            PROSE_CH,
            "src/ch01.md",
            &plan,
            &github_links(),
            Target::Html,
        );
        assert!(!out.contains("<sub>"), "{out}");
        assert!(out.contains("<a id=\"step-note\"></a>"), "{out}");
        assert!(out.contains("Just narrative"), "{out}");
        // No fence, so no code to check out either.
        assert!(!out.contains("git checkout"), "{out}");
    }

    const EXERCISE_CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"broken\" file=\"src/lib.rs\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n\n",
        "```rust\n",
        "fn x() -> u32 { \"42\" }\n",
        "```\n\n",
        "Between.\n\n",
        "<!-- bower repo=\"r\" step=\"fixed\" file=\"src/lib.rs\" op=\"replace\" -->\n\n",
        "```rust\n",
        "fn x() -> u32 { 42 }\n",
        "```\n\n",
        "<!-- bower repo=\"r\" exercise=\"Do it without a literal\" -->\n",
        "```markdown\n",
        "Parse it instead.\n",
        "```\n\n",
        "<!-- bower repo=\"r\" step=\"last\" file=\"src/lib.rs\" op=\"append\" -->\n\n",
        "```rust\n",
        "// fin\n",
        "```\n",
    );

    fn exercise_links() -> BTreeMap<String, LinkTemplates> {
        let mut m = github_links();
        let l = m.get_mut("r").unwrap();
        l.fork = Some("https://x.invalid/fork".to_string());
        l.clone = Some("git clone https://x.invalid/r.git".to_string());
        l.check = Some("cargo check".to_string());
        l.verify = Some("cargo test".to_string());
        m
    }

    fn render_exercise(target: Target, links: &BTreeMap<String, LinkTemplates>) -> String {
        chapter(
            EXERCISE_CH,
            "src/ch01.md",
            &tiny_plan(EXERCISE_CH),
            links,
            target,
        )
    }

    #[test]
    fn exercise__key_form_box_follows_the_checkout_line() {
        let out = render_exercise(Target::Html, &exercise_links());
        let checkout = out
            .find("$> `git checkout step-001-broken`")
            .expect("checkout");
        let bx = out.find("**Your turn: Make this compile**").expect("box");
        let between = out.find("Between.").expect("prose");
        assert!(checkout < bx && bx < between, "{out}");
    }

    #[test]
    fn exercise__box_names_fork_clone_checkout_and_the_check_command() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert!(
            out.contains("Fork [the repository](https://x.invalid/fork), then:"),
            "{out}"
        );
        assert!(
            out.contains(
                "```console\ngit clone https://x.invalid/r.git   # or your fork\ngit checkout step-001-broken\ncargo check\n```"
            ),
            "{out}"
        );
        assert!(
            out.contains("Green means you did it. The answer is [the next step](#step-fixed)."),
            "{out}"
        );
    }

    #[test]
    fn exercise__block_form_replaces_its_fence_and_says_one_way() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert!(
            !out.contains("```markdown"),
            "the detail fence must not render as code: {out}"
        );
        assert!(
            out.contains("**Your turn: Do it without a literal**\n\nParse it instead.\n"),
            "{out}"
        );
        // A green step: `verify`, and one way rather than the answer.
        assert!(
            out.contains("git checkout step-002-fixed\ncargo test\n```"),
            "{out}"
        );
        assert!(
            out.contains("Keep it green. [The next step](#step-last) shows one way."),
            "{out}"
        );
    }

    #[test]
    fn exercise__html_folds_the_answer_step_and_not_the_question() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert_eq!(
            out.matches("<details class=\"step-answer\">").count(),
            2,
            "fixed and last are both answers: {out}"
        );
        assert_eq!(out.matches("</details>").count(), 2, "{out}");
        assert!(out.contains("<summary>Show the answer</summary>"), "{out}");
        assert!(out.contains("<summary>Show one way</summary>"), "{out}");
        let fold = out.find("<details").unwrap();
        let broken = out.find("fn x() -> u32 { \"42\" }").unwrap();
        let fixed = out.find("fn x() -> u32 { 42 }").unwrap();
        assert!(broken < fold && fold < fixed, "{out}");
        // The fold closes after the block's checkout line, before the box.
        let close = out.find("</details>").unwrap();
        let fixed_checkout = out.find("$> `git checkout step-002-fixed`").unwrap();
        let second_box = out.find("**Your turn: Do it without a literal**").unwrap();
        assert!(fixed_checkout < close && close < second_box, "{out}");
    }

    #[test]
    fn exercise__epub_is_a_blockquote_with_no_toggle_or_anchor_link() {
        let out = render_exercise(Target::Epub, &exercise_links());
        assert!(!out.contains("<details"), "{out}");
        assert!(!out.contains("<div class=\"step-exercise\">"), "{out}");
        assert!(out.contains("> **Your turn: Make this compile**"), "{out}");
        assert!(
            out.contains("> Green means you did it. The answer is the next step."),
            "{out}"
        );
        assert!(!out.contains("#step-fixed"), "{out}");
    }

    #[test]
    fn exercise__without_a_remote_keeps_the_task_and_drops_the_commands() {
        let out = render_exercise(Target::Html, &no_links());
        assert!(out.contains("**Your turn: Make this compile**"), "{out}");
        assert!(!out.contains("Fork"), "{out}");
        assert!(!out.contains("git clone"), "{out}");
        assert!(!out.contains("cargo check"), "{out}");
        assert!(out.contains("Green means you did it."), "{out}");
    }

    #[test]
    fn exercise__html_box_is_a_div_the_theme_styles() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert!(
            out.contains("<div class=\"step-exercise\">\n\n**Your turn"),
            "{out}"
        );
        assert!(out.contains("\n\n</div>"), "{out}");
    }

    const PROSE_EXERCISE_CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"note\" op=\"none\" expect=\"none\" exercise=\"Try the refactor\" -->\n\n",
        "Just narrative.\n\n",
        "<!-- bower repo=\"r\" step=\"code\" file=\"src/lib.rs\" -->\n\n",
        "```rust\nfn y() {}\n```\n",
    );

    #[test]
    fn exercise__on_a_prose_step_renders_with_no_command_line() {
        let out = chapter(
            PROSE_EXERCISE_CH,
            "src/ch01.md",
            &tiny_plan(PROSE_EXERCISE_CH),
            &exercise_links(),
            Target::Html,
        );
        let bx = out.find("**Your turn: Try the refactor**").expect("box");
        let prose = out.find("Just narrative.").unwrap();
        assert!(bx < prose, "{out}");
        assert!(
            out.contains("git checkout step-001-note\n```"),
            "no command after the checkout on a skipped step: {out}"
        );
        assert!(
            out.contains("[The next step](#step-code) shows one way."),
            "{out}"
        );
    }
}
