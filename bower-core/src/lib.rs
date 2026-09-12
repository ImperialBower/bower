//! # bower-core — the Bower domain kernel
//!
//! Bower turns an annotated book source into deterministic, replayable git
//! repositories. This crate is the pure heart of that system: it parses
//! directives out of chapter markdown, groups them into steps, orders the
//! steps into a [`prelude::BookPlan`], and folds every step into a complete
//! [`prelude::TreeState`] — the file tree a replay layer will commit.
//!
//! The kernel's contract, per the Bower design spec:
//!
//! * **No I/O.** Input is text handed in by the caller ([`prelude::BookSource`]);
//!   output is values. No filesystem, no git, no network.
//! * **No serialization in the public API.** Lock-file *text* is produced as
//!   a `String`; no format crate appears in any signature.
//! * **Deterministic.** Given the same [`prelude::BookSource`] and [`prelude::RepoCatalog`],
//!   [`prelude::plan`] returns an identical [`prelude::BookPlan`], down to every tree byte.
//! * **Exhaustive, located errors.** Every failure is a [`BowerError`]
//!   carrying the chapter and line that caused it. Errors are collected,
//!   not short-circuited — one pass reports everything it can.
//!
//! The replay layer (`bower` CLI), the mdBook preprocessor, and the
//! publishing pipeline are all consumers of this crate, never the other way
//! around.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]
#![allow(
    clippy::module_name_repetitions, // RepoPlan, RepoName et al. read better qualified
    clippy::missing_panics_doc       // library code has no panic paths; tests unwrap freely
)]

pub mod block;
pub mod capture;
pub mod directive;
pub mod display;
pub mod plan;
pub mod source;
pub mod step;
pub mod tree;

pub mod prelude {
    //! The curated public front door: `use bower_core::prelude::*;` is the
    //! one import a consumer (or a doc test) needs.

    pub use crate::BowerError;
    pub use crate::block::{Block, BlockContent};
    pub use crate::capture::{Scrub, normalize, tidy};
    pub use crate::directive::{Capture, Directive, Expect, Op};
    pub use crate::display::{BlockDisplay, DisplaySpan, LineRange};
    pub use crate::plan::{
        BookPlan, CapturedOutput, Exercise, ExerciseForm, PlannedStep, PlayCell, RepoPlan,
        lock_text, plan,
    };
    pub use crate::source::{BookSource, Chapter, Location, RepoCatalog, RepoName, RepoSpec};
    pub use crate::step::StepId;
    pub use crate::tree::{FileBody, ShowMark, TreeState, show_marker};
}

use crate::directive::{Capture, Expect};
use crate::source::Location;

