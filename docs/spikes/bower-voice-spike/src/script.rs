//! The fold: chapters + cast → a script.
//!
//! Every paragraph of every chapter becomes a [`Passage`] with a speaker and
//! a delivery, whether or not the author cued it — an unmarked paragraph is
//! the narrator, level, measured, in the character's default tones. That
//! completeness is what makes the [`crate::breakdown`] honest: word counts
//! cover the whole book, not the annotated parts.
//!
//! Scoping rules, in one place:
//!
//! * A **one-shot** cue applies to the next paragraph and nothing else. Two
//!   cues with no paragraph between them: the first is `CueWithoutText`.
//! * A **span** (`voice begin` … `voice end`) sets an ambient direction for
//!   every paragraph inside it; a one-shot inside a span overlays it.
//! * **Headings** are always the narrator and never inside a span's reach.
//! * **Fenced code** is a passage of kind `Code`, always the narrator, never
//!   scanned for cues. A programming book read aloud has to decide what to
//!   do with its code; the breakdown counts it so somebody can.
//! * `<!-- bower … -->` directives and every other HTML comment are ignored.

use crate::cast::{Cast, NARRATOR};
use crate::cue::{CueDirective, CueForm, Energy, Pace, Pause};
use crate::source::{Chapter, LineRange, Location};
use crate::{Errors, VoiceError};

/// A canonical character id from the cast.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SpeakerId(pub String);

/// What kind of text a passage is.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PassageKind {
    Prose,
    Heading,
    Code,
}

impl PassageKind {
    /// Every kind, for coverage reporting in the testkit.
    #[must_use]
    pub fn all() -> [PassageKind; 3] {
        [Self::Prose, Self::Heading, Self::Code]
    }
}

/// One unit of reading.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Passage {
    pub kind: PassageKind,
    pub lines_at: LineRange,
    pub speaker: SpeakerId,
    pub tones: Vec<String>,
    pub pace: Pace,
    pub energy: Energy,
    pub pause: Option<Pause>,
    pub note: Option<String>,
    /// The cue that directed this passage, if any — where a narrator's
    /// question about it should point.
    pub cue_at: Option<Location>,
    pub text: Vec<String>,
    pub words: usize,
}

/// One chapter's passages in reading order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChapterScript {
    pub path: String,
    pub stem: String,
    pub passages: Vec<Passage>,
}

/// The whole book, ready to read.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Script {
    pub chapters: Vec<ChapterScript>,
}

/// The ambient direction inside a span, or the defaults outside one.
#[derive(Clone, Debug, Default)]
struct Ambient {
    speaker: Option<String>,
    tones: Vec<String>,
    pace: Option<Pace>,
    energy: Option<Energy>,
    note: Option<String>,
    opened_at: Option<Location>,
}

/// Fold chapters into a script. Errors are collected; the script is always
/// complete — a bad cue falls back to the narrator rather than dropping the
/// paragraph.
#[must_use]
pub fn script(chapters: &[Chapter], cast: &Cast) -> (Script, Errors) {
    let mut errors = Errors::default();
    let mut out = Script::default();
    for ch in chapters {
        out.chapters.push(scan_chapter(ch, cast, &mut errors));
    }
    (out, errors)
}

