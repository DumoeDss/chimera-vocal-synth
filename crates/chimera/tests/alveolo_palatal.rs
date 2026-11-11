//! The consonants of #32 step 3 that Mandarin and Korean need: ɕ ʑ, the affricates tɕ dʑ tʂ dʐ, and
//! ɥ. What is in the inventory, how the tokenizer reads them, and what the sound marks do to them.
//! Rendered tests are in `alveolo_palatal_acoustic.rs`.
use chimera::anatomy::human::{ArticKey, BodyMix};
use chimera::control::score::Gesture;
use chimera::phonetics::inventory::{Manner, Voicing, phoneme};
use chimera::phonetics::ipa::tokenize_ipa;
use chimera::phonetics::planner::{PlanOptions, plan_utterance};
use chimera::phonetics::{Plan, Segment};

fn plan(segs: &[Segment]) -> Plan {
    plan_utterance(segs, &PlanOptions { body: BodyMix::MALE, ..Default::default() })
}

fn labelled<'a>(gs: &'a [Gesture], label: &str) -> Vec<&'a Gesture> {
    gs.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == label)).collect()
}

fn ipas(s: &str) -> Vec<String> {
    tokenize_ipa(s).into_iter().map(|s| s.ipa).collect()
}

#[test]
fn the_seven_sounds_are_in_the_inventory_for_both_bodies() {
    use Manner::{Affricate, Approximant, Fricative};
    use Voicing::{Voiced, Voiceless};
    for body in [BodyMix::MALE, BodyMix::FEMALE] {
        for (ipa, manner, voicing, place, release) in [
            ("ɕ", Fricative, Voiceless, "alveolo-palatal", None),
            ("ʑ", Fricative, Voiced, "alveolo-palatal", None),
            ("tɕ", Affricate, Voiceless, "alveolo-palatal", Some("ɕ")),
            ("dʑ", Affricate, Voiced, "alveolo-palatal", Some("ʑ")),
            ("tʂ", Affricate, Voiceless, "retroflex", Some("ʂ")),
            ("dʐ", Affricate, Voiced, "retroflex", Some("ʐ")),
            ("ɥ", Approximant, Voiced, "labial-palatal", None),
        ] {
            let d = phoneme(ipa, body).unwrap_or_else(|| panic!("{ipa} missing"));
            assert_eq!((d.manner, d.voicing, d.place, d.release), (manner, voicing, place, release), "{ipa}");
        }
    }
}

#[test]
fn the_four_affricates_are_one_segment_with_or_without_a_tie_bar() {
    for (plain, tied) in [("tɕ", "t\u{361}ɕ"), ("dʑ", "d\u{361}ʑ"), ("tʂ", "t\u{361}ʂ"), ("dʐ", "d\u{361}ʐ")] {
        assert_eq!(ipas(&format!("{plain}a")), [plain, "a"], "{plain}");
        assert_eq!(tokenize_ipa(&format!("{tied}a")), tokenize_ipa(&format!("{plain}a")), "{tied:?}");
        // the tie bar below too
        let below = format!("{}\u{35C}{}a", plain.chars().next().unwrap(), plain.chars().nth(1).unwrap());
        assert_eq!(tokenize_ipa(&below), tokenize_ipa(&format!("{plain}a")), "{below:?}");
    }
    // not joined when something comes between, and a stop before a different fricative stays two
    assert_eq!(ipas("tɕ"), ["tɕ"]);
    assert_eq!(ipas("tɹɕ"), ["t", "ɹ", "ɕ"]);
    assert_eq!(ipas("dɕ"), ["d", "ɕ"]);
    assert_eq!(ipas("tʃ ts"), ["tʃ", "ts"]);
    // the single sounds are read as themselves
    assert_eq!(ipas("ɕiɛ ʑ ɥɛ"), ["ɕ", "i", "ɛ", "ʑ", "ɥ", "ɛ"]);
}

#[test]
fn the_pinyin_in_the_issue_reads_whole() {
    // issue #52: Mandarin "xie" as the owner's user typed it; ɕ used to be skipped
    assert_eq!(ipas("ɕiɛ˥˩ɕiɛ"), ["ɕ", "i", "ɛ", "ɕ", "i", "ɛ"]);
}