/// The central kernel error. Every variant carries enough context to point
/// an author at the exact chapter, line, and step that needs fixing —
/// plan-time errors are first-class citizens here, in keeping with the
/// books this tool exists to build.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BowerError {
    /// A directive comment could not be parsed at all.
    DirectiveParse { loc: Location, reason: String },
    /// A directive used a key the kernel does not know.
    UnknownKey { loc: Location, key: String },
    /// A directive value was not one of the allowed forms.
    BadValue {
        loc: Location,
        key: String,
        value: String,
    },
    /// A directive is missing a key its `op` requires.
    MissingKey { loc: Location, key: String },
    /// A directive names a repo absent from the [`source::RepoCatalog`].
    UnknownRepo { loc: Location, repo: String },
    /// A directive that needs a fenced code block has none following it.
    DirectiveWithoutBlock { loc: Location },
    /// A fenced code block opened but never closed before end of chapter.
    UnclosedFence { loc: Location },
    /// An `include` key referenced a path absent from the block library.
    IncludeMissing { loc: Location, path: String },
    /// An included library entry did not contain exactly one block.
    IncludeMalformed {
        loc: Location,
        path: String,
        reason: String,
    },
    /// An `op="copy"` referenced an asset absent from the book source.
    AssetMissing { loc: Location, src: String },
    /// Two blocks in the same step write the same file in ways that don't
    /// compose (a whole-file write shares the file, or two region ops hit
    /// the same region). Distinct regions and appends may share a file.
    DuplicateFileInStep {
        loc: Location,
        step: String,
        file: String,
    },
    /// Blocks sharing a step id declared different repos.
    ConflictingRepoInStep { loc: Location, step: String },
    /// Blocks sharing a step id declared conflicting `expect` values.
    ConflictingExpectInStep { loc: Location, step: String },
    /// `op="create"` (or `copy`) hit a file that already exists.
    FileAlreadyExists {
        loc: Location,
        step: String,
        file: String,
    },
    /// `replace`/`region`/`append`/`delete` hit a file never created.
    FileNotCreated {
        loc: Location,
        step: String,
        file: String,
    },
    /// `op="region"` named a region whose markers are not in the file.
    RegionMissing {
        loc: Location,
        step: String,
        file: String,
        region: String,
    },
    /// A region's `begin` marker has no matching `end` (or vice versa).
    RegionUnbalanced {
        loc: Location,
        step: String,
        file: String,
        region: String,
    },
    /// An `after` key referenced a step id that does not exist in the repo.
    OrphanAfter {
        loc: Location,
        step: String,
        after: String,
    },
    /// `after` constraints formed a cycle; the plan has no valid order.
    OrderingCycle { repo: String, steps: Vec<String> },
    /// A `// bower:show` span was opened but never closed in the block.
    UnclosedShowSpan { loc: Location },
    /// A `// bower:show` span was opened inside another open span.
    NestedShowSpan { loc: Location },
    /// A `// bower:show end` appeared with no open span.
    OrphanShowEnd { loc: Location },
    /// A `show="…"` key named a span the block does not define.
    UnknownShowSpan { loc: Location, name: String },
    /// A displayed span could not be located in the materialized tree.
    /// This is a kernel invariant violation surfaced as an error rather
    /// than a panic — if you see it, the line-map property test has a gap.
    SpanNotInTree {
        loc: Location,
        step: String,
        file: String,
    },
    /// A `notebook="play"` cell has no preceding step of its repo to
    /// attach to, and names none explicitly.
    PlayCellUnbound { loc: Location },
    /// A `notebook="play"` cell names a step that does not exist.
    PlayCellUnknownStep { loc: Location, step: String },
    /// A `notebook="play"` cell carries tree-affecting keys (`file`, `op`,
    /// `region`, `src`, `paths`) — a category confusion the kernel refuses:
    /// play cells never touch a repo tree.
    PlayCellConflictingKeys { loc: Location },
    /// A block-form exercise has no preceding step of its repo to attach
    /// to, and names none explicitly.
    ExerciseUnbound { loc: Location },
    /// A block-form exercise names a step that does not exist.
    ExerciseUnknownStep { loc: Location, step: String },
    /// A step already carries an exercise; reported at the second one.
    ExerciseDuplicate { loc: Location, step: String },
    /// The exercise's step is the last of its repo: nothing follows it to
    /// be the answer.
    ExerciseWithoutAnswer { loc: Location, step: String },
    /// A play cell also declares `exercise=`. A cell is one thing or the
    /// other.
    ExerciseConflictingKeys { loc: Location },
    /// An `output` block has no preceding step of its repo to attach to, and
    /// names none explicitly.
    OutputUnbound { loc: Location },
    /// An `output` block names a step that does not exist.
    OutputUnknownStep { loc: Location, step: String },
    /// An `output` block carries a key that belongs to another kind of block:
    /// a tree key, `notebook`, `exercise`, `expect`, or `include`.
    OutputConflictingKeys { loc: Location },
    /// A step already has an output block for this capture; reported at the
    /// second one.
    OutputDuplicate {
        loc: Location,
        step: String,
        capture: Capture,
    },
    /// The bound step never runs this capture's command: `verify` on a
    /// `compile_fail` step, or anything on a `none` step.
    OutputNeverRuns {
        loc: Location,
        step: String,
        capture: Capture,
        expect: Expect,
    },
}

