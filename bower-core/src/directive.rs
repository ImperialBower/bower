//! The `<!-- bower … -->` directive: grammar, keys, and parser.
//!
//! A directive is a single-line HTML comment whose first word is `bower`
//! (or the drafting alias `bf`), followed by `key="value"` pairs. HTML
//! comments are invisible in mdBook, pandoc, and GitHub preview alike,
//! which is the whole point.

use crate::source::Location;
use crate::{BowerError, Errors};

/// What a block does to the repo tree. The wire form of [`Op::Prose`] is
/// `none` — a step that exists only to carry a commit message for work the
/// book narrates without printing.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Op {
    #[default]
    Create,
    Replace,
    Region,
    Append,
    Delete,
    Copy,
    Prose,
}

impl Op {
    /// Ops that require a following fenced code block.
    #[must_use]
    pub fn needs_block(self) -> bool {
        matches!(
            self,
            Self::Create | Self::Replace | Self::Region | Self::Append
        )
    }

    /// Ops that require a `file` key (directly or via `paths`).
    #[must_use]
    pub fn needs_file(self) -> bool {
        !matches!(self, Self::Prose)
    }

    /// Every op, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [Op; 7] {
        [
            Self::Create,
            Self::Replace,
            Self::Region,
            Self::Append,
            Self::Delete,
            Self::Copy,
            Self::Prose,
        ]
    }
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Create => "create",
            Self::Replace => "replace",
            Self::Region => "region",
            Self::Append => "append",
            Self::Delete => "delete",
            Self::Copy => "copy",
            Self::Prose => "none",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for Op {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create" => Ok(Self::Create),
            "replace" => Ok(Self::Replace),
            "region" => Ok(Self::Region),
            "append" => Ok(Self::Append),
            "delete" => Ok(Self::Delete),
            "copy" => Ok(Self::Copy),
            "none" => Ok(Self::Prose),
            other => Err(other.to_string()),
        }
    }
}

/// What the verifier asserts at this step's tree state. The wire form of
/// [`Expect::Skip`] is `none`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Expect {
    #[default]
    Pass,
    CompileFail,
    TestFail,
    Skip,
}

impl Expect {
    /// Every expectation, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [Expect; 4] {
        [Self::Pass, Self::CompileFail, Self::TestFail, Self::Skip]
    }
}

impl std::fmt::Display for Expect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pass => "pass",
            Self::CompileFail => "compile_fail",
            Self::TestFail => "test_fail",
            Self::Skip => "none",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for Expect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pass" => Ok(Self::Pass),
            "compile_fail" => Ok(Self::CompileFail),
            "test_fail" => Ok(Self::TestFail),
            "none" => Ok(Self::Skip),
            other => Err(other.to_string()),
        }
    }
}

/// One parsed directive, keys as written. Nothing is defaulted or validated
/// against the catalog yet — that happens in [`crate::block`] where the
/// directive meets its code block and the repo catalog.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Directive {
    pub repo: Option<String>,
    pub file: Option<String>,
    pub op: Option<Op>,
    pub step: Option<String>,
    pub msg: Option<String>,
    pub expect: Option<Expect>,
    pub region: Option<String>,
    pub src: Option<String>,
    pub hidden: Option<bool>,
    pub show: Option<String>,
    pub include: Option<String>,
    pub after: Option<String>,
    /// `notebook="play"` — a live notebook cell, not repo content (§ 15).
    pub notebook: Option<String>,
    /// `paths="a,b"` — multi-file `op="delete"`.
    pub paths: Vec<String>,
}

impl Directive {
    /// Is this trimmed line a directive comment? (Cheap pre-check.)
    #[must_use]
    pub fn is_directive_line(line: &str) -> bool {
        let t = line.trim();
        if !t.starts_with("<!--") || !t.ends_with("-->") {
            return false;
        }
        let inner = t.trim_start_matches("<!--").trim_end_matches("-->").trim();
        let word = inner.split_whitespace().next().unwrap_or("");
        word == "bower" || word == "bf"
    }

