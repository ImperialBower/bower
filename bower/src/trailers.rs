//! Commit messages and `STEPS.md` — the links from the repository back to the
//! book that produced it (spec § 5.2).
//!
//! SHAs change whenever an earlier step changes, so the book never links to a
//! SHA and the repository never quotes one. Both directions go through the step
//! id: the book links to a tag, and every commit names the chapter and anchor
//! it came from.

use std::fmt::Write as _;

use bower_core::prelude::{PlannedStep, RepoPlan};

/// The tool version, stamped into every commit so a repository says what built
/// it without anyone having to guess.
const GENERATOR: &str = concat!("bower v", env!("CARGO_PKG_VERSION"));

/// The full commit message for one step: subject, blank line, trailers.
#[must_use]
pub fn commit_message(
    step: &PlannedStep,
    book_name: &str,
    site: Option<&str>,
    repo_name: &str,
) -> String {
    let anchor = format!("step-{}", step.id.0);
    let mut out = format!(
        "{}\n\nBook-Source: {book_name}/{}#{anchor}\n",
        step.msg, step.anchor.chapter
    );
    if let Some(url) = book_url(site, &step.anchor.chapter, &anchor) {
        let _ = writeln!(out, "Book-Url: {url}");
    }
    let _ = writeln!(out, "Bower-Step: {repo_name}/{:03}", step.seq);
    let _ = writeln!(out, "Generated-By: {GENERATOR}");
    out
}

/// The scaffolding commit's message. It comes from the template directory
/// rather than any chapter, so it carries no `Book-Source`: a trailer pointing
/// at a chapter that did not produce it would be a lie.
#[must_use]
pub fn scaffolding_message(repo_name: &str) -> String {
    format!(
        "chore: initial commit — scaffolding\n\nBower-Step: {repo_name}/000\nGenerated-By: {GENERATOR}\n"
    )
}

/// The line that identifies a repository as generated, and by which book.
///
/// `bower push` reads this back off a remote before it will force-push over it,
/// so the writer and the reader must be one definition. Change the wording here
/// and [`book_named_in`] follows; change it in only one place and the guard
/// stops recognizing repositories it wrote itself.
#[must_use]
pub fn marker_line(book_name: &str) -> String {
    format!("This repository is generated from the book *{book_name}*. Do not open")
}

/// The book named by a `STEPS.md`, if it carries the marker at all.
///
/// The inverse of [`marker_line`]. `None` means the text is not something Bower
/// wrote — which, for `bower push`, means it is not something Bower may
/// overwrite.
#[must_use]
pub fn book_named_in(steps_md: &str) -> Option<&str> {
    let after = steps_md.split("generated from the book *").nth(1)?;
    let name = after.split('*').next()?;
    (!name.is_empty()).then_some(name)
}

/// The repository's own table of contents back into the book.
#[must_use]
pub fn steps_md(plan: &RepoPlan, book_name: &str, site: Option<&str>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Steps\n");
    let _ = writeln!(out, "{}", marker_line(book_name));
    let _ = writeln!(
        out,
        "pull requests here — change the book instead. Every row below is one\n\
         commit, and every tag is a permanent link to the code at that point.\n"
    );
    let _ = writeln!(out, "| # | Tag | Expect | Subject | Source |");
    let _ = writeln!(out, "|---|---|---|---|---|");

    for step in &plan.steps {
        let anchor = format!("step-{}", step.id.0);
        let source = match book_url(site, &step.anchor.chapter, &anchor) {
            Some(url) => format!("[{}]({url})", step.anchor.chapter),
            None => format!("`{}`", step.anchor.chapter),
        };
        let _ = writeln!(
            out,
            "| {:03} | `{}` | {} | {} | {source} |",
            step.seq,
            step.tag(),
            step.expect,
            step.msg
        );
    }
    out
}

/// The rendered book's URL for a chapter anchor, when the book declares a site.
/// Absent site means no link at all, rather than a plausible-looking broken one.
fn book_url(site: Option<&str>, chapter: &str, anchor: &str) -> Option<String> {
    let site = site?.trim_end_matches('/');
    Some(format!("{site}/{}#{anchor}", html_name(chapter)))
}

