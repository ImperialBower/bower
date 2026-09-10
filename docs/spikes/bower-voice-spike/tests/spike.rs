//! End-to-end: a small annotated book, its cast, one narrator's palette.
//! These are the tests the EPIC's testkit fixtures will grow from.

#![allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]

use bower_voice_spike::cast::NARRATOR;
use bower_voice_spike::prelude::*;

// ---------------------------------------------------------------- fixtures

fn cast() -> Cast {
    Cast::new(vec![
        Character::new(NARRATOR, "Christoph").with_tones(&["dry"]),
        Character::new("rosa", "Rosa Alvarez")
            .with_aliases(&["mrs-alvarez"])
            .with_tones(&["warm"])
            .with_target(Axes::new(45, 70, 55, 30)),
        Character::new("kid", "The Kid")
            .with_tones(&["bright"])
            .with_target(Axes::new(75, 20, 25, 80)),
        Character::new("dealer", "The Dealer")
            .with_tones(&["flat"])
            .with_target(Axes::new(48, 68, 58, 28)), // five units from rosa
        Character::new("boss", "The Boss")
            .with_tones(&["cold"])
            .with_target(Axes::new(10, 85, 95, 40)),
        Character::new("ghostwriter", "Undesigned"),
    ])
    .with_lexicon(Lexicon::default().with("pkcore", "pee-kay-core"))
}

fn palette() -> Palette {
    Palette::new(
        "christoph",
        vec![
            VoiceRegion::new("own", Axes::new(40, 50, 60, 40), 15),
            VoiceRegion::new("young", Axes::new(70, 25, 30, 70), 12),
            VoiceRegion::new("old-man", Axes::new(30, 80, 70, 30), 10),
        ],
    )
}

const CH01: &str = r#"# The first hand

The room smelled of old felt and older coffee.

<!-- voice speaker="rosa" tone="warm,tired" pause="beat" -->
"Sit down, kid. Nobody bites before midnight."

<!-- voice speaker="kid" tone="bright" pace="rushed" energy="loud" -->
"I'm not scared. I read the whole book."

<!-- bower repo="rust4failures" file="src/rank.rs" -->
```rust
pub enum Rank { Ace, King }
```

<!-- voice begin speaker="dealer" pace="slow" note="never looks up" -->
"Blinds are up."

"Ante."
<!-- voice end -->

She was lying about the biting.
"#;

const CH02: &str = r#"# Later

<!-- voice speaker="mrs-alvarez" -->
"Told you."

<!-- voice speaker="boss" tone="cold" energy="whisper" -->
"Out. Now."
"#;

fn chapters() -> Vec<Chapter> {
    vec![
        Chapter::new("src/ch01.md", CH01),
        Chapter::new("src/ch02.md", CH02),
    ]
}

// ------------------------------------------------------------------ script

#[test]
fn script__the_scene_resolves_cleanly() {
    let (s, e) = script(&chapters(), &cast());
    assert!(e.is_empty(), "{e:?}");
    let ps = &s.chapters[0].passages;

    assert_eq!(ps[0].kind, bower_voice_spike::script::PassageKind::Heading);
    assert_eq!(ps[1].speaker.0, NARRATOR);
    assert_eq!(
        ps[1].tones,
        vec!["dry"],
        "the declared narrator's default tone"
    );

    assert_eq!(ps[2].speaker.0, "rosa");
    assert_eq!(ps[2].tones, vec!["warm", "tired"]);
    assert_eq!(ps[2].pause, Some(bower_voice_spike::cue::Pause::Beat));

    assert_eq!(ps[3].speaker.0, "kid");
    assert_eq!(ps[3].pace, Pace::Rushed);
    assert_eq!(ps[3].energy, Energy::Loud);

    assert_eq!(ps[4].kind, bower_voice_spike::script::PassageKind::Code);
    assert_eq!(ps[4].text, vec!["pub enum Rank { Ace, King }"]);

    assert_eq!(ps[5].speaker.0, "dealer");
    assert_eq!(ps[6].speaker.0, "dealer");
    assert_eq!(ps[6].note.as_deref(), Some("never looks up"));
    assert_eq!(ps[6].pace, Pace::Slow);

    assert_eq!(ps[7].speaker.0, NARRATOR);
    assert_eq!(ps.len(), 8);

    assert_eq!(
        s.chapters[1].passages[1].speaker.0, "rosa",
        "alias resolved"
    );
}

#[test]
fn script__is_deterministic() {
    let (a, _) = script(&chapters(), &cast());
    let (b, _) = script(&chapters(), &cast());
    assert_eq!(a, b);
}

// --------------------------------------------------------------- breakdown