    /// Parse one directive line. `loc` is where the line sits in the book.
    ///
    /// # Errors
    ///
    /// Collects [`BowerError::DirectiveParse`], [`BowerError::UnknownKey`],
    /// and [`BowerError::BadValue`] — everything wrong with the line at once.
    pub fn parse(line: &str, loc: &Location) -> Result<Self, Errors> {
        let mut errors = Errors::default();
        let t = line.trim();
        let inner = t.trim_start_matches("<!--").trim_end_matches("-->").trim();
        let rest = inner
            .strip_prefix("bower")
            .or_else(|| inner.strip_prefix("bf"))
            .unwrap_or(inner)
            .trim();

        let pairs: Vec<(String, String)> = KeyValues::new(rest, loc, &mut errors).collect();
        let mut d = Self::default();
        for (key, value) in pairs {
            match key.as_str() {
                "repo" => d.repo = Some(value),
                "file" => d.file = Some(value),
                "op" => match value.parse::<Op>() {
                    Ok(op) => d.op = Some(op),
                    Err(v) => errors.push(BowerError::BadValue {
                        loc: loc.clone(),
                        key: "op".to_string(),
                        value: v,
                    }),
                },
                "step" => d.step = Some(value),
                "msg" => d.msg = Some(value),
                "expect" => match value.parse::<Expect>() {
                    Ok(e) => d.expect = Some(e),
                    Err(v) => errors.push(BowerError::BadValue {
                        loc: loc.clone(),
                        key: "expect".to_string(),
                        value: v,
                    }),
                },
                "region" => d.region = Some(value),
                "src" => d.src = Some(value),
                "hidden" => match value.as_str() {
                    "true" => d.hidden = Some(true),
                    "false" => d.hidden = Some(false),
                    other => errors.push(BowerError::BadValue {
                        loc: loc.clone(),
                        key: "hidden".to_string(),
                        value: other.to_string(),
                    }),
                },
                "show" => d.show = Some(value),
                "include" => d.include = Some(value),
                "after" => d.after = Some(value),
                "notebook" => {
                    if value == "play" {
                        d.notebook = Some(value);
                    } else {
                        errors.push(BowerError::BadValue {
                            loc: loc.clone(),
                            key: "notebook".to_string(),
                            value,
                        });
                    }
                }
                "paths" => {
                    d.paths = value
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                }
                unknown => errors.push(BowerError::UnknownKey {
                    loc: loc.clone(),
                    key: unknown.to_string(),
                }),
            }
        }

        if errors.is_empty() {
            Ok(d)
        } else {
            Err(errors)
        }
    }

    /// Overlay `self` on `defaults`: any key set here wins; unset keys fall
    /// through. Used when a chapter directive includes a library block that
    /// carries its own defaults.
    #[must_use]
    pub fn overlaid_on(&self, defaults: &Self) -> Self {
        Self {
            repo: self.repo.clone().or_else(|| defaults.repo.clone()),
            file: self.file.clone().or_else(|| defaults.file.clone()),
            op: self.op.or(defaults.op),
            step: self.step.clone().or_else(|| defaults.step.clone()),
            msg: self.msg.clone().or_else(|| defaults.msg.clone()),
            expect: self.expect.or(defaults.expect),
            region: self.region.clone().or_else(|| defaults.region.clone()),
            src: self.src.clone().or_else(|| defaults.src.clone()),
            hidden: self.hidden.or(defaults.hidden),
            show: self.show.clone().or_else(|| defaults.show.clone()),
            include: None, // an include never chains to another include
            after: self.after.clone().or_else(|| defaults.after.clone()),
            notebook: self.notebook.clone().or_else(|| defaults.notebook.clone()),
            paths: if self.paths.is_empty() {
                defaults.paths.clone()
            } else {
                self.paths.clone()
            },
        }
    }
}

/// Iterator over `key="value"` pairs, reporting malformed text as errors on
/// the shared collector rather than stopping the scan.
struct KeyValues<'a> {
    rest: &'a str,
    loc: &'a Location,
    errors: &'a mut Errors,
}

impl<'a> KeyValues<'a> {
    fn new(rest: &'a str, loc: &'a Location, errors: &'a mut Errors) -> Self {
        Self { rest, loc, errors }
    }
}

