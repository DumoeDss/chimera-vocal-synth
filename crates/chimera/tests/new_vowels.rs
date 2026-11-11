//! The vowel added after the TypeScript port (ʏ): it has a recorded reference
//! (`vowel_data.rs`), a calibrated target for every body, and is said by the planner. The
//! calibrator was run with `--missing`, so the English and older vowels' entries are untouched.
use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE, BodyMix, HumanBody};
use chimera::phonetics::ipa::tokenize_ipa;
use chimera::phonetics::planner::{PlanOptions, is_vowel, plan_utterance, vowel_table};
use chimera::phonetics::vowel_data::VOWELS;
use chimera::voice::human_voice::{HumanVoice, SourcePatch};

/// the vowels this file covers: (ipa, largest fit error allowed in any formant, in %)
/// (ʏ fits as well as the English vowels: within 5 % on every formant and body)
const NEW: [(&str, f64); 1] = [("ʏ", 6.0)];
const NEAREST_MIN: f64 = 0.04;

fn reference(ipa: &str, body: &str) -> [f64; 3] {
    let v = VOWELS.iter().find(|v| v.ipa == ipa).unwrap();
    match body {
        "male" => v.male,
        "female" => v.female,
        _ => v.child,
    }
}

#[test]
fn the_new_vowel_is_calibrated_within_tolerance_of_its_reference_on_every_body() {
    let t = vowel_table();
    for (ipa, tol) in NEW {
        for body in ["male", "female", "child"] {
            let got = &t[body][ipa].formants;
            let want = reference(ipa, body);
            for i in 0..3 {
                let err = 100.0 * (got[i] - want[i]) / want[i];
                assert!(err.abs() <= tol, "{ipa} {body} F{}: {:.0} Hz against {:.0} ({err:+.1} %)", i + 1, got[i], want[i]);
            }
        }
    }
}

#[test]
fn the_new_vowel_is_distinct_from_every_other_calibrated_vowel() {
    // log-Hz distance over F1–F3 of what the airway produces (the achieved formants)
    let t = vowel_table();
    for (ipa, _) in NEW {
        for body in ["male", "female", "child"] {
            let mine = &t[body][ipa].formants;
            let mut nearest = (f64::MAX, "");
            for (other, v) in &t[body] {
                if other == ipa {
                    continue;
                }
                let d = (0..3).map(|i| (mine[i] / v.formants[i]).ln().powi(2)).sum::<f64>().sqrt();
                if d < nearest.0 {
                    nearest = (d, other);
                }
            }
            assert!(nearest.0 >= NEAREST_MIN, "{ipa} {body}: {:.3} from {}", nearest.0, nearest.1);
        }
    }
    // ʏ's two neighbours in the vowel chart, by name: close rounded ʏ is not y, lax ʏ is not ɪ
    let m = &t["male"];
    let d = |a: &str, b: &str| (0..3).map(|i| (m[a].formants[i] / m[b].formants[i]).ln().powi(2)).sum::<f64>().sqrt();
    assert!(d("ʏ", "y") > 0.2 && d("ʏ", "ɪ") > 0.2, "ʏ–y {:.3}, ʏ–ɪ {:.3}", d("ʏ", "y"), d("ʏ", "ɪ"));
}

#[test]
fn the_planner_says_the_new_vowel() {
    for (ipa, _) in NEW {
        assert!(is_vowel(ipa, BodyMix::MALE) && is_vowel(ipa, BodyMix::FEMALE), "{ipa}");
    }
    // Glück: before, the ʏ was dropped and the plan had no vowel at all
    let plan = plan_utterance(&tokenize_ipa("ɡlʏk"), &PlanOptions { body: BodyMix::MALE, ..Default::default() });
    let heard: Vec<&str> = plan.segments.iter().map(|s| s.ipa.as_str()).collect();
    assert_eq!(heard, ["g", "l", "ʏ", "k"]); // the tokenizer writes ɡ as g
    assert!(plan.segments.iter().any(|s| s.ipa == "ʏ" && s.t1 - s.t0 > 0.08), "{:?}", plan.segments);
}

/// peak and RMS of a held-open utterance, with every sample checked
fn render(ipa: &str, body: HumanBody, name: &str, f0: f64) -> (f64, f64) {
    let mut v = HumanVoice::new(48000.0, body, 2);
    v.set_source(&SourcePatch::f0(f0));
    let plan = plan_utterance(&tokenize_ipa(ipa), &PlanOptions { body: name.into(), t0: 0.05, f0: Some(f0), ..Default::default() });
    v.schedule(&plan.gestures);
    let mut out = vec![0.0f32; (48000.0 * (plan.end + 0.2)) as usize];
    for c in out.chunks_mut(128) {
        v.render(c);
    }
    assert!(out.iter().all(|x| x.is_finite()), "{ipa} {name}: NaN or infinity");
    let peak = out.iter().fold(0.0f64, |m, x| m.max(x.abs() as f64));
    let rms = (out.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / out.len() as f64).sqrt();
    (peak, rms)
}

#[test]
fn the_new_vowel_renders_on_male_and_female_without_nan_or_silence() {
    for ipa in ["ɡlʏk", "ʏ"] {
        for (body, name, f0) in [(BODY_MALE, "male", 112.0), (BODY_FEMALE, "female", 205.0)] {
            let (peak, rms) = render(ipa, body, name, f0);
            assert!(peak > 0.05 && rms > 0.005, "{ipa} {name}: peak {peak:.3}, rms {rms:.4}");
        }
    }
}