#[test]
fn breakdown__counts_the_whole_book_and_places_first_appearances() {
    let (s, _) = script(&chapters(), &cast());
    let b = breakdown(&s, &cast());

    assert_eq!(b.chapters.len(), 2);
    assert_eq!(b.total_code_lines, 1);
    assert_eq!(
        b.chapters[0].speakers,
        vec!["dealer", "kid", "narrator", "rosa"]
    );

    let rosa = b.characters.iter().find(|c| c.id == "rosa").unwrap();
    assert_eq!(rosa.first_at, Some(Location::new("src/ch01.md", 6)));
    assert_eq!(rosa.chapters, vec!["src/ch01.md", "src/ch02.md"]);
    assert_eq!(rosa.tones.get("warm"), Some(&2));
    assert_eq!(rosa.tones.get("tired"), Some(&1));

    let boss = b.characters.iter().find(|c| c.id == "boss").unwrap();
    assert_eq!(boss.chapters, vec!["src/ch02.md"]);

    let ghost = b.characters.iter().find(|c| c.id == "ghostwriter").unwrap();
    assert_eq!(ghost.passages, 0);

    let text = breakdown_text(&b);
    assert_eq!(text, breakdown_text(&breakdown(&s, &cast())));
    assert!(text.contains(&format!("{:<28} words=", "ch01")));
}

// --------------------------------------------------------------------- fit

#[test]
fn fit__places_every_designed_voice_and_grades_the_reach() {
    let (s, _) = script(&chapters(), &cast());
    let r = fit(&palette(), &cast(), &s).unwrap();

    let by = |id: &str| r.placements.iter().find(|p| p.character == id).unwrap();

    // rosa (45,70,55,30) vs old-man (30,80,70,30): d = sqrt(225+100+225) = 23 > 2*10 → out of reach
    // vs own (40,50,60,40): sqrt(25+400+25+100)= 23 → tie; name order picks "old-man".
    let rosa = by("rosa");
    assert_eq!(rosa.region, "old-man", "tie broken by region name, stably");
    assert!(matches!(rosa.reach, Reach::OutOfReach { .. }));

    // kid (75,20,25,80) vs young (70,25,30,70): sqrt(25+25+25+100)=13 > 12, ≤ 24 → stretch
    let kid = by("kid");
    assert_eq!(kid.region, "young");
    assert!(matches!(kid.reach, Reach::Stretch { .. }));
    if let Reach::Stretch { gap } = kid.reach {
        assert_eq!(
            (gap.pitch, gap.age, gap.weight, gap.energy),
            (5, -5, -5, 10)
        );
    }

    // boss (10,85,95,40) vs old-man (30,80,70,30): sqrt(400+25+625+100)=33 → out of reach
    let boss = by("boss");
    assert_eq!(boss.region, "old-man");
    assert!(matches!(boss.reach, Reach::OutOfReach { .. }));

    assert_eq!(r.undesigned, vec!["ghostwriter"]);
    assert!(
        r.placements.iter().all(|p| p.character != "ghostwriter"),
        "undesigned voices are not placed"
    );
}

#[test]
fn fit__flags_two_close_voices_only_where_they_share_a_chapter() {
    let (s, _) = script(&chapters(), &cast());
    let r = fit(&palette(), &cast(), &s).unwrap();

    // rosa and dealer are 5 apart and both speak in ch01.
    assert_eq!(r.collisions.len(), 1);
    let c = &r.collisions[0];
    assert_eq!((c.a.as_str(), c.b.as_str()), ("dealer", "rosa"));
    assert_eq!(c.distance, 5);
    assert_eq!(c.chapters, vec!["src/ch01.md"]);

    // Teeth: move the dealer to a chapter of his own, and the collision
    // disappears although the voices are exactly as close.
    let ch1 = CH01.replace("speaker=\"dealer\"", "speaker=\"kid\"");
    let ch3 = "# Alone\n\n<!-- voice speaker=\"dealer\" -->\n\"Blinds.\"\n";
    let (s2, e) = script(
        &[
            Chapter::new("src/ch01.md", &ch1),
            Chapter::new("src/ch02.md", CH02),
            Chapter::new("src/ch03.md", ch3),
        ],
        &cast(),
    );
    assert!(e.is_empty(), "{e:?}");
    let r2 = fit(&palette(), &cast(), &s2).unwrap();
    assert!(
        r2.collisions.is_empty(),
        "no shared chapter, no collision: {:?}",
        r2.collisions
    );
}

#[test]
fn fit__text_is_stable_and_names_the_gap() {
    let (s, _) = script(&chapters(), &cast());
    let r = fit(&palette(), &cast(), &s).unwrap();
    let t = fit_text(&r);
    assert_eq!(t, fit_text(&r));
    let kid_line = format!(
        "{:<12} -> {:<12} d={:<3} stretch  (pitch +5 age -5 weight -5 energy +10)\n",
        "kid", "young", 13
    );
    assert!(t.contains(&kid_line), "{t}");
    assert!(t.contains("COLLISION dealer ~ rosa d=5 in src/ch01.md"));
    assert!(t.contains("undesigned ghostwriter"));
}

