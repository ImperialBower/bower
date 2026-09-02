//! `bower publish` — one render, several targets (spec § 13 M1).
//!
//! Publishing is a pure fold from a book and its plan to a *render plan*, with
//! renderers as replaceable I/O at the edges — the same relationship
//! `TreeState` has with git. This module owns the fold; the renderers live
//! beside it.
//!
//! As of EPIC-06 Phase 0 it owns [`Target`] alone, which is the one thing the
//! render actually varies on.

use std::fmt;

/// What is being produced.
///
/// Threaded through the render because the elision rule differs, and only
/// because of that. A boolean would do the job today; an enum is right anyway,
/// because spec § 13 names `pdf` and `ipynb` as the next two rungs and a `bool`
/// called `is_html` is a variable that stops being true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// mdBook HTML. Rust fences keep the eye-toggle a reader expands in place.
    Html,
    /// pandoc epub. No toggle exists anywhere in an epub, so every elision —
    /// Rust included — collapses to a comment naming what was left out.
    Epub,
}

impl Target {
    /// Whether this target can hide code behind a toggle the reader expands.
    ///
    /// True only for [`Target::Html`], and even there only Rust fences use it:
    /// mdBook's hidden-line rule is a Rust feature, not a markdown one.
    #[must_use]
    pub fn has_hidden_lines(self) -> bool {
        matches!(self, Self::Html)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Html => "html",
            Self::Epub => "epub",
        })
    }
}

impl std::str::FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(Self::Html),
            "epub" => Ok(Self::Epub),
            other => Err(format!(
                "unknown target `{other}` — this build knows html and epub \
                 (pdf and ipynb are spec § 13's next rungs, not built yet)"
            )),
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod publish_tests {
    use super::*;

    #[test]
    fn target__only_html_hides_lines() {
        assert!(Target::Html.has_hidden_lines());
        assert!(!Target::Epub.has_hidden_lines());
    }

    #[test]
    fn target__round_trips_through_its_name() {
        for t in [Target::Html, Target::Epub] {
            assert_eq!(t.to_string().parse::<Target>().unwrap(), t);
        }
    }

    #[test]
    fn target__an_unbuilt_target_says_which_are_built() {
        let err = "pdf".parse::<Target>().unwrap_err();
        assert!(err.contains("html and epub"), "{err}");
    }
}
