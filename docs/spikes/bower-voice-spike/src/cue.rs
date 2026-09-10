//! The `<!-- voice … -->` cue: grammar, keys, and parser.
//!
//! A cue is a single-line HTML comment whose first word is `voice` (or the
//! drafting alias `vo`), followed by `key="value"` pairs. Like Bower's
//! `<!-- bower … -->` directive it is invisible in mdBook, pandoc, and GitHub
//! preview, so a book annotated for the ear renders for the eye untouched —
//! and `bower-core` never sees it, because its directive word differs.
//!
//! Three forms:
//!
//! ```markdown
//! <!-- voice speaker="rosa" tone="wry" -->          one-shot: the next paragraph
//! <!-- voice begin speaker="dealer" pace="brisk" --> opens a span …
//! <!-- voice end -->                                  … closed here
//! ```

use crate::source::Location;
use crate::{Errors, VoiceError};

/// How fast the passage should move. The wire forms are lowercase.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Pace {
    Slow,
    #[default]
    Measured,
    Brisk,
    Rushed,
}

impl Pace {
    /// Every pace, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [Pace; 4] {
        [Self::Slow, Self::Measured, Self::Brisk, Self::Rushed]
    }
}

impl std::fmt::Display for Pace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Slow => "slow",
            Self::Measured => "measured",
            Self::Brisk => "brisk",
            Self::Rushed => "rushed",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for Pace {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "slow" => Ok(Self::Slow),
            "measured" => Ok(Self::Measured),
            "brisk" => Ok(Self::Brisk),
            "rushed" => Ok(Self::Rushed),
            other => Err(other.to_string()),
        }
    }
}

/// How much air is behind the passage.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Energy {
    Whisper,
    Soft,
    #[default]
    Level,
    Loud,
    Shout,
}

impl Energy {
    /// Every energy, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [Energy; 5] {
        [
            Self::Whisper,
            Self::Soft,
            Self::Level,
            Self::Loud,
            Self::Shout,
        ]
    }
}

impl std::fmt::Display for Energy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Whisper => "whisper",
            Self::Soft => "soft",
            Self::Level => "level",
            Self::Loud => "loud",
            Self::Shout => "shout",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for Energy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "whisper" => Ok(Self::Whisper),
            "soft" => Ok(Self::Soft),
            "level" => Ok(Self::Level),
            "loud" => Ok(Self::Loud),
            "shout" => Ok(Self::Shout),
            other => Err(other.to_string()),
        }
    }
}

/// A pause *before* the passage. The booth vocabulary: a beat, a breath, or
/// a long hold.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Pause {
    Beat,
    Breath,
    Long,
}

impl Pause {
    /// Every pause, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [Pause; 3] {
        [Self::Beat, Self::Breath, Self::Long]
    }
}

impl std::fmt::Display for Pause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Beat => "beat",
            Self::Breath => "breath",
            Self::Long => "long",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for Pause {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "beat" => Ok(Self::Beat),
            "breath" => Ok(Self::Breath),
            "long" => Ok(Self::Long),
            other => Err(other.to_string()),
        }
    }
}

/// Which of the three cue forms a line is.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CueForm {
    /// Applies to the next paragraph only.
    OneShot,
    /// Opens a span that runs to the matching `voice end`.
    Begin,
    /// Closes the open span. Carries no keys.
    End,
}

impl CueForm {
    /// Every form, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [CueForm; 3] {
        [Self::OneShot, Self::Begin, Self::End]
    }
}

/// One parsed cue, keys as written. Nothing is validated against the cast
/// yet — that happens in [`crate::script`] where the cue meets its text.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CueDirective {
    pub form: Option<CueForm>,
    pub speaker: Option<String>,
    /// `tone="wry,tired"` — zero or more, order preserved, trimmed, empties dropped.
    pub tones: Vec<String>,
    pub pace: Option<Pace>,
    pub energy: Option<Energy>,
    pub pause: Option<Pause>,
    /// Free-text direction for the narrator: `note="she is lying"`.
    pub note: Option<String>,
}