#[test]
fn the_aspiration_marks_work_on_the_new_affricates() {
    for c in ["tɕ", "tʂ"] {
        let asp = tokenize_ipa(&format!("{c}ʰa"));
        assert_eq!((asp[0].ipa.as_str(), asp[0].aspirated), (c, Some(true)), "{c}ʰ");
        let un = tokenize_ipa(&format!("{c}\u{2ED}a")); // ˭
        assert_eq!((un[0].ipa.as_str(), un[0].aspirated), (c, Some(false)), "{c}˭");
        // the same with a tie bar, and the plain one leaves it to the planner's English rule
        assert_eq!(tokenize_ipa(&format!("{}\u{361}{}ʰa", &c[..1], &c[1..])), asp);
        assert_eq!(tokenize_ipa(&format!("{c}a"))[0].aspirated, None);
        // the planner follows the flag: a longer release and the folds parted for longer
        let g = |s: &[Segment]| {
            let p = plan(s);
            let a = labelled(&p.gestures, "aspiration")[0];
            (a.t1 - p.segments[0].t1, a.targets.get(chimera::control::score::ControlKey::ABDUCTION).unwrap())
        };
        let (aspirated, unaspirated) = (g(&tokenize_ipa(&format!("{c}ʰa"))), g(&tokenize_ipa(&format!("{c}\u{2ED}a"))));
        assert!(aspirated.0 > unaspirated.0 + 0.01 && aspirated.1 > unaspirated.1, "{c}: {aspirated:?} against {unaspirated:?}");
    }
}

#[test]
fn a_breathy_release_works_on_the_voiced_affricates() {
    for c in ["dʑ", "dʐ"] {
        let s = tokenize_ipa(&format!("{c}ʱa"));
        assert_eq!((s[0].ipa.as_str(), s[0].breathy), (c, Some(true)), "{c}ʱ");
        assert_eq!(labelled(&plan(&s).gestures, "breathy").len(), 1, "{c}");
    }
    // not after the voiceless ones
    assert!(tokenize_ipa("tɕʱa").iter().all(|s| s.breathy.is_none()));
}

#[test]
fn palatalization_adds_nothing_to_a_sound_that_is_already_palatal_and_fronts_a_retroflex() {
    let soft = |c: &str| labelled(&plan(&tokenize_ipa(&format!("{c}ʲa"))).gestures, "palatalized").len();
    // The alveolo-palatals and ɥ already hold the tongue body at /j/'s place (their closure or
    // constriction owns both of its coordinates): the mark is read and plans no gesture, as for ʲ on j or ɲ
    for c in ["ɕ", "ʑ", "tɕ", "dʑ", "ɥ"] {
        assert_eq!(tokenize_ipa(&format!("{c}ʲa"))[0].palatalized, Some(true), "{c}");
        let owns = phoneme(c, BodyMix::MALE).unwrap().dominant;
        assert!(owns.contains(&ArticKey::TbX) && owns.contains(&ArticKey::TbY), "{c}");
        assert_eq!(soft(c), 0, "{c}");
        // and nothing else about the plan changes
        assert_eq!(serde_json::to_string(&plan(&tokenize_ipa(&format!("{c}ʲa"))).gestures).unwrap(), serde_json::to_string(&plan(&tokenize_ipa(&format!("{c}a"))).gestures).unwrap(), "{c}");
    }
    // The retroflex affricates use the tip, as ʂ does: the body is free, so it goes toward /j/
    for c in ["tʂ", "dʐ", "ʂ"] {
        assert_eq!(soft(c), 1, "{c}");
        let p = plan(&tokenize_ipa(&format!("{c}ʲa")));
        let g = labelled(&p.gestures, "palatalized")[0];
        assert!(g.targets.get(ArticKey::TbX).is_some() && g.targets.get(ArticKey::TbY).is_some(), "{c}");
    }
}

#[test]
fn a_new_sound_between_vowels_plans_a_closure_or_constriction_and_nothing_unknown() {
    for c in ["ɕ", "ʑ", "tɕ", "dʑ", "tʂ", "dʐ", "ɥ"] {
        let p = plan(&tokenize_ipa(&format!("ˈa{c}a")));
        assert_eq!(p.segments.len(), 3, "{c}");
        assert_eq!(p.segments[1].ipa, c);
        assert!(p.segments[1].t1 > p.segments[1].t0, "{c}");
        assert!(p.end.is_finite(), "{c}");
    }
}
