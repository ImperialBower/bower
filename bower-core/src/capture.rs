//! What a command printed, made comparable (EPIC-11).
//!
//! Pure string functions: the edge runs the command and hands the text in.
//! Nothing here knows about cargo beyond the shape of its output lines.

/// Machine-specific path prefixes and what replaces each: the scratch tree,
/// the shared target directory, cargo's home. The edge knows them; the kernel
/// only replaces text (EPIC-11 Decision 4). Applied longest prefix first, so
/// `/w/tree/` is replaced before `/w/tree`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Scrub(pub Vec<(String, String)>);

/// Reduce what a command printed to the lines that depend only on what the
/// compiler said — not the machine, path, clock, terminal, or scheduling.
/// Pure and idempotent: normalizing normalized text changes nothing.
#[must_use]
pub fn normalize(raw: &str, scrub: &Scrub) -> Vec<String> {
    let text = raw.replace("\r\n", "\n"); // rule 1
    let text = strip_ansi(&text); // rule 2
    let text = scrub_paths(&text, scrub); // rule 3
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| !is_cargo_status(l)) // rule 4
        .map(drop_thread_id) // rule 5
        .map(|l| strip_test_timing(&l)) // rule 6
        // Trimmed before sorting, so trailing blanks cannot change an order.
        .map(|l| l.trim_end().to_string())
        .collect();
    sort_test_runs(&mut lines); // rule 7
    tidy(&lines) // rule 8
}

/// Rule 2. A CSI sequence (`ESC [ … final byte`) and an OSC sequence
/// (`ESC ] … BEL` or `ESC ] … ESC \`) go whole; any other escape character
/// goes alone. No escape character survives, so a second pass finds nothing.
/// Hand-written: the kernel takes no `regex`.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                // Parameters and intermediates, then one final byte in `@`..=`~`.
                for n in chars.by_ref() {
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                while let Some(n) = chars.next() {
                    if n == '\x07' {
                        break;
                    }
                    if n == '\x1b' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Rule 3. Longest prefix first, so a path that contains another is never cut
/// in half.
fn scrub_paths(text: &str, scrub: &Scrub) -> String {
    let mut pairs: Vec<&(String, String)> = scrub
        .0
        .iter()
        .filter(|(from, _)| !from.is_empty())
        .collect();
    pairs.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));
    pairs.iter().fold(text.to_string(), |t, (from, to)| {
        t.replace(from.as_str(), to)
    })
}

/// The verbs cargo right-aligns in its status column. All are shorter than the
/// column, so a status line always starts with a space.
const STATUS_VERBS: &[&str] = &[
    "Compiling",
    "Checking",
    "Finished",
    "Running",
    "Doc-tests",
    "Blocking",
    "Locking",
    "Updating",
    "Downloading",
    "Downloaded",
    "Adding",
    "Fresh",
];

/// Rule 4: build chatter that varies with the cache, and a `Running` line
/// that names a hashed binary.
fn is_cargo_status(line: &str) -> bool {
    let t = line.trim_start();
    t.len() < line.len()
        && STATUS_VERBS
            .iter()
            .any(|v| t.strip_prefix(v).is_some_and(|rest| rest.starts_with(' ')))
}

/// Rule 5: `thread 'name' (691424) panicked at` → `thread 'name' panicked at`.
/// The number is an OS thread ID, new every run (EPIC-11 Decision 13). Only a
/// parenthesized number directly before `panicked at` is removed, which keeps
/// the rule idempotent.
fn drop_thread_id(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("thread '")
        && let Some(close) = rest.find("' (")
    {
        let after = &rest[close + 3..];
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && after[digits..].starts_with(") panicked at") {
            return format!("thread '{}'{}", &rest[..close], &after[digits + 1..]);
        }
    }
    line.to_string()
}

/// Rule 6: `; finished in 0.42s` off a libtest summary.
fn strip_test_timing(line: &str) -> String {
    if line.starts_with("test result: ")
        && let Some(at) = line.find("; finished in ")
    {
        return line[..at].to_string();
    }
    line.to_string()
}

/// One libtest progress line: `test <name> ... <result>`.
fn is_test_line(line: &str) -> bool {
    line.starts_with("test ") && line.contains(" ... ")
}

/// Rule 7: sort each contiguous run of test lines.
fn sort_test_runs(lines: &mut [String]) {
    let mut i = 0;
    while i < lines.len() {
        if !is_test_line(&lines[i]) {
            i += 1;
            continue;
        }
        let end = (i..lines.len())
            .find(|&k| !is_test_line(&lines[k]))
            .unwrap_or(lines.len());
        lines[i..end].sort();
        i = end;
    }
}

