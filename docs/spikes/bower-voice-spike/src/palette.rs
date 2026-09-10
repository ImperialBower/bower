//! The voice tone palette — one narrator's mapped range — and the fit of a
//! cast against it.
//!
//! Prior art maps a voice on two axes (pitch × loudness, the clinical
//! *voice range profile*) or not at all (a casting director's ear). The
//! palette here is four axes a producer can argue about, each `0..=100`, and
//! a **region** is a voice the narrator can reliably produce plus how far
//! they can bend it. The fit is a pure function: nearest region per
//! character, the gap vector when the character is out of reach, and
//! collisions between characters who share a chapter and sit too close to
//! tell apart.
//!
//! The gap vector is the seam to any audio-side processing: "this character
//! needs +20 pitch and +30 age beyond what the narrator has" is exactly the
//! control input a pitch/formant or speech-to-speech stage would take.
//! Nothing in this crate does that processing; it only says how much.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::cast::{Cast, NARRATOR};
use crate::script::Script;
use crate::{Errors, VoiceError};

/// Where a voice sits. Every axis runs `0..=100`; the labels are the ends.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Axes {
    /// 0 = low, 100 = high.
    pub pitch: u8,
    /// 0 = child, 100 = elder. The *impression* of age, not the number.
    pub age: u8,
    /// 0 = light/breathy, 100 = heavy/chesty.
    pub weight: u8,
    /// 0 = still, 100 = driving.
    pub energy: u8,
}

impl Axes {
    #[must_use]
    pub const fn new(pitch: u8, age: u8, weight: u8, energy: u8) -> Self {
        Self {
            pitch,
            age,
            weight,
            energy,
        }
    }

    /// Euclidean distance, rounded down. Integer so it is the same on every
    /// machine.
    #[must_use]
    pub fn distance(self, other: Self) -> u32 {
        let d = |a: u8, b: u8| {
            let x = i32::from(a).abs_diff(i32::from(b));
            x * x
        };
        (d(self.pitch, other.pitch)
            + d(self.age, other.age)
            + d(self.weight, other.weight)
            + d(self.energy, other.energy))
        .isqrt()
    }

    /// `other - self`, per axis: how far and which way to move from here.
    #[must_use]
    pub fn gap_to(self, other: Self) -> Gap {
        let g = |a: u8, b: u8| i16::from(b) - i16::from(a);
        Gap {
            pitch: g(self.pitch, other.pitch),
            age: g(self.age, other.age),
            weight: g(self.weight, other.weight),
            energy: g(self.energy, other.energy),
        }
    }
}

/// A signed per-axis delta.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Gap {
    pub pitch: i16,
    pub age: i16,
    pub weight: i16,
    pub energy: i16,
}

impl std::fmt::Display for Gap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pitch {:+} age {:+} weight {:+} energy {:+}",
            self.pitch, self.age, self.weight, self.energy
        )
    }
}

/// A voice the narrator can do: where it sits and how far it bends.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceRegion {
    pub name: String,
    pub center: Axes,
    /// How far from `center` the narrator stays convincing. Beyond twice
    /// this is out of reach.
    pub radius: u8,
}

impl VoiceRegion {
    #[must_use]
    pub fn new(name: &str, center: Axes, radius: u8) -> Self {
        Self {
            name: name.to_string(),
            center,
            radius,
        }
    }
}

/// One narrator's palette.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Palette {
    pub narrator: String,
    pub regions: Vec<VoiceRegion>,
}

impl Palette {
    #[must_use]
    pub fn new(narrator: &str, regions: Vec<VoiceRegion>) -> Self {
        Self {
            narrator: narrator.to_string(),
            regions,
        }
    }

    /// Every problem with the palette itself.
    #[must_use]
    pub fn validate(&self) -> Errors {
        let mut errors = Errors::default();
        if self.regions.is_empty() {
            errors.push(VoiceError::PaletteEmpty);
        }
        let mut seen = BTreeSet::new();
        for r in &self.regions {
            if !seen.insert(r.name.as_str()) {
                errors.push(VoiceError::DuplicateRegion {
                    name: r.name.clone(),
                });
            }
        }
        errors
    }
}

/// Can the narrator get there?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reach {
    /// Inside the region's radius.
    InRange,
    /// Inside twice the radius: doable, flag it for the producer.
    Stretch { gap: Gap },
    /// Beyond that: recast, redesign the character, or process the audio.
    OutOfReach { gap: Gap },
}

impl Reach {
    /// Every reach, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [&'static str; 3] {
        ["in-range", "stretch", "out-of-reach"]
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::InRange => "in-range",
            Self::Stretch { .. } => "stretch",
            Self::OutOfReach { .. } => "out-of-reach",
        }
    }
}

/// Where one character landed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placement {
    pub character: String,
    pub region: String,
    pub distance: u32,
    pub reach: Reach,
}

/// Two designed characters who share a chapter and sit closer than
/// [`COLLISION_DISTANCE`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collision {
    pub a: String,
    pub b: String,
    pub distance: u32,
    pub chapters: Vec<String>,
}

/// Closer than this, in the same chapter, and a listener cannot tell two
/// voices apart. A starting value for the spike; a real palette study
/// should replace it with a measured one.
pub const COLLISION_DISTANCE: u32 = 15;

