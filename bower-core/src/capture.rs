//! What a command printed, made comparable (EPIC-11).
//!
//! Pure string functions: the edge runs the command and hands the text in.
//! Nothing here knows about cargo beyond the shape of its output lines.

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
}
