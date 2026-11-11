//! IPA marks that change how a sound is made (#32 step 1): the tokenizer keeps them as flags on the
//! segment, the planner turns each into gestures on controls the engine has, and a sung syllable
//! keeps them. Unflagged segments plan exactly as before (the planning goldens). Rendered tests
//! are in `sound_marks_acoustic.rs`.
use chimera::anatomy::human::{ArticKey, BodyMix};
use chimera::control::score::Gesture;
use chimera::phonetics::ipa::tokenize_ipa;
use chimera::phonetics::planner::{PlanOptions, plan_utterance};
use chimera::phonetics::sing::{NoteOnOptions, SongOptions, SungNote, plan_note_on, plan_song};
use chimera::phonetics::{Plan, Segment, Syllable};

fn plan(segs: &[Segment]) -> Plan {
    plan_utterance(segs, &PlanOptions { body: BodyMix::MALE, ..Default::default() })
}

fn labelled<'a>(gs: &'a [Gesture], label: &str) -> Vec<&'a Gesture> {
    gs.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == label)).collect()
}

/// (ipa, flagged) of every segment of `ipa`, by a flag.
fn flags(ipa: &str, flag: fn(&Segment) -> Option<bool>) -> Vec<(String, bool)> {
    tokenize_ipa(ipa).iter().map(|s| (s.ipa.clone(), flag(s) == Some(true))).collect()
}

fn nasalized(s: &Segment) -> Option<bool> {
    s.nasalized
}

fn pairs(v: &[(&str, bool)]) -> Vec<(String, bool)> {
    v.iter().map(|(a, b)| (a.to_string(), *b)).collect()
}

#[test]
fn a_combining_tilde_nasalizes_the_vowel_before_it() {
    // ɛ̃ ɔ̃ ɑ̃ œ̃ have no precomposed form
    assert_eq!(flags("p\u{25B}\u{303}", nasalized), pairs(&[("p", false), ("ɛ", true)]));
    assert_eq!(flags("\u{153}\u{303}", nasalized), pairs(&[("œ", true)]));
    // a diphthong, and the tilde after the length mark
    assert_eq!(flags("a\u{26A}\u{303}", nasalized), pairs(&[("aɪ", true)]));
    let long = tokenize_ipa("\u{25B}\u{2D0}\u{303}");
    assert_eq!((long.len(), long[0].nasalized, long[0].dur), (1, Some(true), Some(0.15 * 1.7)));
}

#[test]
fn nasalized_vowels_read_the_same_precomposed_or_not() {
    // NFC joins a vowel and its tilde into one code point (ã õ ẽ ĩ ũ ỹ), which used to be dropped
    for (precomposed, base) in [("\u{E3}", "a"), ("\u{F5}", "o"), ("\u{1EBD}", "e"), ("\u{129}", "i"), ("\u{169}", "u"), ("\u{1EF9}", "y")] {
        let decomposed = format!("{base}\u{303}");
        assert_eq!(tokenize_ipa(precomposed), tokenize_ipa(&decomposed), "{precomposed}");
        assert_eq!(flags(precomposed, nasalized), pairs(&[(base, true)]), "{precomposed}");
    }
}

#[test]
fn a_tilde_on_a_consonant_is_ignored() {
    // an oral vowel after it stays oral; the consonant is as it was
    assert_eq!(tokenize_ipa("m\u{303}a"), tokenize_ipa("ma"));
    assert_eq!(tokenize_ipa("a \u{303}"), tokenize_ipa("a "));
}

#[test]
fn the_owners_french_pair_now_differs() {
    // paix / pain were both "p ɛ"
    assert_ne!(tokenize_ipa("pɛ"), tokenize_ipa("p\u{25B}\u{303}"));
    assert_eq!(tokenize_ipa("pɛ").iter().map(|s| &s.ipa).collect::<Vec<_>>(), tokenize_ipa("p\u{25B}\u{303}").iter().map(|s| &s.ipa).collect::<Vec<_>>());
}

#[test]
fn a_nasalized_vowel_lowers_the_velum_for_the_whole_vowel() {
    let oral = plan(&tokenize_ipa("pɛ"));
    let nasal = plan(&tokenize_ipa("p\u{25B}\u{303}"));
    assert!(labelled(&oral.gestures, "velum").is_empty());
    let v = labelled(&nasal.gestures, "velum");
    assert_eq!(v.len(), 1);
    let vowel = nasal.segments.iter().find(|s| s.kind == "vowel").unwrap();
    assert_eq!((v[0].t0, v[0].t1), (vowel.t0, vowel.t1));
    assert!(v[0].targets.get(ArticKey::Velum).unwrap() > 0.1);
    // nothing else about the plan changes
    // (compared as JSON: unset targets are NaN)
    let rest = |p: &Plan| serde_json::to_string(&p.gestures.iter().filter(|g| g.label.as_ref().is_none_or(|l| l.as_str() != "velum")).collect::<Vec<_>>()).unwrap();
    assert_eq!(rest(&oral), rest(&nasal));
    assert_eq!(oral.segments, nasal.segments);
}

#[test]
fn a_nasalized_diphthong_keeps_the_velum_down_through_its_glide() {
    let p = plan(&tokenize_ipa("a\u{26A}\u{303}"));
    let v = labelled(&p.gestures, "velum");
    assert_eq!(v.len(), 1);
    assert_eq!((v[0].t0, v[0].t1), (p.segments[0].t0, p.segments[1].t1));
}

fn syl(onset: Vec<Segment>, nucleus: Segment) -> Syllable {
    Syllable { text: "x".into(), onset, nucleus, coda: vec![], melisma: None, word_end: true, phrase_end: false }
}

#[test]
fn a_sung_vowel_keeps_its_nasalization() {
    let nucleus = |n: Option<bool>| Segment { nasalized: n, ..Segment::new("ɛ") };
    let sung = |n| plan_song(&[SungNote { midi: 57.0, beats: 1.0, syl: syl(vec![Segment::new("p")], nucleus(n)) }], &SongOptions { body: "male".into(), bpm: 90.0, t0: 0.5, vibrato: 0.0 });
    assert_eq!(labelled(&sung(Some(true)).gestures, "velum").len(), 1);
    assert!(labelled(&sung(None).gestures, "velum").is_empty());
    // a live note
    let live = |n| plan_note_on(&syl(vec![Segment::new("p")], nucleus(n)), 57.0, &NoteOnOptions { body: "male".into(), vibrato: 0.0, tag: 1.0, legato_from: None });
    assert_eq!(labelled(&live(Some(true)), "velum").len(), 1);
    assert!(labelled(&live(None), "velum").is_empty());
    // a nasalized diphthong, sung: each half is a vowel of its own
    let diph = Segment { nasalized: Some(true), ..Segment::new("aɪ") };
    let p = plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl: syl(vec![], diph) }], &SongOptions { body: "male".into(), bpm: 90.0, t0: 0.5, vibrato: 0.0 });
    assert_eq!(labelled(&p.gestures, "velum").len(), 2);
}