impl CueDirective {
    /// Is this trimmed line a cue comment? (Cheap pre-check.)
    #[must_use]
    pub fn is_cue_line(line: &str) -> bool {
        let t = line.trim();
        if !t.starts_with("<!--") || !t.ends_with("-->") {
            return false;
        }
        let inner = t.trim_start_matches("<!--").trim_end_matches("-->").trim();
        let word = inner.split_whitespace().next().unwrap_or("");
        word == "voice" || word == "vo"
    }

    /// The form of this cue: `OneShot` unless `begin`/`end` is present.
    #[must_use]
    pub fn form(&self) -> CueForm {
        self.form.unwrap_or(CueForm::OneShot)
    }

    /// Does the cue direct anything at all?
    #[must_use]
    pub fn has_keys(&self) -> bool {
        self.speaker.is_some()
            || !self.tones.is_empty()
            || self.pace.is_some()
            || self.energy.is_some()
            || self.pause.is_some()
            || self.note.is_some()
    }

    /// Parse one cue line. `loc` is where the line sits in the book.
    ///
    /// # Errors
    ///
    /// Collects [`VoiceError::CueParse`], [`VoiceError::UnknownKey`],
    /// [`VoiceError::BadValue`], and [`VoiceError::EndWithKeys`] — everything
    /// wrong with the line at once.
    pub fn parse(line: &str, loc: &Location) -> Result<Self, Errors> {
        let mut errors = Errors::default();
        let t = line.trim();
        let inner = t.trim_start_matches("<!--").trim_end_matches("-->").trim();
        let mut rest = inner
            .strip_prefix("voice")
            .or_else(|| inner.strip_prefix("vo"))
            .unwrap_or(inner)
            .trim();

        let mut d = Self::default();
        if let Some(r) = rest.strip_prefix("begin") {
            d.form = Some(CueForm::Begin);
            rest = r.trim();
        } else if let Some(r) = rest.strip_prefix("end") {
            d.form = Some(CueForm::End);
            rest = r.trim();
        }

        let pairs: Vec<(String, String)> = KeyValues::new(rest, loc, &mut errors).collect();
        for (key, value) in pairs {
            match key.as_str() {
                "speaker" => d.speaker = Some(value),
                "tone" => {
                    d.tones = value
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                }
                "pace" => match value.parse::<Pace>() {
                    Ok(p) => d.pace = Some(p),
                    Err(v) => errors.push(bad(loc, "pace", v)),
                },
                "energy" => match value.parse::<Energy>() {
                    Ok(e) => d.energy = Some(e),
                    Err(v) => errors.push(bad(loc, "energy", v)),
                },
                "pause" => match value.parse::<Pause>() {
                    Ok(p) => d.pause = Some(p),
                    Err(v) => errors.push(bad(loc, "pause", v)),
                },
                "note" => d.note = Some(value),
                unknown => errors.push(VoiceError::UnknownKey {
                    loc: loc.clone(),
                    key: unknown.to_string(),
                }),
            }
        }

        if d.form == Some(CueForm::End) && d.has_keys() {
            errors.push(VoiceError::EndWithKeys { loc: loc.clone() });
        }

        if errors.is_empty() {
            Ok(d)
        } else {
            Err(errors)
        }
    }
}

fn bad(loc: &Location, key: &str, value: String) -> VoiceError {
    VoiceError::BadValue {
        loc: loc.clone(),
        key: key.to_string(),
        value,
    }
}

/// Iterator over `key="value"` pairs, reporting malformed text as errors on
/// the shared collector rather than stopping the scan. Same grammar as
/// `bower-core`'s directive parser; the real crate should call that one.
struct KeyValues<'a> {
    rest: &'a str,
    loc: &'a Location,
    errors: &'a mut Errors,
}

