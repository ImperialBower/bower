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

/// A recorded line that is exactly this stands for zero or more live lines —
/// the editorial ellipsis of any quotation. Not `...`: rustc prints that in
/// the gutter of a long multi-line span (EPIC-11 Decision 5).
pub const ELISION: &str = "[...]";

/// The first place a recording and the live output disagree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Drift {
    /// 1-based, within the recorded lines.
    pub line: usize,
    /// The recorded line there; `None` when the recording has ended.
    pub recorded: Option<String>,
    /// The live line there; `None` when the output has ended, or when an
    /// unanchored piece appears nowhere after the piece before it.
    pub live: Option<String>,
}

/// Does the recording hold against the live output?
///
/// Without a `[...]` line the two must be equal. With one, the recording is
/// split into pieces at each `[...]`; every piece must appear as consecutive
/// live lines, in order. A piece with no `[...]` before it must start the
/// output, and one with none after it must end it. An empty recording is not
/// judged here: the caller reports it as not recorded yet.
#[must_use]
pub fn drift(recorded: &[String], live: &[String]) -> Option<Drift> {
    if !recorded.iter().any(|l| l == ELISION) {
        return compare_at(recorded, 0, live, 0, recorded.len().max(live.len()));
    }

    // (index of the piece's first line in `recorded`, the piece)
    let mut pieces: Vec<(usize, &[String])> = Vec::new();
    let mut start = 0;
    for (i, line) in recorded.iter().enumerate() {
        if line == ELISION {
            if i > start {
                pieces.push((start, &recorded[start..i]));
            }
            start = i + 1;
        }
    }
    if start < recorded.len() {
        pieces.push((start, &recorded[start..]));
    }

    let anchored_start = recorded.first().is_some_and(|l| l != ELISION);
    let anchored_end = recorded.last().is_some_and(|l| l != ELISION);
    let last = pieces.len().saturating_sub(1);
    let mut pos = 0;

    for (k, &(start, piece)) in pieces.iter().enumerate() {
        if k == 0 && anchored_start {
            if let Some(d) = compare_at(piece, start, live, 0, piece.len()) {
                return Some(d);
            }
            pos = piece.len();
        } else if k == last && anchored_end {
            if live.len() < pos + piece.len() {
                return Some(Drift {
                    line: start + 1,
                    recorded: Some(piece[0].clone()),
                    live: None,
                });
            }
            let at = live.len() - piece.len();
            if let Some(d) = compare_at(piece, start, live, at, piece.len()) {
                return Some(d);
            }
            pos = live.len();
        } else {
            let found = (pos..=live.len().saturating_sub(piece.len()))
                .find(|&at| live.get(at..at + piece.len()) == Some(piece));
            let Some(at) = found else {
                return Some(Drift {
                    line: start + 1,
                    recorded: Some(piece[0].clone()),
                    live: None,
                });
            };
            pos = at + piece.len();
        }
    }
    None
}

/// Compare `piece` (which starts at recorded index `start`) against `live`
/// from index `at`, for `len` lines — longer than the piece when the live
/// output must also end where it does.
fn compare_at(
    piece: &[String],
    start: usize,
    live: &[String],
    at: usize,
    len: usize,
) -> Option<Drift> {
    (0..len).find_map(|i| {
        let r = piece.get(i);
        let l = live.get(at + i);
        (r != l).then(|| Drift {
            line: start + i + 1,
            recorded: r.cloned(),
            live: l.cloned(),
        })
    })
}