/// `src/ch03-lints-and-format.md` → `ch03-lints-and-format.html`, matching what
/// mdBook emits.
fn html_name(chapter: &str) -> String {
    let name = chapter.strip_prefix("src/").unwrap_or(chapter);
    match name.strip_suffix(".md") {
        Some(stem) => format!("{stem}.html"),
        None => name.to_string(),
    }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod trailer_tests {
    use super::*;
    use bower_core::prelude::{RepoCatalog, plan};

    fn sample_plan() -> RepoPlan {
        let f = bower_testkit::fixtures::hello_playbook();
        let catalog = RepoCatalog::from_names(&["hello-playbook"]);
        plan(&f.book, &catalog)
            .unwrap()
            .repos
            .into_iter()
            .next()
            .unwrap()
    }

    fn step_named(plan: &RepoPlan, id: &str) -> PlannedStep {
        plan.steps.iter().find(|s| s.id.0 == id).unwrap().clone()
    }

    #[test]
    fn html_name__matches_what_mdbook_emits() {
        assert_eq!(
            html_name("src/ch03-lints-and-format.md"),
            "ch03-lints-and-format.html"
        );
    }

    #[test]
    fn trailers__name_the_chapter_and_step() {
        let plan = sample_plan();
        let step = step_named(&plan, "test-that-fails");
        let msg = commit_message(
            &step,
            "hello-playbook",
            Some("https://example.invalid/book/"),
            "hello-playbook",
        );

        assert!(msg.starts_with("test: greet should ignore stray whitespace (failing)\n\n"));
        assert!(msg.contains(
            "Book-Source: hello-playbook/src/ch04-tests-and-failing-on-purpose.md#step-test-that-fails"
        ));
        assert!(msg.contains(
            "Book-Url: https://example.invalid/book/ch04-tests-and-failing-on-purpose.html#step-test-that-fails"
        ));
        assert!(msg.contains("Bower-Step: hello-playbook/011"));
        assert!(msg.contains("Generated-By: bower v"));
    }

    #[test]
    fn trailers__book_url_is_omitted_without_a_site() {
        let plan = sample_plan();
        let step = step_named(&plan, "cargo-init");
        let msg = commit_message(&step, "hello-playbook", None, "hello-playbook");
        assert!(!msg.contains("Book-Url"), "{msg}");
        assert!(msg.contains("Book-Source:"), "{msg}");
    }

    #[test]
    fn scaffolding__carries_no_book_source() {
        // Step 0 comes from the template directory, not from any chapter.
        let msg = scaffolding_message("hello-playbook");
        assert!(!msg.contains("Book-Source"), "{msg}");
        assert!(msg.contains("Bower-Step: hello-playbook/000"));
    }

    #[test]
    fn marker__round_trips_through_its_own_reader() {
        // The writer and the reader are one definition, and this is what
        // proves it. `bower push` refuses to overwrite a repository whose
        // marker it cannot recognize.
        let line = marker_line("hello-playbook");
        assert_eq!(book_named_in(&line), Some("hello-playbook"));
    }

    #[test]
    fn marker__is_found_in_a_whole_steps_md() {
        let plan = sample_plan();
        let md = steps_md(&plan, "hello-playbook", None);
        assert_eq!(book_named_in(&md), Some("hello-playbook"));
    }

    #[test]
    fn marker__absent_from_ordinary_prose() {
        assert_eq!(book_named_in("# My Project\n\nA real repository.\n"), None);
        assert_eq!(book_named_in(""), None);
    }

    #[test]
    fn steps_md__lists_every_step_once() {
        let plan = sample_plan();
        let md = steps_md(&plan, "hello-playbook", Some("https://example.invalid"));
        assert_eq!(
            md.matches("| step-").count() + md.matches("| `step-").count(),
            20
        );
        assert!(md.contains("`step-011-test-that-fails`"));
        assert!(md.contains("test_fail"));
    }
}
