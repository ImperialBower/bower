//! The breakdown — the producer's sheet, folded from the script.
//!
//! Who speaks, how much, where each voice first appears, which tones each
//! character is asked for, how much code the book carries, and how long the
//! finished audio will run. Every number is derived from the script, so
//! editing a chapter changes the sheet the same build.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::cast::Cast;
use crate::script::{PassageKind, Script};
use crate::source::Location;

/// Words per finished hour, the audiobook industry's rule of thumb (ACX
/// quotes ~9,300). A book of 93,000 words is a ten-hour book. Studio time
/// runs two to four times finished time; the report says so rather than
/// guessing a multiplier.
pub const WORDS_PER_FINISHED_HOUR: usize = 9_300;

/// One character's share of the book.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CharacterLoad {
    pub id: String,
    pub name: String,
    pub passages: usize,
    pub words: usize,
    /// Chapter paths, in reading order, where this voice speaks.
    pub chapters: Vec<String>,
    /// First line this voice speaks — the narrator's "when do I first need
    /// this voice warmed up".
    pub first_at: Option<Location>,
    /// Tone → passages asked for it.
    pub tones: BTreeMap<String, usize>,
}

/// One chapter's totals.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChapterLoad {
    pub path: String,
    pub stem: String,
    pub prose_words: usize,
    pub heading_words: usize,
    pub code_lines: usize,
    pub passages: usize,
    /// Distinct voices in this chapter, sorted.
    pub speakers: Vec<String>,
}

/// The producer's sheet.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Breakdown {
    pub chapters: Vec<ChapterLoad>,
    /// Sorted by id.
    pub characters: Vec<CharacterLoad>,
    pub total_words: usize,
    pub total_code_lines: usize,
    /// Finished-audio estimate in whole minutes, from [`WORDS_PER_FINISHED_HOUR`].
    pub finished_minutes: usize,
}

/// Fold a script into its breakdown. Pure; never fails — a script is always
/// countable.
#[must_use]
pub fn breakdown(script: &Script, cast: &Cast) -> Breakdown {
    let mut chapters = Vec::new();
    let mut by_id: BTreeMap<String, CharacterLoad> = BTreeMap::new();
    let mut total_words = 0;
    let mut total_code_lines = 0;

    for ch in &script.chapters {
        let mut load = ChapterLoad {
            path: ch.path.clone(),
            stem: ch.stem.clone(),
            ..ChapterLoad::default()
        };
        let mut speakers = BTreeSet::new();
        for p in &ch.passages {
            load.passages += 1;
            match p.kind {
                PassageKind::Code => {
                    load.code_lines += p.text.len();
                    continue; // code has no speaker until somebody decides
                }
                PassageKind::Heading => load.heading_words += p.words,
                PassageKind::Prose => load.prose_words += p.words,
            }
            speakers.insert(p.speaker.0.clone());

            let entry = by_id.entry(p.speaker.0.clone()).or_insert_with(|| {
                let name = cast
                    .get(&p.speaker.0)
                    .map_or_else(|| p.speaker.0.clone(), |c| c.name.clone());
                CharacterLoad {
                    id: p.speaker.0.clone(),
                    name,
                    ..CharacterLoad::default()
                }
            });
            entry.passages += 1;
            entry.words += p.words;
            if entry.chapters.last() != Some(&ch.path) {
                entry.chapters.push(ch.path.clone());
            }
            if entry.first_at.is_none() {
                entry.first_at = Some(Location::new(&ch.path, p.lines_at.start));
            }
            for t in &p.tones {
                *entry.tones.entry(t.clone()).or_insert(0) += 1;
            }
        }
        load.speakers = speakers.into_iter().collect();
        total_words += load.prose_words + load.heading_words;
        total_code_lines += load.code_lines;
        chapters.push(load);
    }

    // Cast members who never speak still appear, at zero — a producer wants
    // to know about the character the author designed and never used.
    for (id, c) in &cast.characters {
        by_id.entry(id.clone()).or_insert_with(|| CharacterLoad {
            id: id.clone(),
            name: c.name.clone(),
            ..CharacterLoad::default()
        });
    }

    Breakdown {
        chapters,
        characters: by_id.into_values().collect(),
        total_words,
        total_code_lines,
        finished_minutes: total_words * 60 / WORDS_PER_FINISHED_HOUR,
    }
}