#[test]
fn fit__refuses_an_invalid_palette_rather_than_guessing() {
    let (s, _) = script(&chapters(), &cast());
    let e = fit(&Palette::new("x", vec![]), &cast(), &s).unwrap_err();
    assert!(e.contains(&VoiceError::PaletteEmpty));
}

// ------------------------------------------------- every error has a fixture

#[test]
fn errors__every_variant_is_producible_from_a_fixture() {
    let at = |l: usize| Location::new("src/ch01.md", l);
    let run = |text: &str| script(&[Chapter::new("src/ch01.md", text)], &cast()).1;

    let cases: Vec<(&str, VoiceError)> = vec![
        (
            "<!-- voice speaker=rosa -->\nX.\n",
            VoiceError::CueParse {
                loc: at(1),
                reason: "value for `speaker` must be double-quoted".into(),
            },
        ),
        (
            "<!-- voice colour=\"red\" -->\nX.\n",
            VoiceError::UnknownKey {
                loc: at(1),
                key: "colour".into(),
            },
        ),
        (
            "<!-- voice energy=\"eleven\" -->\nX.\n",
            VoiceError::BadValue {
                loc: at(1),
                key: "energy".into(),
                value: "eleven".into(),
            },
        ),
        (
            "<!-- voice speaker=\"nobody\" -->\nX.\n",
            VoiceError::UnknownSpeaker {
                loc: at(1),
                speaker: "nobody".into(),
            },
        ),
        (
            "<!-- voice tone=\"smarmy\" -->\nX.\n",
            VoiceError::UnknownTone {
                loc: at(1),
                tone: "smarmy".into(),
            },
        ),
        (
            "<!-- voice speaker=\"rosa\" -->\n",
            VoiceError::CueWithoutText { loc: at(1) },
        ),
        (
            "<!-- voice begin speaker=\"rosa\" -->\nX.\n",
            VoiceError::UnclosedSpan { loc: at(1) },
        ),
        (
            "<!-- voice begin speaker=\"rosa\" -->\n<!-- voice begin speaker=\"kid\" -->\nX.\n<!-- voice end -->\n",
            VoiceError::NestedSpan { loc: at(2) },
        ),
        (
            "X.\n<!-- voice end -->\n",
            VoiceError::OrphanEnd { loc: at(2) },
        ),
        (
            "<!-- voice end speaker=\"rosa\" -->\n",
            VoiceError::EndWithKeys { loc: at(1) },
        ),
    ];
    for (text, want) in cases {
        let got = run(text);
        assert!(
            got.contains(&want),
            "{text:?}\n  wanted {want:?}\n  got {got:?}"
        );
    }

    // The four that come from cast and palette rather than chapters.
    let dup = vec![Character::new("a", "A"), Character::new("a", "A2")];
    assert!(
        Cast::validate(&dup, &Tones::standard())
            .contains(&VoiceError::DuplicateCharacter { id: "a".into() })
    );
    let bad_tone = vec![Character::new("a", "A").with_tones(&["smarmy"])];
    assert!(
        Cast::validate(&bad_tone, &Tones::standard()).contains(&VoiceError::CastUnknownTone {
            id: "a".into(),
            tone: "smarmy".into()
        })
    );
    assert!(
        Palette::new("x", vec![])
            .validate()
            .contains(&VoiceError::PaletteEmpty)
    );
    let two = vec![
        VoiceRegion::new("r", Axes::default(), 1),
        VoiceRegion::new("r", Axes::default(), 1),
    ];
    assert!(
        Palette::new("x", two)
            .validate()
            .contains(&VoiceError::DuplicateRegion { name: "r".into() })
    );
}

#[test]
fn errors__display_is_grep_shaped() {
    let e = VoiceError::UnknownSpeaker {
        loc: Location::new("src/ch07.md", 42),
        speaker: "ghost".into(),
    };
    assert_eq!(
        e.to_string(),
        "src/ch07.md:42: speaker `ghost` is not in the cast"
    );
}

// ------------------------------------------------------ the eye is untouched

#[test]
fn cues__are_html_comments_so_a_reader_never_sees_them() {
    // The whole argument for the syntax: strip cue lines and the chapter is
    // the chapter. No renderer needs to learn anything.
    let visible: Vec<&str> = CH01
        .lines()
        .filter(|l| !CueDirective::is_cue_line(l))
        .collect();
    assert!(visible.iter().all(|l| !l.contains("<!-- voice")));
    assert!(visible.iter().any(|l| l.contains("Sit down, kid.")));
}