impl Iterator for KeyValues<'_> {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.rest = self.rest.trim_start();
            if self.rest.is_empty() {
                return None;
            }
            let Some(eq) = self.rest.find('=') else {
                self.errors.push(BowerError::DirectiveParse {
                    loc: self.loc.clone(),
                    reason: format!("expected key=\"value\", found `{}`", self.rest),
                });
                self.rest = "";
                return None;
            };
            let key = self.rest[..eq].trim().to_string();
            let after_eq = self.rest[eq + 1..].trim_start();
            let Some(stripped) = after_eq.strip_prefix('"') else {
                self.errors.push(BowerError::DirectiveParse {
                    loc: self.loc.clone(),
                    reason: format!("value for `{key}` must be double-quoted"),
                });
                self.rest = "";
                return None;
            };
            let Some(close) = stripped.find('"') else {
                self.errors.push(BowerError::DirectiveParse {
                    loc: self.loc.clone(),
                    reason: format!("unterminated value for `{key}`"),
                });
                self.rest = "";
                return None;
            };
            let value = stripped[..close].to_string();
            self.rest = &stripped[close + 1..];
            if key.is_empty() {
                self.errors.push(BowerError::DirectiveParse {
                    loc: self.loc.clone(),
                    reason: "empty key before `=`".to_string(),
                });
                continue;
            }
            return Some((key, value));
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod directive_tests {
    use super::*;
    use rstest::rstest;

    fn loc() -> Location {
        Location::new("ch01.md", 1)
    }

    #[rstest]
    #[case("<!-- bower repo=\"failers\" -->", true)]
    #[case("<!-- bf repo=\"failers\" -->", true)]
    #[case("  <!-- bower -->  ", true)]
    #[case("<!-- note to self -->", false)]
    #[case("plain prose", false)]
    fn is_directive_line__recognizes(#[case] line: &str, #[case] expected: bool) {
        assert_eq!(Directive::is_directive_line(line), expected);
    }

    #[test]
    fn parse__full_key_set() {
        let d = Directive::parse(
            "<!-- bower repo=\"failers\" file=\"src/rank.rs\" op=\"region\" region=\"from_char\" step=\"rank\" msg=\"m\" expect=\"compile_fail\" hidden=\"false\" show=\"a\" after=\"init\" -->",
            &loc(),
        )
        .unwrap();
        assert_eq!(d.repo.as_deref(), Some("failers"));
        assert_eq!(d.file.as_deref(), Some("src/rank.rs"));
        assert_eq!(d.op, Some(Op::Region));
        assert_eq!(d.region.as_deref(), Some("from_char"));
        assert_eq!(d.expect, Some(Expect::CompileFail));
        assert_eq!(d.hidden, Some(false));
        assert_eq!(d.after.as_deref(), Some("init"));
    }

    #[test]
    fn parse__paths_splits_on_commas() {
        let d = Directive::parse(
            "<!-- bower repo=\"r\" op=\"delete\" paths=\"a.rs, b.rs\" -->",
            &loc(),
        )
        .unwrap();
        assert_eq!(d.paths, vec!["a.rs".to_string(), "b.rs".to_string()]);
    }

    #[test]
    fn parse__unknown_key_and_bad_op_both_reported() {
        let errs = Directive::parse(
            "<!-- bower repo=\"r\" flavor=\"salt\" op=\"explode\" -->",
            &loc(),
        )
        .unwrap_err();
        assert_eq!(errs.len(), 2);
    }

    #[test]
    fn parse__unterminated_value_is_an_error() {
        let errs = Directive::parse("<!-- bower repo=\"r -->", &loc()).unwrap_err();
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn op__display_from_str_round_trips() {
        for op in Op::all() {
            assert_eq!(op.to_string().parse::<Op>().unwrap(), op);
        }
        for e in Expect::all() {
            assert_eq!(e.to_string().parse::<Expect>().unwrap(), e);
        }
    }

    #[test]
    fn overlaid_on__self_wins_defaults_fill() {
        let lib = Directive {
            repo: Some("failers".into()),
            file: Some("a.rs".into()),
            ..Directive::default()
        };
        let chapter = Directive {
            file: Some("b.rs".into()),
            ..Directive::default()
        };
        let merged = chapter.overlaid_on(&lib);
        assert_eq!(merged.repo.as_deref(), Some("failers"));
        assert_eq!(merged.file.as_deref(), Some("b.rs"));
    }
}