// One scan, one state machine. Splitting it would spread the scoping rules
// this module's docs list in one place across several functions.
#[allow(clippy::too_many_lines)]
fn scan_chapter(chapter: &Chapter, cast: &Cast, errors: &mut Errors) -> ChapterScript {
    let lines: Vec<&str> = chapter.text.lines().collect();
    let mut passages = Vec::new();
    let mut ambient = Ambient::default();
    let mut pending: Option<(CueDirective, Location)> = None;
    let mut i = 0_usize;

    while i < lines.len() {
        let line = lines[i];
        let loc = Location::new(&chapter.path, i + 1);

        if let Some(width) = fence_width(line) {
            let (end, closed) = skip_fence(&lines, i, width);
            // Code is never a cue's target; a cue left waiting is an error.
            take_pending_as_error(&mut pending, errors);
            let body_end = if closed { end - 1 } else { end };
            let body: Vec<String> = lines[i + 1..body_end]
                .iter()
                .map(|s| (*s).to_string())
                .collect();
            passages.push(plain(
                PassageKind::Code,
                LineRange { start: i + 1, end },
                body,
            ));
            i = end;
            continue;
        }

        if let Some(h) = heading_text(line) {
            take_pending_as_error(&mut pending, errors);
            passages.push(plain(
                PassageKind::Heading,
                LineRange {
                    start: i + 1,
                    end: i + 1,
                },
                vec![h],
            ));
            i += 1;
            continue;
        }

        if CueDirective::is_cue_line(line) {
            match CueDirective::parse(line, &loc) {
                Err(errs) => errors.extend(errs),
                Ok(cue) => match cue.form() {
                    CueForm::OneShot => {
                        take_pending_as_error(&mut pending, errors);
                        pending = Some((cue, loc));
                    }
                    CueForm::Begin => {
                        take_pending_as_error(&mut pending, errors);
                        if ambient.opened_at.is_some() {
                            errors.push(VoiceError::NestedSpan { loc: loc.clone() });
                        } else {
                            ambient = Ambient {
                                speaker: cue.speaker,
                                tones: cue.tones,
                                pace: cue.pace,
                                energy: cue.energy,
                                note: cue.note,
                                opened_at: Some(loc),
                            };
                        }
                    }
                    CueForm::End => {
                        take_pending_as_error(&mut pending, errors);
                        if ambient.opened_at.is_none() {
                            errors.push(VoiceError::OrphanEnd { loc });
                        }
                        ambient = Ambient::default();
                    }
                },
            }
            i += 1;
            continue;
        }

        if line.trim().is_empty() || is_other_comment(line) {
            i += 1;
            continue;
        }

        // A paragraph: run to the next blank, cue, heading, or fence.
        let start = i;
        let mut text = Vec::new();
        while i < lines.len() {
            let l = lines[i];
            if l.trim().is_empty()
                || CueDirective::is_cue_line(l)
                || heading_text(l).is_some()
                || fence_width(l).is_some()
                || is_other_comment(l)
            {
                break;
            }
            text.push(l.to_string());
            i += 1;
        }
        let range = LineRange {
            start: start + 1,
            end: i,
        };
        let cue = pending.take();
        passages.push(direct(range, text, cue, &ambient, cast, errors));
    }

    take_pending_as_error(&mut pending, errors);
    if let Some(loc) = ambient.opened_at {
        errors.push(VoiceError::UnclosedSpan { loc });
    }

    ChapterScript {
        path: chapter.path.clone(),
        stem: chapter.stem().to_string(),
        passages,
    }
}

fn take_pending_as_error(pending: &mut Option<(CueDirective, Location)>, errors: &mut Errors) {
    if let Some((_, loc)) = pending.take() {
        errors.push(VoiceError::CueWithoutText { loc });
    }
}

/// A passage nobody directed: the narrator, plain.
fn plain(kind: PassageKind, lines_at: LineRange, text: Vec<String>) -> Passage {
    let words = count_words(&text);
    Passage {
        kind,
        lines_at,
        speaker: SpeakerId(NARRATOR.to_string()),
        tones: Vec::new(),
        pace: Pace::default(),
        energy: Energy::default(),
        pause: None,
        note: None,
        cue_at: None,
        text,
        words,
    }
}

/// A prose passage under a cue and/or an ambient span.
fn direct(
    lines_at: LineRange,
    text: Vec<String>,
    cue: Option<(CueDirective, Location)>,
    ambient: &Ambient,
    cast: &Cast,
    errors: &mut Errors,
) -> Passage {
    let (cue, cue_at) = match cue {
        Some((c, l)) => (c, Some(l)),
        None => (CueDirective::default(), None),
    };
    let err_at = cue_at
        .clone()
        .or_else(|| ambient.opened_at.clone())
        .unwrap_or_else(|| Location::new("", lines_at.start));

    let named = cue.speaker.as_deref().or(ambient.speaker.as_deref());
    let speaker = match named {
        None => NARRATOR.to_string(),
        Some(n) => cast.resolve(n).map_or_else(
            || {
                errors.push(VoiceError::UnknownSpeaker {
                    loc: err_at.clone(),
                    speaker: n.to_string(),
                });
                NARRATOR.to_string()
            },
            str::to_string,
        ),
    };

    let mut tones: Vec<String> = if cue.tones.is_empty() {
        ambient.tones.clone()
    } else {
        cue.tones.clone()
    };
    tones.retain(|t| {
        let ok = cast.tones.contains(t);
        if !ok {
            errors.push(VoiceError::UnknownTone {
                loc: err_at.clone(),
                tone: t.clone(),
            });
        }
        ok
    });
    if tones.is_empty() {
        tones = cast
            .get(&speaker)
            .map(|c| c.default_tones.clone())
            .unwrap_or_default();
    }

    let words = count_words(&text);
    Passage {
        kind: PassageKind::Prose,
        lines_at,
        speaker: SpeakerId(speaker),
        tones,
        pace: cue.pace.or(ambient.pace).unwrap_or_default(),
        energy: cue.energy.or(ambient.energy).unwrap_or_default(),
        pause: cue.pause,
        note: cue.note.or_else(|| ambient.note.clone()),
        cue_at,
        text,
        words,
    }
}