impl BowerError {
    /// The chapter/line this error points at, when it has one.
    #[must_use]
    pub fn location(&self) -> Option<&Location> {
        match self {
            Self::DirectiveParse { loc, .. }
            | Self::UnknownKey { loc, .. }
            | Self::BadValue { loc, .. }
            | Self::MissingKey { loc, .. }
            | Self::UnknownRepo { loc, .. }
            | Self::DirectiveWithoutBlock { loc }
            | Self::UnclosedFence { loc }
            | Self::IncludeMissing { loc, .. }
            | Self::IncludeMalformed { loc, .. }
            | Self::AssetMissing { loc, .. }
            | Self::DuplicateFileInStep { loc, .. }
            | Self::ConflictingRepoInStep { loc, .. }
            | Self::ConflictingExpectInStep { loc, .. }
            | Self::FileAlreadyExists { loc, .. }
            | Self::FileNotCreated { loc, .. }
            | Self::RegionMissing { loc, .. }
            | Self::RegionUnbalanced { loc, .. }
            | Self::OrphanAfter { loc, .. }
            | Self::UnclosedShowSpan { loc }
            | Self::NestedShowSpan { loc }
            | Self::OrphanShowEnd { loc }
            | Self::UnknownShowSpan { loc, .. }
            | Self::SpanNotInTree { loc, .. }
            | Self::PlayCellUnbound { loc }
            | Self::PlayCellUnknownStep { loc, .. }
            | Self::PlayCellConflictingKeys { loc }
            | Self::ExerciseUnbound { loc }
            | Self::ExerciseUnknownStep { loc, .. }
            | Self::ExerciseDuplicate { loc, .. }
            | Self::ExerciseWithoutAnswer { loc, .. }
            | Self::ExerciseConflictingKeys { loc }
            | Self::OutputUnbound { loc }
            | Self::OutputUnknownStep { loc, .. }
            | Self::OutputConflictingKeys { loc }
            | Self::OutputDuplicate { loc, .. }
            | Self::OutputNeverRuns { loc, .. } => Some(loc),
            Self::OrderingCycle { .. } => None,
        }
    }
}