/// Rule 8, shared by both sides of every comparison: trailing whitespace off
/// each line, blank lines off both ends. A recorded fence goes through this at
/// plan time, so an editor that strips trailing spaces changes nothing.
#[must_use]
pub fn tidy(lines: &[String]) -> Vec<String> {
    let trimmed: Vec<String> = lines.iter().map(|l| l.trim_end().to_string()).collect();
    let start = trimmed
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(trimmed.len());
    let end = trimmed
        .iter()
        .rposition(|l| !l.is_empty())
        .map_or(start, |i| i + 1);
    trimmed[start..end].to_vec()
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod capture_tests {
    use super::*;

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn tidy__trims_line_ends_and_outer_blanks() {
        assert_eq!(
            tidy(&v(&["", "  ", "a  ", "", "b\t", ""])),
            v(&["a", "", "b"])
        );
        assert!(tidy(&v(&["", " "])).is_empty());
    }

    fn norm(raw: &str) -> Vec<String> {
        normalize(raw, &Scrub::default())
    }

    #[test]
    fn normalize__is_empty_for_silence() {
        assert!(norm("").is_empty());
        assert!(norm("\n\n").is_empty());
    }

    #[test]
    fn normalize__folds_crlf() {
        // A Windows runner must record what a Mac records.
        assert_eq!(norm("a\r\nb\r\n"), v(&["a", "b"]));
    }

    #[test]
    fn normalize__strips_ansi() {
        // CARGO_TERM_COLOR=always in someone's shell. The second line is an
        // OSC 8 hyperlink, which cargo emits on terminals that support one.
        let raw = "\x1b[1m\x1b[91merror\x1b[0m: x\n\x1b]8;;http://x.invalid\x07link\x1b]8;;\x07\n";
        assert_eq!(norm(raw), v(&["error: x", "link"]));
    }

    #[test]
    fn normalize__scrubs_the_tree_prefix() {
        // Given in the wrong order on purpose: longest first is the
        // function's job, not the caller's.
        let scrub = Scrub(vec![
            ("/w/tree".to_string(), ".".to_string()),
            ("/w/tree/".to_string(), String::new()),
        ]);
        assert_eq!(
            normalize("at `/w/tree/Cargo.toml` in /w/tree", &scrub),
            v(&["at `Cargo.toml` in ."])
        );
    }

    #[test]
    fn normalize__never_scrubs_an_empty_prefix() {
        // `str::replace("", …)` inserts between every character.
        let scrub = Scrub(vec![(String::new(), "X".to_string())]);
        assert_eq!(normalize("abc", &scrub), v(&["abc"]));
    }

    #[test]
    fn normalize__drops_cargo_status_lines() {
        let raw = concat!(
            "   Compiling a v0.1.0 (/x)\n",
            "    Checking a v0.1.0 (/x)\n",
            "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.31s\n",
            "     Running unittests src/lib.rs (/x/deps/a-08506271f5924bd7)\n",
            "   Doc-tests a\n",
            "    Blocking waiting for file lock on build directory\n",
            "error: real\n",
        );
        assert_eq!(norm(raw), v(&["error: real"]));
    }

    #[test]
    fn normalize__keeps_a_verb_that_is_not_right_aligned() {
        // Only cargo indents its status verbs. A program's own line that
        // starts with `Running` is the program's.
        assert_eq!(norm("Running the numbers\n"), v(&["Running the numbers"]));
    }

    #[test]
    fn normalize__keeps_cargo_summary_lines() {
        // A reader sees these, and they do not vary between runs.
        let raw = concat!(
            "error[E0308]: mismatched types\n",
            "error: could not compile `a` (lib) due to 1 previous error\n",
            "warning: `a` (lib) generated 1 warning\n",
            "error: test failed, to rerun pass `--lib`\n",
        );
        assert_eq!(norm(raw).len(), 4);
    }

    #[test]
    fn normalize__removes_the_panicking_thread_id() {
        assert_eq!(
            norm("thread 'tests::a' (691424) panicked at src/lib.rs:31:9:"),
            v(&["thread 'tests::a' panicked at src/lib.rs:31:9:"])
        );
        // A thread with no ID (before Rust 1.91) is left alone.
        assert_eq!(
            norm("thread 'main' panicked at src/main.rs:1:1:"),
            v(&["thread 'main' panicked at src/main.rs:1:1:"])
        );
    }

    #[test]
    fn normalize__strips_test_timings() {
        assert_eq!(
            norm(
                "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s"
            ),
            v(&["test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"])
        );
    }

    #[test]
    fn normalize__sorts_parallel_test_lines() {
        // libtest reports tests as they finish. `verify` runs a recorded step
        // on one thread; this rule serves output recorded anywhere else.
        let raw =
            "running 3 tests\ntest c ... ok\ntest a ... FAILED\ntest b ... ok\n\ntest result: x\n";
        assert_eq!(
            norm(raw),
            v(&[
                "running 3 tests",
                "test a ... FAILED",
                "test b ... ok",
                "test c ... ok",
                "",
                "test result: x"
            ])
        );
    }

    #[test]
    fn normalize__trims_line_ends_and_outer_blanks() {
        // A snapshot must survive an editor.
        assert_eq!(norm("\n\nerror: x   \n\n"), v(&["error: x"]));
    }
}