/// The fit of a cast against a palette, given where the cast speaks.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FitReport {
    pub narrator: String,
    /// Sorted by character id.
    pub placements: Vec<Placement>,
    /// Sorted by (a, b).
    pub collisions: Vec<Collision>,
    /// Characters with no `target` — counted, not placed.
    pub undesigned: Vec<String>,
}

/// Place every designed character in the palette and find collisions.
///
/// # Errors
///
/// [`VoiceError::PaletteEmpty`] and [`VoiceError::DuplicateRegion`] from the
/// palette; a report is never produced over an invalid palette because
/// "nearest region" would be meaningless.
pub fn fit(palette: &Palette, cast: &Cast, script: &Script) -> Result<FitReport, Errors> {
    let errors = palette.validate();
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut report = FitReport {
        narrator: palette.narrator.clone(),
        ..FitReport::default()
    };

    for (id, c) in &cast.characters {
        let Some(target) = c.target else {
            // The narrator *is* the palette; an undesigned narrator is the
            // normal case, not a gap in the bible.
            if id != NARRATOR {
                report.undesigned.push(id.clone());
            }
            continue;
        };
        // Nearest region; ties broken by name so the answer is stable.
        let mut best: Option<(&VoiceRegion, u32)> = None;
        for r in &palette.regions {
            let d = r.center.distance(target);
            let better = match best {
                None => true,
                Some((br, bd)) => d < bd || (d == bd && r.name < br.name),
            };
            if better {
                best = Some((r, d));
            }
        }
        let Some((region, distance)) = best else {
            continue; // unreachable: validate() rejected an empty palette
        };
        let radius = u32::from(region.radius);
        let reach = if distance <= radius {
            Reach::InRange
        } else if distance <= radius * 2 {
            Reach::Stretch {
                gap: region.center.gap_to(target),
            }
        } else {
            Reach::OutOfReach {
                gap: region.center.gap_to(target),
            }
        };
        report.placements.push(Placement {
            character: id.clone(),
            region: region.name.clone(),
            distance,
            reach,
        });
    }

    // Who shares a chapter with whom.
    let mut chapters_of: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for ch in &script.chapters {
        for p in &ch.passages {
            chapters_of
                .entry(p.speaker.0.as_str())
                .or_default()
                .insert(ch.path.as_str());
        }
    }

    let designed: Vec<(&String, Axes)> = cast
        .characters
        .iter()
        .filter_map(|(id, c)| c.target.map(|t| (id, t)))
        .collect();
    for (i, (a, ta)) in designed.iter().enumerate() {
        for (b, tb) in &designed[i + 1..] {
            let distance = ta.distance(*tb);
            if distance >= COLLISION_DISTANCE {
                continue;
            }
            let (Some(ca), Some(cb)) = (chapters_of.get(a.as_str()), chapters_of.get(b.as_str()))
            else {
                continue;
            };
            let shared: Vec<String> = ca.intersection(cb).map(|s| (*s).to_string()).collect();
            if shared.is_empty() {
                continue;
            }
            report.collisions.push(Collision {
                a: (*a).clone(),
                b: (*b).clone(),
                distance,
                chapters: shared,
            });
        }
    }

    Ok(report)
}

/// The report as stable text — one line per placement, one per collision.
#[must_use]
pub fn fit_text(report: &FitReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# fit: {}", report.narrator);
    for p in &report.placements {
        let _ = write!(
            out,
            "{:<12} -> {:<12} d={:<3} {}",
            p.character,
            p.region,
            p.distance,
            p.reach.label()
        );
        match p.reach {
            Reach::InRange => {}
            Reach::Stretch { gap } | Reach::OutOfReach { gap } => {
                let _ = write!(out, "  ({gap})");
            }
        }
        out.push('\n');
    }
    for c in &report.collisions {
        let _ = writeln!(
            out,
            "COLLISION {} ~ {} d={} in {}",
            c.a,
            c.b,
            c.distance,
            c.chapters.join(", ")
        );
    }
    for u in &report.undesigned {
        let _ = writeln!(out, "undesigned {u}");
    }
    out
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn distance__is_euclidean_and_symmetric() {
        let a = Axes::new(0, 0, 0, 0);
        let b = Axes::new(3, 4, 0, 0);
        assert_eq!(a.distance(b), 5);
        assert_eq!(b.distance(a), 5);
        assert_eq!(a.distance(a), 0);
    }

    #[test]
    fn gap__is_signed_and_directional() {
        let from = Axes::new(50, 50, 50, 50);
        let to = Axes::new(70, 20, 50, 55);
        let g = from.gap_to(to);
        assert_eq!((g.pitch, g.age, g.weight, g.energy), (20, -30, 0, 5));
        assert_eq!(g.to_string(), "pitch +20 age -30 weight +0 energy +5");
    }

    #[test]
    fn validate__empty_and_duplicate_are_named() {
        assert!(
            Palette::new("x", vec![])
                .validate()
                .contains(&VoiceError::PaletteEmpty)
        );
        let p = Palette::new(
            "x",
            vec![
                VoiceRegion::new("own", Axes::default(), 10),
                VoiceRegion::new("own", Axes::default(), 10),
            ],
        );
        assert!(
            p.validate()
                .contains(&VoiceError::DuplicateRegion { name: "own".into() })
        );
    }
}
