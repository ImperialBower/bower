//! State coverage over the fixture corpus: the analog of code coverage,
//! taken over the kernel's input space. The report answers "which
//! (op × expect × mechanism) combinations do our fixtures actually put the
//! kernel into?" — so a coverage gap is a listed fact, not a hunch.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use bower_core::block;
use bower_core::prelude::*;

use crate::fixtures::Fixture;

/// The qualitative mechanisms a block can exercise, orthogonal to its op.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Mechanism {
    /// mdBook `#`-hidden lines present.
    HiddenLines,
    /// `// bower:begin/end` region markers or a region op.
    Regions,
    /// `// bower:show` display markers.
    ShowMarkers,
    /// A `show="…"` span filter on the directive.
    ShowFilter,
    /// Content pulled from the block library via `include=`.
    Include,
    /// A step assembled from more than one block.
    MultiBlockStep,
    /// A book feeding more than one repo.
    MultiRepo,
    /// An `after=` ordering bend.
    AfterConstraint,
    /// A `notebook="play"` cell (§ 15).
    PlayCell,
}

impl Mechanism {
    #[must_use]
    pub fn all() -> [Mechanism; 9] {
        [
            Self::HiddenLines,
            Self::Regions,
            Self::ShowMarkers,
            Self::ShowFilter,
            Self::Include,
            Self::MultiBlockStep,
            Self::MultiRepo,
            Self::AfterConstraint,
            Self::PlayCell,
        ]
    }
}

impl std::fmt::Display for Mechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::HiddenLines => "hidden-lines",
            Self::Regions => "regions",
            Self::ShowMarkers => "show-markers",
            Self::ShowFilter => "show-filter",
            Self::Include => "include",
            Self::MultiBlockStep => "multi-block-step",
            Self::MultiRepo => "multi-repo",
            Self::AfterConstraint => "after-constraint",
            Self::PlayCell => "play-cell",
        };
        write!(f, "{s}")
    }
}

/// What a fixture corpus exercises.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CoverageReport {
    pub ops: BTreeSet<String>,
    pub expects: BTreeSet<String>,
    pub mechanisms: BTreeSet<String>,
}

impl CoverageReport {
    /// Measure a corpus. Fixtures that fail to plan still contribute the
    /// blocks that extracted cleanly — broken fixtures are part of the
    /// state space too.
    #[must_use]
    pub fn measure(corpus: &[Fixture]) -> Self {
        let mut report = Self::default();
        for fixture in corpus {
            report.absorb(fixture);
        }
        report
    }

    fn absorb(&mut self, fixture: &Fixture) {
        let (blocks, _errors) = block::extract(&fixture.book, &fixture.catalog);

        let mut repos: BTreeSet<&str> = BTreeSet::new();
        let mut step_ids: Vec<&str> = Vec::new();
        for b in &blocks {
            if b.play {
                self.mechanisms.insert(Mechanism::PlayCell.to_string());
                repos.insert(&b.repo.0);
                continue;
            }
            self.ops.insert(b.op.to_string());
            self.expects
                .insert(b.expect.unwrap_or_default().to_string());
            repos.insert(&b.repo.0);
            if let Some(s) = &b.step {
                step_ids.push(s);
            }
            if b.after.is_some() {
                self.mechanisms
                    .insert(Mechanism::AfterConstraint.to_string());
            }
            if b.show.is_some() {
                self.mechanisms.insert(Mechanism::ShowFilter.to_string());
            }
            if b.region.is_some() {
                self.mechanisms.insert(Mechanism::Regions.to_string());
            }
            for line in &b.content.lines {
                let t = line.trim_start();
                if t == "#" || t.starts_with("# ") {
                    self.mechanisms.insert(Mechanism::HiddenLines.to_string());
                }
                if t.contains("bower:show") || t.contains("bf:show") {
                    self.mechanisms.insert(Mechanism::ShowMarkers.to_string());
                }
                if t.contains("bower:begin") || t.contains("bf:begin") {
                    self.mechanisms.insert(Mechanism::Regions.to_string());
                }
            }
        }
        if repos.len() > 1 {
            self.mechanisms.insert(Mechanism::MultiRepo.to_string());
        }
        for id in &step_ids {
            if step_ids.iter().filter(|s| s == &id).count() > 1 {
                self.mechanisms
                    .insert(Mechanism::MultiBlockStep.to_string());
            }
        }
        if !fixture.book.library.is_empty() {
            self.mechanisms.insert(Mechanism::Include.to_string());
        }
    }

    /// Ops the corpus never exercises.
    #[must_use]
    pub fn missing_ops(&self) -> Vec<String> {
        Op::all()
            .iter()
            .map(ToString::to_string)
            .filter(|o| !self.ops.contains(o))
            .collect()
    }

    /// Expectations the corpus never exercises.
    #[must_use]
    pub fn missing_expects(&self) -> Vec<String> {
        Expect::all()
            .iter()
            .map(ToString::to_string)
            .filter(|e| !self.expects.contains(e))
            .collect()
    }

    /// Mechanisms the corpus never exercises.
    #[must_use]
    pub fn missing_mechanisms(&self) -> Vec<String> {
        Mechanism::all()
            .iter()
            .map(ToString::to_string)
            .filter(|m| !self.mechanisms.contains(m))
            .collect()
    }

    /// Full coverage means every op, expect, and mechanism appears at
    /// least once across the corpus.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.missing_ops().is_empty()
            && self.missing_expects().is_empty()
            && self.missing_mechanisms().is_empty()
    }
}

impl std::fmt::Display for CoverageReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line = |title: &str, hit: &BTreeSet<String>, missing: &[String]| {
            let mut s = String::new();
            let _ = write!(s, "{title}: {} hit", hit.len());
            if missing.is_empty() {
                let _ = write!(s, " (complete)");
            } else {
                let _ = write!(s, " (missing: {})", missing.join(", "));
            }
            s
        };
        writeln!(f, "{}", line("ops", &self.ops, &self.missing_ops()))?;
        writeln!(
            f,
            "{}",
            line("expects", &self.expects, &self.missing_expects())
        )?;
        writeln!(
            f,
            "{}",
            line("mechanisms", &self.mechanisms, &self.missing_mechanisms())
        )
    }
}