fn count_words(text: &[String]) -> usize {
    text.iter().map(|l| l.split_whitespace().count()).sum()
}

fn is_other_comment(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("<!--") && t.ends_with("-->")
}

/// If the line opens a fence, return its backtick count.
fn fence_width(line: &str) -> Option<usize> {
    let t = line.trim_start();
    let count = t.chars().take_while(|&c| c == '`').count();
    (count >= 3).then_some(count)
}

/// Given the index of an opening fence, return the index just past its
/// closing fence (`CommonMark`: closer must be at least as wide) and whether
/// a closer was found. An unclosed fence runs to the end of the chapter.
fn skip_fence(lines: &[&str], open: usize, width: usize) -> (usize, bool) {
    let mut j = open + 1;
    while j < lines.len() {
        if fence_width(lines[j]).is_some_and(|w| w >= width) {
            return (j + 1, true);
        }
        j += 1;
    }
    (lines.len(), false)
}

fn heading_text(line: &str) -> Option<String> {
    let t = line.trim_start();
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &t[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some(rest.trim().to_string())
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::cast::Character;

    fn cast() -> Cast {
        Cast::new(vec![
            Character::new("rosa", "Rosa").with_tones(&["dry"]),
            Character::new("tom", "Tom").with_aliases(&["the-kid"]),
        ])
    }

    fn one(text: &str) -> (Script, Errors) {
        script(&[Chapter::new("src/ch01.md", text)], &cast())
    }

    #[test]
    fn unmarked__paragraph_is_the_narrator_by_default() {
        let (s, e) = one("Just prose.\n");
        assert!(e.is_empty());
        let p = &s.chapters[0].passages[0];
        assert_eq!(p.speaker.0, NARRATOR);
        assert_eq!(p.tones, vec!["neutral"]);
        assert_eq!(p.words, 2);
    }

    #[test]
    fn one_shot__directs_exactly_one_paragraph() {
        let (s, e) = one("<!-- voice speaker=\"rosa\" tone=\"wry\" -->\nLine one.\n\nLine two.\n");
        assert!(e.is_empty(), "{e:?}");
        let ps = &s.chapters[0].passages;
        assert_eq!(ps[0].speaker.0, "rosa");
        assert_eq!(ps[0].tones, vec!["wry"]);
        assert_eq!(ps[0].cue_at, Some(Location::new("src/ch01.md", 1)));
        assert_eq!(ps[1].speaker.0, NARRATOR);
        assert_eq!(ps[1].cue_at, None);
    }

    #[test]
    fn cue_without_tone__uses_the_character_default() {
        let (s, _) = one("<!-- voice speaker=\"rosa\" -->\nHm.\n");
        assert_eq!(s.chapters[0].passages[0].tones, vec!["dry"]);
    }

    #[test]
    fn span__covers_every_paragraph_until_end_and_a_one_shot_overlays_it() {
        let text = "<!-- voice begin speaker=\"tom\" pace=\"brisk\" -->\nA.\n\n<!-- voice tone=\"afraid\" -->\nB.\n\nC.\n<!-- voice end -->\n\nD.\n";
        let (s, e) = one(text);
        assert!(e.is_empty(), "{e:?}");
        let ps = &s.chapters[0].passages;
        assert_eq!(ps.len(), 4);
        assert_eq!(ps[0].speaker.0, "tom");
        assert_eq!(ps[0].pace, Pace::Brisk);
        assert_eq!(ps[1].speaker.0, "tom");
        assert_eq!(ps[1].tones, vec!["afraid"]);
        assert_eq!(ps[1].pace, Pace::Brisk, "overlay keeps the span's pace");
        assert_eq!(ps[2].speaker.0, "tom");
        assert_eq!(ps[3].speaker.0, NARRATOR);
    }

    #[test]
    fn alias__resolves_to_the_canonical_id() {
        let (s, _) = one("<!-- voice speaker=\"the-kid\" -->\nYo.\n");
        assert_eq!(s.chapters[0].passages[0].speaker.0, "tom");
    }

    #[test]
    fn unknown_speaker__is_reported_and_falls_back_to_the_narrator() {
        let (s, e) = one("<!-- voice speaker=\"ghost\" -->\nBoo.\n");
        assert!(e.contains(&VoiceError::UnknownSpeaker {
            loc: Location::new("src/ch01.md", 1),
            speaker: "ghost".into()
        }));
        assert_eq!(s.chapters[0].passages[0].speaker.0, NARRATOR);
    }

    #[test]
    fn unknown_tone__is_reported_and_dropped() {
        let (s, e) = one("<!-- voice speaker=\"rosa\" tone=\"wry,smarmy\" -->\nHa.\n");
        assert!(e.contains(&VoiceError::UnknownTone {
            loc: Location::new("src/ch01.md", 1),
            tone: "smarmy".into()
        }));
        assert_eq!(s.chapters[0].passages[0].tones, vec!["wry"]);
    }

    #[test]
    fn cue__in_a_code_fence_is_not_a_cue() {
        let text = "```markdown\n<!-- voice speaker=\"ghost\" -->\n```\n\nProse.\n";
        let (s, e) = one(text);
        assert!(e.is_empty(), "{e:?}");
        let ps = &s.chapters[0].passages;
        assert_eq!(ps[0].kind, PassageKind::Code);
        assert_eq!(ps[0].text, vec!["<!-- voice speaker=\"ghost\" -->"]);
        assert_eq!(ps[1].kind, PassageKind::Prose);
    }

    #[test]
    fn cue__followed_by_a_heading_or_fence_or_cue_is_without_text() {
        for text in [
            "<!-- voice speaker=\"rosa\" -->\n# Heading\n",
            "<!-- voice speaker=\"rosa\" -->\n```\ncode\n```\n",
            "<!-- voice speaker=\"rosa\" -->\n<!-- voice speaker=\"tom\" -->\nX.\n",
            "<!-- voice speaker=\"rosa\" -->\n",
        ] {
            let (_, e) = one(text);
            assert!(
                e.contains(&VoiceError::CueWithoutText {
                    loc: Location::new("src/ch01.md", 1)
                }),
                "{text:?} -> {e:?}"
            );
        }
    }

    #[test]
    fn span__errors_are_located_at_the_begin_or_end() {
        let (_, e) = one("<!-- voice begin speaker=\"rosa\" -->\nA.\n");
        assert!(e.contains(&VoiceError::UnclosedSpan {
            loc: Location::new("src/ch01.md", 1)
        }));
        let (_, e) = one("A.\n<!-- voice end -->\n");
        assert!(e.contains(&VoiceError::OrphanEnd {
            loc: Location::new("src/ch01.md", 2)
        }));
        let (_, e) = one(
            "<!-- voice begin speaker=\"rosa\" -->\n<!-- voice begin speaker=\"tom\" -->\nA.\n<!-- voice end -->\n",
        );
        assert!(e.contains(&VoiceError::NestedSpan {
            loc: Location::new("src/ch01.md", 2)
        }));
    }

    #[test]
    fn bower_directives__and_other_comments_are_ignored() {
        let text = "<!-- bower repo=\"x\" file=\"a.rs\" -->\n```rust\nfn a() {}\n```\n\n<!-- TODO: tighten -->\nProse.\n";
        let (s, e) = one(text);
        assert!(e.is_empty(), "{e:?}");
        let ps = &s.chapters[0].passages;
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].kind, PassageKind::Code);
        assert_eq!(ps[1].text, vec!["Prose."]);
    }

    #[test]
    fn headings__are_always_the_narrator_even_inside_a_span() {
        let text = "<!-- voice begin speaker=\"rosa\" -->\n## Scene\nA.\n<!-- voice end -->\n";
        let (s, _) = one(text);
        let ps = &s.chapters[0].passages;
        assert_eq!(ps[0].kind, PassageKind::Heading);
        assert_eq!(ps[0].speaker.0, NARRATOR);
        assert_eq!(ps[1].speaker.0, "rosa");
    }
}
