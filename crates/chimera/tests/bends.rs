//! Pitch bends: a syllable's front end shapes the pitch inside its note (start or end, capped in
//! length) when the song asks for them (`bend_strength` 1; by default, 0, the melody stays as
//! written); none leaves the note as written. And the vowel before a nasal coda is nasalised only
//! when asked.
use chimera::anatomy::human::ArticKey;
use chimera::control::score::{ControlKey, Gesture};
use chimera::phonetics::sing::{SongOptions, SungNote, midi_to_hz, plan_song};
use chimera::phonetics::{Bend, Edge, Segment, Syllable};

fn syl(onset: &str, v: &str, coda: &[&str], pitch: Vec<Bend>) -> Syllable {
    Syllable { text: v.into(), onset: vec![Segment::new(onset)], nucleus: Segment::new(v), coda: coda.iter().map(|c| Segment::new(c)).collect(), word_end: true, pitch, ..Default::default() }
}
/// the bends drawn as the front end wrote them (bend_strength 1); the default draws none
fn opts(nasal_lead: Option<f64>) -> SongOptions {
    SongOptions { body: "male".into(), bpm: 60.0, t0: 0.5, nasal_lead, bend_strength: Some(1.0), ..Default::default() }
}
fn labelled<'a>(gs: &'a [Gesture], l: &str) -> Vec<&'a Gesture> {
    gs.iter().filter(|g| g.label.as_ref().is_some_and(|x| x.as_str() == l)).collect()
}

#[test]
fn bends_colour_the_start_or_end_of_the_note_and_none_leave_it() {
    let hz = midi_to_hz(57.0);
    let one = |pitch| plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl: syl("l", "a", &[], pitch) }], &opts(None));
    assert!(labelled(&one(vec![]).gestures, "bend").is_empty());
    let up = Bend { at: Edge::Start, len: 0.35, max: 0.15, st: -2.0 };
    let down = Bend { at: Edge::End, len: 0.35, max: 0.15, st: -3.0 };
    let p = one(vec![up, down]);
    let b = labelled(&p.gestures, "bend");
    assert_eq!(b.len(), 2);
    // the start bend: from just before the beat, at most 0.15 s into a 2 s note, 2 semitones down
    assert!((b[0].targets.get(ControlKey::F0).unwrap() / hz - 2f64.powf(-2.0 / 12.0)).abs() < 1e-9);
    assert!(b[0].t0 < 0.5 && (b[0].t1 - 0.65).abs() < 1e-9);
    // the end bend: the last 0.15 s of the vowel
    assert!(b[1].targets.get(ControlKey::F0).unwrap() < hz && (b[1].t1 - b[1].t0 - 0.17).abs() < 1e-9);
}

#[test]
fn a_melisma_keeps_its_written_pitches() {
    let bend = vec![Bend { at: Edge::Start, len: 0.35, max: 0.15, st: -2.0 }];
    let first = syl("l", "a", &[], bend.clone());
    let cont = Syllable { text: "~".into(), nucleus: Segment::new("a"), melisma: Some(true), pitch: bend, ..Default::default() };
    let p = plan_song(&[SungNote { midi: 57.0, beats: 1.0, syl: first }, SungNote { midi: 59.0, beats: 1.0, syl: cont }], &opts(None));
    assert_eq!(labelled(&p.gestures, "bend").len(), 1, "only the syllable's first note bends");
}

#[test]
fn nasal_lead_nasalises_only_before_a_nasal_coda() {
    let notes = vec![SungNote { midi: 57.0, beats: 2.0, syl: syl("ɕ", "i", &["ŋ"], vec![]) }, SungNote { midi: 59.0, beats: 2.0, syl: syl("t", "a", &[], vec![]) }];
    assert!(labelled(&plan_song(&notes, &opts(None)).gestures, "nasal vowel").is_empty());
    let p = plan_song(&notes, &opts(Some(0.4)));
    let nv = labelled(&p.gestures, "nasal vowel");
    assert_eq!(nv.len(), 1, "only the syllable with a nasal coda");
    assert!(nv[0].targets.get(ArticKey::Velum).unwrap() > 0.0);
    assert!(nv[0].t0 > 0.5 && nv[0].t1 < 0.5 + 2.0);
}

#[test]
fn a_syllable_can_ask_for_its_own_breath() {
    let one = |breath| plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl: Syllable { breath, ..syl("l", "a", &[], vec![]) } }], &opts(None));
    assert!(labelled(&one(None).gestures, "syllable breath").is_empty());
    let p = one(Some(0.7));
    let b = labelled(&p.gestures, "syllable breath");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].targets.get(ControlKey::BREATH), Some(0.7));
}