impl std::fmt::Display for BowerError {
    // One arm per variant; splitting it would need a `_` arm and lose the
    // exhaustiveness check that forces every new error to carry a message.
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DirectiveParse { loc, reason } => {
                write!(f, "{loc}: cannot parse directive: {reason}")
            }
            Self::UnknownKey { loc, key } => write!(f, "{loc}: unknown directive key `{key}`"),
            Self::BadValue { loc, key, value } => {
                write!(f, "{loc}: bad value `{value}` for key `{key}`")
            }
            Self::MissingKey { loc, key } => write!(f, "{loc}: directive missing key `{key}`"),
            Self::UnknownRepo { loc, repo } => {
                write!(f, "{loc}: repo `{repo}` is not declared in the catalog")
            }
            Self::DirectiveWithoutBlock { loc } => {
                write!(f, "{loc}: directive is not followed by a fenced code block")
            }
            Self::UnclosedFence { loc } => write!(f, "{loc}: fenced code block never closes"),
            Self::IncludeMissing { loc, path } => {
                write!(f, "{loc}: include `{path}` is not in the block library")
            }
            Self::IncludeMalformed { loc, path, reason } => {
                write!(f, "{loc}: include `{path}` is malformed: {reason}")
            }
            Self::AssetMissing { loc, src } => {
                write!(f, "{loc}: asset `{src}` is not in the book source")
            }
            Self::DuplicateFileInStep { loc, step, file } => {
                write!(f, "{loc}: step `{step}` has conflicting writes to `{file}`")
            }
            Self::ConflictingRepoInStep { loc, step } => {
                write!(f, "{loc}: blocks in step `{step}` disagree about the repo")
            }
            Self::ConflictingExpectInStep { loc, step } => {
                write!(
                    f,
                    "{loc}: blocks in step `{step}` declare conflicting `expect` values"
                )
            }
            Self::FileAlreadyExists { loc, step, file } => {
                write!(
                    f,
                    "{loc}: step `{step}` creates `{file}`, which already exists"
                )
            }
            Self::FileNotCreated { loc, step, file } => {
                write!(
                    f,
                    "{loc}: step `{step}` modifies `{file}`, which was never created"
                )
            }
            Self::RegionMissing {
                loc,
                step,
                file,
                region,
            } => write!(
                f,
                "{loc}: step `{step}` targets region `{region}` in `{file}`, but its markers are not there"
            ),
            Self::RegionUnbalanced {
                loc,
                step,
                file,
                region,
            } => write!(
                f,
                "{loc}: step `{step}`: region `{region}` in `{file}` has unbalanced begin/end markers"
            ),
            Self::OrphanAfter { loc, step, after } => write!(
                f,
                "{loc}: step `{step}` says after=\"{after}\", but no such step exists"
            ),
            Self::OrderingCycle { repo, steps } => write!(
                f,
                "repo `{repo}`: after-constraints form a cycle through steps [{}]",
                steps.join(", ")
            ),
            Self::UnclosedShowSpan { loc } => {
                write!(f, "{loc}: `bower:show` span opened but never closed")
            }
            Self::NestedShowSpan { loc } => {
                write!(f, "{loc}: `bower:show` span opened inside an open span")
            }
            Self::OrphanShowEnd { loc } => {
                write!(f, "{loc}: `bower:show end` with no open span")
            }
            Self::UnknownShowSpan { loc, name } => {
                write!(
                    f,
                    "{loc}: show=\"{name}\" names a span this block does not define"
                )
            }
            Self::SpanNotInTree { loc, step, file } => write!(
                f,
                "{loc}: internal: displayed span from step `{step}` not found in materialized `{file}`"
            ),
            Self::PlayCellUnbound { loc } => {
                write!(f, "{loc}: play cell has no preceding step to attach to")
            }
            Self::PlayCellUnknownStep { loc, step } => {
                write!(
                    f,
                    "{loc}: play cell names step `{step}`, which does not exist"
                )
            }
            Self::PlayCellConflictingKeys { loc } => write!(
                f,
                "{loc}: play cell carries tree-affecting keys; a notebook cell never touches the repo"
            ),
            Self::ExerciseUnbound { loc } => {
                write!(f, "{loc}: exercise has no preceding step to attach to")
            }
            Self::ExerciseUnknownStep { loc, step } => {
                write!(
                    f,
                    "{loc}: exercise names step `{step}`, which does not exist"
                )
            }
            Self::ExerciseDuplicate { loc, step } => {
                write!(f, "{loc}: step `{step}` already carries an exercise")
            }
            Self::ExerciseWithoutAnswer { loc, step } => write!(
                f,
                "{loc}: step `{step}` is the last of its repo; no next step can be the answer"
            ),
            Self::ExerciseConflictingKeys { loc } => {
                write!(f, "{loc}: a play cell cannot also be an exercise")
            }
            Self::OutputUnbound { loc } => {
                write!(f, "{loc}: output block has no preceding step to attach to")
            }
            Self::OutputUnknownStep { loc, step } => write!(
                f,
                "{loc}: output block names step `{step}`, which does not exist"
            ),
            Self::OutputConflictingKeys { loc } => write!(
                f,
                "{loc}: output block carries keys of another kind of block; it names only its repo, step, and capture"
            ),
            Self::OutputDuplicate { loc, step, capture } => write!(
                f,
                "{loc}: step `{step}` already has an output=\"{capture}\" block"
            ),
            Self::OutputNeverRuns {
                loc,
                step,
                capture,
                expect,
            } => write!(
                f,
                "{loc}: step `{step}` declares expect=\"{expect}\", so its `{capture}` command never runs"
            ),
        }
    }
}

impl std::error::Error for BowerError {}

/// The collected, ordered errors from a planning pass. Bower reports
/// everything it can find in one run rather than stopping at the first
/// problem — an author fixing a chapter should not need ten rebuilds to
/// see ten mistakes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Errors(pub Vec<BowerError>);

impl Errors {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, e: BowerError) {
        self.0.push(e);
    }

    pub fn extend(&mut self, other: Errors) {
        self.0.extend(other.0);
    }
}

impl std::fmt::Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for e in &self.0 {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Errors {}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod lib_tests {
    use super::*;
    use crate::source::Location;

    fn loc() -> Location {
        Location {
            chapter: "ch01.md".to_string(),
            line: 7,
        }
    }

    #[test]
    fn display__carries_location() {
        let e = BowerError::UnknownRepo {
            loc: loc(),
            repo: "nope".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "ch01.md:7: repo `nope` is not declared in the catalog"
        );
    }

    #[test]
    fn location__cycle_has_none() {
        let e = BowerError::OrderingCycle {
            repo: "failers".to_string(),
            steps: vec![],
        };
        assert!(e.location().is_none());
    }
}