/// The rustc error codes an output names, from its `error[E0004]` and
/// `warning[E0004]` headlines: first-seen order, each once. The one parser
/// both the render caption and EPIC-12's failure index use.
#[must_use]
pub fn error_codes(lines: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        let t = line.trim_start();
        let Some(rest) = t
            .strip_prefix("error[")
            .or_else(|| t.strip_prefix("warning["))
        else {
            continue;
        };
        let Some(end) = rest.find(']') else { continue };
        let code = &rest[..end];
        let is_code = code.len() == 5
            && code.starts_with('E')
            && code[1..].bytes().all(|b| b.is_ascii_digit());
        if is_code && !out.iter().any(|c| c == code) {
            out.push(code.to_string());
        }
    }
    out
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

    #[test]
    fn drift__none_when_equal() {
        assert_eq!(drift(&v(&["a", "b"]), &v(&["a", "b"])), None);
    }

    #[test]
    fn drift__names_the_first_differing_line() {
        assert_eq!(
            drift(&v(&["a", "b", "c"]), &v(&["a", "x", "c"])),
            Some(Drift {
                line: 2,
                recorded: Some("b".into()),
                live: Some("x".into())
            })
        );
    }

    #[test]
    fn drift__a_shorter_live_output_is_drift() {
        assert_eq!(
            drift(&v(&["a", "b"]), &v(&["a"])),
            Some(Drift {
                line: 2,
                recorded: Some("b".into()),
                live: None
            })
        );
    }

    #[test]
    fn drift__a_longer_live_output_is_drift() {
        // A new note at the end is a rewording too. Only `[...]` excuses it.
        assert_eq!(
            drift(&v(&["a"]), &v(&["a", "b"])),
            Some(Drift {
                line: 2,
                recorded: None,
                live: Some("b".into())
            })
        );
    }

    #[test]
    fn drift__elision_matches_a_middle_piece() {
        let live = v(&[
            "w1",
            "w2",
            "thread panicked",
            "left: 1",
            "right: 2",
            "summary",
        ]);
        let recorded = v(&[ELISION, "thread panicked", "left: 1", "right: 2", ELISION]);
        assert_eq!(drift(&recorded, &live), None);
    }

    #[test]
    fn drift__elision_at_neither_end_anchors_both() {
        let live = v(&["head", "middle", "tail"]);
        assert_eq!(drift(&v(&["head", ELISION, "tail"]), &live), None);
        assert_eq!(drift(&v(&["head", ELISION]), &live), None);
        assert_eq!(drift(&v(&[ELISION, "tail"]), &live), None);
        // Anchored: `middle` is neither the first line nor the last.
        assert_eq!(
            drift(&v(&["middle", ELISION]), &live),
            Some(Drift {
                line: 1,
                recorded: Some("middle".into()),
                live: Some("head".into())
            })
        );
        assert_eq!(
            drift(&v(&[ELISION, "middle"]), &live),
            Some(Drift {
                line: 2,
                recorded: Some("middle".into()),
                live: Some("tail".into())
            })
        );
    }

    #[test]
    fn drift__elided_pieces_must_keep_their_order() {
        let live = v(&["a", "b", "c"]);
        assert_eq!(
            drift(&v(&[ELISION, "a", ELISION, "c", ELISION]), &live),
            None
        );
        assert_eq!(
            drift(&v(&[ELISION, "c", ELISION, "a", ELISION]), &live),
            Some(Drift {
                line: 4,
                recorded: Some("a".into()),
                live: None
            })
        );
    }

    #[test]
    fn drift__an_elided_piece_that_is_gone_names_its_first_line() {
        assert_eq!(
            drift(&v(&[ELISION, "z", "y", ELISION]), &v(&["a", "b"])),
            Some(Drift {
                line: 2,
                recorded: Some("z".into()),
                live: None
            })
        );
    }

    #[test]
    fn drift__an_elision_may_stand_for_no_lines() {
        assert_eq!(drift(&v(&["a", ELISION, "b"]), &v(&["a", "b"])), None);
        assert_eq!(drift(&v(&[ELISION]), &v(&["x"])), None);
    }

    #[test]
    fn drift__anchored_ends_may_not_overlap() {
        // `a [...] a` against the single line `a`: both ends cannot claim it.
        assert_eq!(
            drift(&v(&["a", ELISION, "a"]), &v(&["a"])),
            Some(Drift {
                line: 3,
                recorded: Some("a".into()),
                live: None
            })
        );
    }

    #[test]
    fn error_codes__in_order_once_each() {
        let lines = v(&[
            "error[E0308]: mismatched types",
            "  = note: see E0004",
            "error[E0004]: non-exhaustive patterns",
            "error[E0308]: again",
            "warning: unused variable",
            "For more information about this error, try `rustc --explain E0308`.",
        ]);
        assert_eq!(error_codes(&lines), vec!["E0308", "E0004"]);
    }

    #[test]
    fn error_codes__only_headline_codes_count() {
        let lines = v(&[
            "error[E12]: short",
            "error[X0001]: not rustc",
            "error: plain",
            "error[E0004",
        ]);
        assert!(error_codes(&lines).is_empty());
    }
}
