//! # bower-testkit — the controllability half of the Bower kernel
//!
//! Per the kernel-testkit pattern: a kernel is not done until putting it
//! into any state a test needs is cheap. This crate ships that capability
//! for `bower-core` as three things:
//!
//! * **Fixtures** — named, hand-written book sources covering every op,
//!   every mechanism, and every error the kernel can report. These are the
//!   *data textures* of book sources: the qualitative shapes an annotated
//!   chapter can take.
//! * **Generators** — proptest strategies producing arbitrary *valid*
//!   books, for the properties that must hold everywhere (determinism,
//!   line-map exactness, fold idempotence).
//! * **The coverage report** — *state coverage* over the fixture corpus:
//!   which (op × expect × mechanism) combinations the corpus actually
//!   exercises, so a gap is a number, not a feeling.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]
#![allow(clippy::module_name_repetitions)]

pub mod coverage;
pub mod fixtures;
pub mod generators;
pub mod textures;

pub mod prelude {
    //! One import for a test file.
    pub use crate::coverage::{CoverageReport, Mechanism};
    pub use crate::fixtures::{self, Fixture};
    pub use crate::generators::{arb_book, arb_plain_lines, arb_raw_output, arb_words};
    pub use bower_core::prelude::*;
}
