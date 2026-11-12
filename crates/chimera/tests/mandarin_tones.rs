//! Mandarin's four tones, when a song asks for them (`bend_strength` 1; by default a song follows
//! its melody and draws none). Each tone is the pitch's path through the syllable, in semitones
//! from the note, the first tone's level: 2 rises to it from below, 3 falls to the bottom and comes
//! part of the way back, 4 falls from just above it. As bends held at the note's start, 2 and 3
//! were both a low start jumping up to the note, and a native listener heard every syllable as a
//! second tone.
use chimera::analysis::pitch::{YinOpts, yin_pitch};
use chimera::anatomy::human::BODY_MALE;
use chimera::phonetics::mandarin::{Pinyin, pinyin_syllable, tone_bends};
use chimera::phonetics::sing::{SongOptions, SungNote, plan_song};
use chimera::phonetics::syllables::FrontEnd;
use chimera::phonetics::{Plan, Syllable};
use chimera::voice::human_voice::{HumanVoice, SourcePatch, oversample_for};

const FS: f64 = 48000.0;
const BEAT_AT: f64 = 0.5;
const SECS: f64 = 0.4;
/// A3: low enough to fall 10 semitones and stay above the estimator's floor
const NOTE: f64 = 220.0;

fn plan(syl: Syllable, strength: Option<f64>) -> Plan {
    plan_song(&[SungNote { midi: 57.0, beats: SECS, syl }], &SongOptions { body: "male".into(), bpm: 60.0, t0: BEAT_AT, vibrato: 0.0, bend_strength: strength, ..Default::default() })
}

fn sung(py: &str) -> Vec<f32> {
    let p = plan(pinyin_syllable(py, py).unwrap(), Some(1.0));
    let mut v = HumanVoice::new(FS, BODY_MALE, oversample_for(FS));
    v.set_source(&SourcePatch::f0(NOTE));
    v.schedule(&p.gestures);
    let mut out = vec![0f32; (FS * (BEAT_AT + SECS + 0.3)) as usize];
    for block in out.chunks_mut(512) {
        v.render(block);
    }
    assert!(out.iter().all(|x| x.is_finite()));
    out
}

/// the pitch of the 40 ms centred on t, in semitones from the note; None where there is no voice
fn st(out: &[f32], t: f64) -> Option<f64> {
    let a = ((t - 0.02) * FS) as usize;
    let frame: Vec<f64> = out[a..a + (0.04 * FS) as usize].iter().map(|x| *x as f64).collect();
    let rms = (frame.iter().map(|x| x * x).sum::<f64>() / frame.len() as f64).sqrt();
    let p = yin_pitch(&frame, FS, YinOpts { min_f0: Some(60.0), max_f0: Some(600.0), ..Default::default() });
    (rms > 0.003 && p.f0 > 0.0).then(|| 12.0 * (p.f0 / NOTE).log2())
}

/// the pitch every 20 ms through the vowel (the beat is at 0.5 s, the note ends 50 ms early)
fn contour(out: &[f32]) -> Vec<f64> {
    (0..16).filter_map(|i| st(out, BEAT_AT + 0.03 + 0.02 * i as f64)).collect()
}

#[test]
fn each_tone_is_its_own_contour() {
    let c = |py: &str| contour(&sung(py));
    let (first, last) = (|c: &[f64]| c[0], |c: &[f64]| c[c.len() - 1]);
    let lowest = |c: &[f64]| c.iter().cloned().fold(f64::MAX, f64::min);
    // 1: level on the note
    let t1 = c("ma1");
    assert!(t1.iter().all(|x| x.abs() < 1.0), "1: {t1:.1?}");
    // 2: low and still at first, at the note by the end
    let t2 = c("ma2");
    assert!(first(&t2) < -3.5 && t2[t2.len() / 3] < -3.5 && last(&t2) > -1.5, "2: {t2:.1?}");
    // 3: down to the bottom and part of the way back, never near the note (it was: low, then the note)
    let t3 = c("ma3");
    assert!(first(&t3) < -4.0 && first(&t3) > -7.5, "3: {t3:.1?}");
    assert!(lowest(&t3) < -8.5 && last(&t3) > lowest(&t3) + 2.5 && last(&t3) < -3.0, "3: {t3:.1?}");
    // 4: from above the note to the bottom
    let t4 = c("ma4");
    assert!(first(&t4) > 1.0 && last(&t4) < -6.0, "4: {t4:.1?}");
    // neutral: low and falling a little
    let t5 = c("ma5");
    assert!(first(&t5) < -2.5 && last(&t5) < first(&t5) - 1.5 && last(&t5) > -8.5, "5: {t5:.1?}");
}

#[test]
fn a_fourth_tone_does_not_flick_up_as_the_voice_fades() {
    // the contour's last pitch holds through the fade instead of letting go back to the note
    let out = sung("ma4");
    let at_end = st(&out, BEAT_AT + SECS - 0.07).unwrap();
    for k in 0..6 {
        let t = BEAT_AT + SECS - 0.05 + 0.02 * k as f64;
        if let Some(x) = st(&out, t) {
            assert!(x < at_end + 0.7, "{:.0} ms past the note's end the pitch is {x:.1}, was {at_end:.1}", (t - (BEAT_AT + SECS - 0.05)) * 1000.0);
        }
    }
}

#[test]
fn a_nasal_coda_carries_the_end_of_the_tone() {
    // 烫 tang4 is still falling in its -ng, 糖 tang2 has reached the note there: the contour is
    // measured on the vowel and the coda together
    let coda = BEAT_AT + SECS - 0.05 - 0.04;
    let fall = st(&sung("tang4"), coda).unwrap();
    let rise = st(&sung("tang2"), coda).unwrap();
    assert!(fall < -5.5, "tang4's coda is at {fall:.1}");
    assert!(rise > -2.0, "tang2's coda is at {rise:.1}");
}

#[test]
fn a_third_tone_inside_a_word_has_no_rise() {
    // 老师 lǎoshī: a half third, low and falling; a word's last syllable keeps the full contour
    let w = Pinyin.word("lǎoshī");
    assert_eq!(w[0].pitch.len(), 1);
    assert!(w[0].pitch[0].to.unwrap() < w[0].pitch[0].st && w[0].pitch[0].st < -4.0);
    assert!(w[1].pitch.is_empty());
    assert_eq!(Pinyin.word("mǎ")[0].pitch, tone_bends(3));
    // sandhi comes first: 你好 is a second tone, then a full third
    let w = Pinyin.word("nǐhǎo");
    assert_eq!((w[0].pitch.clone(), w[1].pitch.clone()), (tone_bends(2), tone_bends(3)));
}

#[test]
fn a_song_draws_tones_only_when_asked_and_as_strongly_as_asked() {
    let bends = |strength| plan(pinyin_syllable("ma4", "ma4").unwrap(), strength).gestures.into_iter().filter(|g| g.label_str() == "bend").collect::<Vec<_>>();
    assert!(bends(None).is_empty() && bends(Some(0.0)).is_empty());
    let (full, half) = (bends(Some(1.0)), bends(Some(0.5)));
    assert!(full.len() >= 4 && full.len() == half.len());
    let f0 = |g: &chimera::control::score::Gesture| g.targets.get(chimera::control::score::ControlKey::F0).unwrap();
    // the last knot: 9 semitones down, or 4.5
    assert!((f0(full.last().unwrap()) / NOTE - 2f64.powf(-9.0 / 12.0)).abs() < 1e-9);
    assert!((f0(half.last().unwrap()) / NOTE - 2f64.powf(-4.5 / 12.0)).abs() < 1e-9);
}
