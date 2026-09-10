//! # bower-voice — the Bower voice kernel (spike)
//!
//! Bower already treats a book as the single source of truth for its code.
//! This crate extends the same idea to the *ear*: the book is also the single
//! source of truth for how it should be read aloud.
//!
//! Three things fall out of one pure fold over the chapters:
//!
//! * a **script** — every passage, in order, with the speaker and the
//!   delivery the author asked for ([`script::Script`]);
//! * a **breakdown** — who speaks, how much, where they first appear, and how
//!   long the book will run in the booth ([`breakdown::Breakdown`]);
//! * a **fit** — how the cast sits inside one narrator's palette, which
//!   characters collide, and which are out of reach ([`palette::FitReport`]).
//!
//! The kernel's contract is `bower-core`'s, unchanged:
//!
//! * **No I/O.** Chapters, cast, and palette are values handed in by the
//!   caller. No filesystem, no audio, no network.
//! * **No serialization in the public API.** Report *text* is a `String`.
//! * **Deterministic.** Equal input, byte-identical script and reports.
//! * **Exhaustive, located errors.** Every failure is a [`VoiceError`] with a
//!   chapter and line; errors are collected, not short-circuited.
//!
//! In the real crate `Chapter` and `Location` come from `bower-core` so there
//! is one definition of "where in the book". The spike carries copies to stay
//! zero-dependency; see `source.rs`.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::module_name_repetitions, clippy::missing_panics_doc)]

pub mod breakdown;
pub mod cast;
pub mod cue;
pub mod palette;
pub mod script;
pub mod source;

pub mod prelude {
    //! `use bower_voice_spike::prelude::*;` — the one import a consumer needs.
    pub use crate::VoiceError;
    pub use crate::breakdown::{Breakdown, breakdown, breakdown_text};
    pub use crate::cast::{Cast, Character, Lexicon, Pronunciation, Tones};
    pub use crate::cue::{CueDirective, CueForm, Energy, Pace};
    pub use crate::palette::{
        Axes, Collision, FitReport, Palette, Placement, Reach, VoiceRegion, fit, fit_text,
    };
    pub use crate::script::{ChapterScript, Passage, Script, SpeakerId, script};
    pub use crate::source::{Chapter, Location};
}

use crate::source::Location;

/// The kernel error. Every variant that points at the book carries the
/// chapter and line; cast and palette errors carry the offending name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum VoiceError {
    /// A cue comment could not be parsed at all.
    CueParse { loc: Location, reason: String },
    /// A cue used a key the kernel does not know.
    UnknownKey { loc: Location, key: String },
    /// A cue value was not one of the allowed forms.
    BadValue {
        loc: Location,
        key: String,
        value: String,
    },
    /// A cue names a speaker absent from the [`cast::Cast`].
    UnknownSpeaker { loc: Location, speaker: String },
    /// A cue names a tone absent from the cast's tone vocabulary.
    UnknownTone { loc: Location, tone: String },
    /// A one-shot cue has no paragraph after it to apply to.
    CueWithoutText { loc: Location },
    /// A `voice begin` was never closed before the end of the chapter.
    UnclosedSpan { loc: Location },
    /// A `voice begin` opened inside an open span.
    NestedSpan { loc: Location },
    /// A `voice end` appeared with no open span.
    OrphanEnd { loc: Location },
    /// A `voice end` carried keys. An end closes; it does not direct.
    EndWithKeys { loc: Location },
    /// Two cast entries share an id (or an alias collides with an id).
    DuplicateCharacter { id: String },
    /// A cast entry's tones are not in the vocabulary.
    CastUnknownTone { id: String, tone: String },
    /// A palette declares no regions; nothing can be placed.
    PaletteEmpty,
    /// Two palette regions share a name.
    DuplicateRegion { name: String },
}

impl VoiceError {
    /// The chapter/line this error points at, when it has one.
    #[must_use]
    pub fn location(&self) -> Option<&Location> {
        match self {
            Self::CueParse { loc, .. }
            | Self::UnknownKey { loc, .. }
            | Self::BadValue { loc, .. }
            | Self::UnknownSpeaker { loc, .. }
            | Self::UnknownTone { loc, .. }
            | Self::CueWithoutText { loc }
            | Self::UnclosedSpan { loc }
            | Self::NestedSpan { loc }
            | Self::OrphanEnd { loc }
            | Self::EndWithKeys { loc } => Some(loc),
            Self::DuplicateCharacter { .. }
            | Self::CastUnknownTone { .. }
            | Self::PaletteEmpty
            | Self::DuplicateRegion { .. } => None,
        }
    }
}

impl std::fmt::Display for VoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CueParse { loc, reason } => write!(f, "{loc}: cannot parse cue: {reason}"),
            Self::UnknownKey { loc, key } => write!(f, "{loc}: unknown cue key `{key}`"),
            Self::BadValue { loc, key, value } => {
                write!(f, "{loc}: bad value `{value}` for key `{key}`")
            }
            Self::UnknownSpeaker { loc, speaker } => {
                write!(f, "{loc}: speaker `{speaker}` is not in the cast")
            }
            Self::UnknownTone { loc, tone } => {
                write!(f, "{loc}: tone `{tone}` is not in the vocabulary")
            }
            Self::CueWithoutText { loc } => {
                write!(f, "{loc}: cue is not followed by a paragraph")
            }
            Self::UnclosedSpan { loc } => write!(f, "{loc}: `voice begin` never closes"),
            Self::NestedSpan { loc } => {
                write!(f, "{loc}: `voice begin` inside an open span")
            }
            Self::OrphanEnd { loc } => write!(f, "{loc}: `voice end` with no open span"),
            Self::EndWithKeys { loc } => write!(f, "{loc}: `voice end` carries keys"),
            Self::DuplicateCharacter { id } => write!(f, "cast: `{id}` is declared twice"),
            Self::CastUnknownTone { id, tone } => {
                write!(
                    f,
                    "cast: `{id}` default tone `{tone}` is not in the vocabulary"
                )
            }
            Self::PaletteEmpty => write!(f, "palette: no regions declared"),
            Self::DuplicateRegion { name } => {
                write!(f, "palette: region `{name}` is declared twice")
            }
        }
    }
}

impl std::error::Error for VoiceError {}

/// The error collector. One pass reports everything it can.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Errors(pub Vec<VoiceError>);

impl Errors {
    pub fn push(&mut self, e: VoiceError) {
        self.0.push(e);
    }

    pub fn extend(&mut self, other: Errors) {
        self.0.extend(other.0);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn contains(&self, e: &VoiceError) -> bool {
        self.0.contains(e)
    }

    pub fn iter(&self) -> impl Iterator<Item = &VoiceError> {
        self.0.iter()
    }
}