impl<'a> KeyValues<'a> {
    fn new(rest: &'a str, loc: &'a Location, errors: &'a mut Errors) -> Self {
        Self { rest, loc, errors }
    }

    fn fail(&mut self, reason: &str) -> Option<(String, String)> {
        self.errors.push(VoiceError::CueParse {
            loc: self.loc.clone(),
            reason: reason.to_string(),
        });
        self.rest = "";
        None
    }
}

impl Iterator for KeyValues<'_> {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        self.rest = self.rest.trim_start();
        if self.rest.is_empty() {
            return None;
        }
        let Some(eq) = self.rest.find('=') else {
            return self.fail("expected `key=\"value\"`");
        };
        let key = self.rest[..eq].trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return self.fail(&format!("bad key `{key}`"));
        }
        let after = self.rest[eq + 1..].trim_start();
        let Some(body) = after.strip_prefix('"') else {
            return self.fail(&format!("value for `{key}` must be double-quoted"));
        };
        let Some(close) = body.find('"') else {
            return self.fail(&format!("unterminated value for `{key}`"));
        };
        let value = body[..close].to_string();
        self.rest = &body[close + 1..];
        Some((key.to_string(), value))
    }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn at() -> Location {
        Location::new("src/ch01.md", 7)
    }

    #[test]
    fn one_shot__parses_every_key() {
        let d = CueDirective::parse(
            r#"<!-- voice speaker="rosa" tone="wry, tired" pace="brisk" energy="soft" pause="beat" note="lying" -->"#,
            &at(),
        )
        .unwrap();
        assert_eq!(d.form(), CueForm::OneShot);
        assert_eq!(d.speaker.as_deref(), Some("rosa"));
        assert_eq!(d.tones, vec!["wry", "tired"]);
        assert_eq!(d.pace, Some(Pace::Brisk));
        assert_eq!(d.energy, Some(Energy::Soft));
        assert_eq!(d.pause, Some(Pause::Beat));
        assert_eq!(d.note.as_deref(), Some("lying"));
    }

    #[test]
    fn alias__vo_is_accepted() {
        assert!(CueDirective::is_cue_line(r#"<!-- vo speaker="x" -->"#));
        assert!(!CueDirective::is_cue_line(r#"<!-- bower repo="x" -->"#));
        assert!(!CueDirective::is_cue_line("<!-- voices are nice -->"));
    }

    #[test]
    fn begin_and_end__are_forms() {
        let b = CueDirective::parse(r#"<!-- voice begin speaker="dealer" -->"#, &at()).unwrap();
        assert_eq!(b.form(), CueForm::Begin);
        let e = CueDirective::parse("<!-- voice end -->", &at()).unwrap();
        assert_eq!(e.form(), CueForm::End);
    }

    #[test]
    fn end__with_keys_is_an_error() {
        let errs = CueDirective::parse(r#"<!-- voice end speaker="x" -->"#, &at()).unwrap_err();
        assert!(errs.contains(&VoiceError::EndWithKeys { loc: at() }));
    }

    #[test]
    fn errors__are_collected_not_short_circuited() {
        let errs =
            CueDirective::parse(r#"<!-- voice pace="warp" colour="red" -->"#, &at()).unwrap_err();
        assert_eq!(errs.len(), 2);
        assert!(errs.contains(&VoiceError::BadValue {
            loc: at(),
            key: "pace".into(),
            value: "warp".into()
        }));
        assert!(errs.contains(&VoiceError::UnknownKey {
            loc: at(),
            key: "colour".into()
        }));
    }

    #[test]
    fn parse__unquoted_value_is_a_located_parse_error() {
        let errs = CueDirective::parse("<!-- voice speaker=rosa -->", &at()).unwrap_err();
        assert!(matches!(errs.0[0], VoiceError::CueParse { .. }));
        assert_eq!(errs.0[0].location(), Some(&at()));
    }
}