#[test]
fn a_start_bend_begins_in_the_onset() {
    // a rising tone starts low: the onset consonant already sings the bend's pitch, not the note's
    let hz = midi_to_hz(57.0);
    let up = Bend { at: Edge::Start, len: 0.35, max: 0.15, st: -3.0 };
    let p = plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl: syl("m", "a", &[], vec![up]) }], &opts(None));
    let onset_f0: Vec<f64> = p.gestures.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "pitch") && g.t1 <= 0.5 + 1e-9).filter_map(|g| g.targets.get(ControlKey::F0)).collect();
    assert!(!onset_f0.is_empty());
    assert!(onset_f0.iter().all(|f| (f / hz - 2f64.powf(-3.0 / 12.0)).abs() < 1e-9), "{onset_f0:?}");
    // without a start bend the onset sings the note
    let plain = plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl: syl("m", "a", &[], vec![]) }], &opts(None));
    assert!(plain.gestures.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "pitch") && g.t1 <= 0.5 + 1e-9).filter_map(|g| g.targets.get(ControlKey::F0)).all(|f| (f - hz).abs() < 1e-9));
}

#[test]
fn before_ng_after_a_back_vowel_the_tongue_rises_toward_the_velum() {
    // in the nasal-lead window, -ng after ɑ gets the tongue body's approach; -n and other vowels don't
    let approach = |v: &str, coda: &str| {
        let p = plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl: Syllable { nasal_lead: Some(0.4), ..syl("t", v, &[coda], vec![]) } }], &opts(None));
        labelled(&p.gestures, "coda approach").len()
    };
    assert_eq!(approach("ɑ", "ŋ"), 1);
    assert_eq!(approach("ɑ", "n"), 0);
    assert_eq!(approach("ə", "ŋ"), 0);
    // no nasal lead, no approach
    let p = plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl: syl("t", "ɑ", &["ŋ"], vec![]) }], &opts(None));
    assert!(labelled(&p.gestures, "coda approach").is_empty());
}

/// a Mandarin 2nd and 4th tone's bends, as the front end writes them
fn tone_syl() -> Syllable {
    syl("m", "a", &[], vec![Bend { at: Edge::Start, len: 0.45, max: 0.25, st: -3.5 }, Bend { at: Edge::End, len: 0.4, max: 0.2, st: -4.0 }])
}

fn plan_with(strength: Option<f64>, syl: Syllable) -> chimera::phonetics::Plan {
    let o = SongOptions { bend_strength: strength, ..opts(None) };
    plan_song(&[SungNote { midi: 57.0, beats: 2.0, syl }], &o)
}

#[test]
fn by_default_the_melody_is_sung_as_written() {
    // singing follows the melody: no bend gestures and the onset on the note's pitch, so the plan is
    // exactly the plan of the same syllable with no bends at all; 0 says the same as unset
    let plain = Syllable { pitch: vec![], ..tone_syl() };
    let want = serde_json::to_string(&plan_with(None, plain.clone())).unwrap();
    for strength in [None, Some(0.0), Some(-1.0)] {
        let p = plan_with(strength, tone_syl());
        assert!(labelled(&p.gestures, "bend").is_empty(), "{strength:?}");
        assert_eq!(serde_json::to_string(&p).unwrap(), want, "{strength:?}");
    }
    let hz = midi_to_hz(57.0);
    assert!(plan_with(None, tone_syl()).gestures.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "pitch")).filter_map(|g| g.targets.get(ControlKey::F0)).all(|f| (f - hz).abs() < 1e-9));
}

#[test]
fn strength_one_draws_the_bends_as_written_and_scales_them() {
    let hz = midi_to_hz(57.0);
    let ratio = |p: &chimera::phonetics::Plan| labelled(&p.gestures, "bend").iter().map(|g| g.targets.get(ControlKey::F0).unwrap() / hz).collect::<Vec<_>>();
    // 1: today's bends, to the semitone (the same numbers `bends_colour_the_start_or_end...` checks)
    let one = plan_with(Some(1.0), tone_syl());
    let r = ratio(&one);
    assert_eq!(r.len(), 2);
    assert!((r[0] - 2f64.powf(-3.5 / 12.0)).abs() < 1e-12 && (r[1] - 2f64.powf(-4.0 / 12.0)).abs() < 1e-12, "{r:?}");
    // the bends' timing is the data's, whatever the strength; only the semitones scale
    let half = plan_with(Some(0.5), tone_syl());
    let h = ratio(&half);
    assert!((h[0] - 2f64.powf(-1.75 / 12.0)).abs() < 1e-12 && (h[1] - 2f64.powf(-2.0 / 12.0)).abs() < 1e-12, "{h:?}");
    let spans = |p: &chimera::phonetics::Plan| labelled(&p.gestures, "bend").iter().map(|g| (g.t0, g.t1)).collect::<Vec<_>>();
    assert_eq!(spans(&one), spans(&half));
    // and the start bend's pitch carries into the onset consonant
    let onset = |p: &chimera::phonetics::Plan| p.gestures.iter().filter(|g| g.label.as_ref().is_some_and(|l| l.as_str() == "pitch") && g.t1 <= 0.5 + 1e-9).filter_map(|g| g.targets.get(ControlKey::F0)).next().unwrap();
    assert!((onset(&one) / hz - 2f64.powf(-3.5 / 12.0)).abs() < 1e-9);
    assert!((onset(&half) / hz - 2f64.powf(-1.75 / 12.0)).abs() < 1e-9);
}