/// The sheet as stable text. Same input, same bytes — so it can sit in a
/// lock-style file and `status` can diff it.
#[must_use]
pub fn breakdown_text(b: &Breakdown) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# breakdown: {} words, {} code lines, ~{}h{:02}m finished",
        b.total_words,
        b.total_code_lines,
        b.finished_minutes / 60,
        b.finished_minutes % 60
    );
    out.push_str("\n[chapters]\n");
    for c in &b.chapters {
        let _ = writeln!(
            out,
            "{:<28} words={:<6} code={:<4} voices={}",
            c.stem,
            c.prose_words + c.heading_words,
            c.code_lines,
            c.speakers.join(",")
        );
    }
    out.push_str("\n[cast]\n");
    for c in &b.characters {
        let first = c
            .first_at
            .as_ref()
            .map_or_else(|| "never".to_string(), ToString::to_string);
        let tones: Vec<String> = c.tones.iter().map(|(t, n)| format!("{t}:{n}")).collect();
        let _ = writeln!(
            out,
            "{:<12} passages={:<4} words={:<6} first={:<22} tones={}",
            c.id,
            c.passages,
            c.words,
            first,
            tones.join(",")
        );
    }
    out
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::cast::Character;
    use crate::script::script;
    use crate::source::Chapter;

    fn book() -> (Script, Cast) {
        let cast = Cast::new(vec![
            Character::new("rosa", "Rosa").with_tones(&["dry"]),
            Character::new("unused", "Never Speaks"),
        ]);
        let ch1 = Chapter::new(
            "src/ch01.md",
            "# One\n\nNarration here.\n\n<!-- voice speaker=\"rosa\" tone=\"wry\" -->\nRosa says four words.\n\n```rust\nfn a() {}\nfn b() {}\n```\n",
        );
        let ch2 = Chapter::new(
            "src/ch02.md",
            "<!-- voice speaker=\"rosa\" -->\nRosa again.\n\nThe end.\n",
        );
        let (s, e) = script(&[ch1, ch2], &cast);
        assert!(e.is_empty(), "{e:?}");
        (s, cast)
    }

    #[test]
    fn totals__count_prose_and_headings_but_not_code() {
        let (s, cast) = book();
        let b = breakdown(&s, &cast);
        // "One"(1) + "Narration here."(2) + "Rosa says four words."(4) + "Rosa again."(2) + "The end."(2)
        assert_eq!(b.total_words, 11);
        assert_eq!(b.total_code_lines, 2);
        assert_eq!(b.chapters[0].code_lines, 2);
        assert_eq!(b.chapters[0].speakers, vec!["narrator", "rosa"]);
    }

    #[test]
    fn character__load_tracks_first_appearance_chapters_and_tones() {
        let (s, cast) = book();
        let b = breakdown(&s, &cast);
        let rosa = b.characters.iter().find(|c| c.id == "rosa").unwrap();
        assert_eq!(rosa.passages, 2);
        assert_eq!(rosa.words, 6);
        assert_eq!(rosa.first_at, Some(Location::new("src/ch01.md", 6)));
        assert_eq!(rosa.chapters, vec!["src/ch01.md", "src/ch02.md"]);
        assert_eq!(rosa.tones.get("wry"), Some(&1));
        assert_eq!(rosa.tones.get("dry"), Some(&1), "default tone counts too");
    }

    #[test]
    fn character__declared_but_silent_appears_at_zero() {
        let (s, cast) = book();
        let b = breakdown(&s, &cast);
        let u = b.characters.iter().find(|c| c.id == "unused").unwrap();
        assert_eq!(u.passages, 0);
        assert_eq!(u.first_at, None);
    }

    #[test]
    fn runtime__uses_the_industry_rule_of_thumb() {
        let mut b = Breakdown::default();
        b.total_words = WORDS_PER_FINISHED_HOUR * 10;
        b.finished_minutes = b.total_words * 60 / WORDS_PER_FINISHED_HOUR;
        assert_eq!(b.finished_minutes, 600);
    }

    #[test]
    fn text__is_stable() {
        let (s, cast) = book();
        let a = breakdown_text(&breakdown(&s, &cast));
        let b = breakdown_text(&breakdown(&s, &cast));
        assert_eq!(a, b);
        assert!(a.starts_with("# breakdown: 11 words, 2 code lines, ~0h00m finished\n"));
        assert!(a.contains("unused       passages=0    words=0      first=never"));
    }
}
