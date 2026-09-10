//! The cast — the "voice bible" every narrator builds by hand today, as a
//! value the kernel can check cues against.
//!
//! In the real crate the CLI reads `voices.toml` and projects it into these
//! types, exactly as `bower.toml` is projected into `RepoCatalog`. Only what
//! changes a pure computation crosses: ids, aliases, tones, and the voice
//! target. Reference-recording paths, session notes, and the like stay on the
//! shell side.

use std::collections::{BTreeMap, BTreeSet};

use crate::palette::Axes;
use crate::{Errors, VoiceError};

/// The id every unmarked passage belongs to.
pub const NARRATOR: &str = "narrator";

/// The controlled tone vocabulary. Controlled, because `wrly` in chapter 12
/// should be a build error rather than a mystery to the narrator, and
/// because the breakdown counts tones per character.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tones(pub BTreeSet<String>);

impl Tones {
    /// A starter vocabulary. Books extend or replace it in `voices.toml`.
    #[must_use]
    pub fn standard() -> Self {
        Self(
            [
                "neutral", "warm", "wry", "dry", "tender", "urgent", "tired", "angry", "afraid",
                "amused", "cold", "bright", "grave", "playful", "flat",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
        )
    }

    #[must_use]
    pub fn contains(&self, tone: &str) -> bool {
        self.0.contains(tone)
    }
}

/// One voice in the book. `id` is what cues name; `aliases` let prose-side
/// names (`"Mrs. Alvarez"`) resolve to the same voice.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    /// One line for the booth: "sixties, ex-dealer, never raises her voice".
    pub description: String,
    /// Tones this character falls back to when a cue names none.
    pub default_tones: Vec<String>,
    /// Where this voice should sit. `None` means "not yet designed" — the
    /// breakdown still counts it; the fit report skips it and says so.
    pub target: Option<Axes>,
}

impl Character {
    #[must_use]
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_target(mut self, target: Axes) -> Self {
        self.target = Some(target);
        self
    }

    #[must_use]
    pub fn with_tones(mut self, tones: &[&str]) -> Self {
        self.default_tones = tones.iter().map(|t| (*t).to_string()).collect();
        self
    }

    #[must_use]
    pub fn with_aliases(mut self, aliases: &[&str]) -> Self {
        self.aliases = aliases.iter().map(|a| (*a).to_string()).collect();
        self
    }
}

/// How a term is said. `respelling` is the booth form (`pee-kay-core`);
/// `ipa` is optional and exact.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Pronunciation {
    pub respelling: String,
    pub ipa: Option<String>,
}

/// The pronunciation guide: term → how to say it. The second document every
/// narrator prepares by hand.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Lexicon(pub BTreeMap<String, Pronunciation>);

impl Lexicon {
    #[must_use]
    pub fn with(mut self, term: &str, respelling: &str) -> Self {
        self.0.insert(
            term.to_string(),
            Pronunciation {
                respelling: respelling.to_string(),
                ipa: None,
            },
        );
        self
    }
}

/// Everything about voices the book declares. The narrator is always
/// present: if the book does not declare `narrator`, a default one is added
/// so an unannotated book still has a script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cast {
    pub characters: BTreeMap<String, Character>,
    pub tones: Tones,
    pub lexicon: Lexicon,
}

impl Default for Cast {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Cast {
    /// Build a cast from declared characters, adding the implicit narrator
    /// and validating. Duplicates and unknown default tones are reported;
    /// the first declaration of a duplicate id wins.
    #[must_use]
    pub fn new(declared: Vec<Character>) -> Self {
        Self::with_tones(declared, Tones::standard())
    }

    #[must_use]
    pub fn with_tones(declared: Vec<Character>, tones: Tones) -> Self {
        let mut characters = BTreeMap::new();
        for c in declared {
            characters.entry(c.id.clone()).or_insert(c);
        }
        characters
            .entry(NARRATOR.to_string())
            .or_insert_with(|| Character::new(NARRATOR, "Narrator").with_tones(&["neutral"]));
        Self {
            characters,
            tones,
            lexicon: Lexicon::default(),
        }
    }

    #[must_use]
    pub fn with_lexicon(mut self, lexicon: Lexicon) -> Self {
        self.lexicon = lexicon;
        self
    }

    /// Every problem with the cast itself, independent of any chapter.
    ///
    /// Call this once with the *declared* list (before `new` deduplicates)
    /// to catch `DuplicateCharacter`; `new` keeps the first and drops the
    /// rest so planning can continue.
    #[must_use]
    pub fn validate(declared: &[Character], tones: &Tones) -> Errors {
        let mut errors = Errors::default();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for c in declared {
            if !seen.insert(&c.id) {
                errors.push(VoiceError::DuplicateCharacter { id: c.id.clone() });
            }
            for t in &c.default_tones {
                if !tones.contains(t) {
                    errors.push(VoiceError::CastUnknownTone {
                        id: c.id.clone(),
                        tone: t.clone(),
                    });
                }
            }
        }
        for c in declared {
            for a in &c.aliases {
                if seen.contains(a.as_str()) && a != &c.id {
                    errors.push(VoiceError::DuplicateCharacter { id: a.clone() });
                }
            }
        }
        errors
    }

    /// Resolve an id or alias to the character's canonical id.
    #[must_use]
    pub fn resolve<'a>(&'a self, name: &str) -> Option<&'a str> {
        if let Some((id, _)) = self.characters.get_key_value(name) {
            return Some(id.as_str());
        }
        self.characters
            .values()
            .find(|c| c.aliases.iter().any(|a| a == name))
            .map(|c| c.id.as_str())
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Character> {
        self.characters.get(id)
    }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn cast__always_has_a_narrator() {
        let cast = Cast::new(vec![]);
        assert!(cast.get(NARRATOR).is_some());
    }

    #[test]
    fn cast__declared_narrator_is_kept() {
        let cast = Cast::new(vec![
            Character::new(NARRATOR, "Christoph").with_tones(&["dry"]),
        ]);
        assert_eq!(
            cast.get(NARRATOR).map(|c| c.name.as_str()),
            Some("Christoph")
        );
    }

    #[test]
    fn resolve__follows_aliases() {
        let cast = Cast::new(vec![
            Character::new("rosa", "Rosa Alvarez").with_aliases(&["mrs-alvarez"]),
        ]);
        assert_eq!(cast.resolve("mrs-alvarez"), Some("rosa"));
        assert_eq!(cast.resolve("rosa"), Some("rosa"));
        assert_eq!(cast.resolve("nobody"), None);
    }

    #[test]
    fn validate__names_duplicates_and_unknown_tones() {
        let declared = vec![
            Character::new("rosa", "Rosa"),
            Character::new("rosa", "Rosa again"),
            Character::new("tom", "Tom").with_tones(&["smarmy"]),
        ];
        let errs = Cast::validate(&declared, &Tones::standard());
        assert!(errs.contains(&VoiceError::DuplicateCharacter { id: "rosa".into() }));
        assert!(errs.contains(&VoiceError::CastUnknownTone {
            id: "tom".into(),
            tone: "smarmy".into()
        }));
        assert_eq!(errs.len(), 2);
    }

    #[test]
    fn validate__an_alias_that_shadows_an_id_is_a_duplicate() {
        let declared = vec![
            Character::new("rosa", "Rosa"),
            Character::new("tom", "Tom").with_aliases(&["rosa"]),
        ];
        let errs = Cast::validate(&declared, &Tones::standard());
        assert!(errs.contains(&VoiceError::DuplicateCharacter { id: "rosa".into() }));
    }
}
