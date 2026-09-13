//! Lines of history (EPIC-09): which line a step's commit sits on, the fold
//! that keeps one tree per line, the pull requests a book declares, and the
//! rules a branch name must follow.

/// Why `name` cannot name a branch, or `None` when it can (Decision 6).
///
/// Git's ref-name rules (`git check-ref-format`), written out rather than
/// asked of git — the kernel runs no programs — plus Bower's own two: `main`
/// is the main line's name, and `step-…` is how step tags are named, so a
/// branch called that would be ambiguous in `git checkout`.
// Git's own `.lock` suffix check is a literal byte match, not a
// case-insensitive extension check — `clippy::case_sensitive_file_extension_comparisons`
// is meant for filesystem paths, not ref-name syntax, so it does not apply here.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
#[must_use]
pub fn branch_name_problem(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return Some("it is empty");
    }
    if name == "main" || name == "HEAD" {
        return Some("the main line already has that name");
    }
    if name.starts_with("step-") {
        return Some("`step-` begins every step tag");
    }
    if name == "@" || name.contains("@{") {
        return Some("git reserves `@` and `@{`");
    }
    if name.starts_with(['-', '/']) || name.ends_with(['/', '.']) {
        return Some("it may not begin with `-` or `/`, or end with `/` or `.`");
    }
    if name.contains("..") || name.contains("//") {
        return Some("it may not contain `..` or `//`");
    }
    if name
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c))
    {
        return Some("it may not contain whitespace, control characters, or any of ~ ^ : ? * [ \\");
    }
    if name
        .split('/')
        .any(|part| part.starts_with('.') || part.ends_with(".lock"))
    {
        return Some("no part of it may begin with `.` or end with `.lock`");
    }
    None
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod branch_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("try/lookup-table")]
    #[case("feat/greet-many")]
    #[case("from-char")]
    #[case("a.b")]
    fn branch_name_problem__accepts_ordinary_names(#[case] name: &str) {
        assert_eq!(branch_name_problem(name), None, "{name}");
    }

    #[rstest]
    #[case("")]
    #[case("main")]
    #[case("HEAD")]
    #[case("step-001-rank")]
    #[case("@")]
    #[case("a@{b")]
    #[case("-x")]
    #[case("/x")]
    #[case("x/")]
    #[case("x.")]
    #[case("a..b")]
    #[case("a//b")]
    #[case("a b")]
    #[case("a~b")]
    #[case("a^b")]
    #[case("a:b")]
    #[case("a?b")]
    #[case("a*b")]
    #[case("a[b")]
    #[case("a\\b")]
    #[case(".x")]
    #[case("a/.b")]
    #[case("x.lock")]
    #[case("a/x.lock/b")]
    fn branch_name_problem__refuses_what_git_or_bower_cannot_use(#[case] name: &str) {
        assert!(branch_name_problem(name).is_some(), "{name:?} was accepted");
    }
}
