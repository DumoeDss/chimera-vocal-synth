//! ɕ ʑ tɕ dʑ tʂ dʐ ɥ (#32 step 3), heard: each is said between two vowels through the real voice
//! (male and female bodies) and measured against the engine's own neighbours.
use chimera::analysis::fft::{Fft, hann, power_spectrum_db};
use chimera::analysis::lpc::{FormantOpts, estimate_formants};
use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE, HumanBody};
use chimera::phonetics::planner::{PlanOptions, plan_utterance};
use chimera::phonetics::{Intonation, Plan, Segment};
use chimera::voice::human_voice::{HumanVoice, SourcePatch, oversample_for};

const FS: f64 = 48000.0;
const VOICES: [(HumanBody, &str, f64); 2] = [(BODY_MALE, "male", 112.0), (BODY_FEMALE, "female", 205.0)];

struct Rendered {
    plan: Plan,
    fs: f64,
    out: Vec<f32>,
    /// glottal voicing (0..1) at the end of each 128-sample block
    voicing: Vec<f64>,
    /// the narrowest area (cm²) of the oral tract and lips at the end of each 128-sample block
    narrowest: Vec<f64>,
}

/// "a C a" with the pitch held level; the second vowel is stressed and held.
fn vcv(c: &str, dur: Option<f64>) -> Vec<Segment> {
    vec![Segment { dur: Some(0.25), ..Segment::new("a") }, Segment { dur, ..Segment::new(c) }, Segment { dur: Some(0.3), stress: Some(1), ..Segment::new("a") }]
}

fn render(segs: &[Segment], body: HumanBody, name: &str, f0: f64, fs: f64) -> Rendered {
    let mut v = HumanVoice::new(fs, body, oversample_for(fs));
    v.set_source(&SourcePatch::f0(f0));
    let plan = plan_utterance(segs, &PlanOptions { body: name.into(), t0: 0.05, f0: Some(f0), intonation: Some(Intonation::Flat), ..Default::default() });
    v.schedule(&plan.gestures);
    let mut out = vec![0.0f32; (fs * (plan.end + 0.3)).round() as usize];
    let (mut voicing, mut narrowest) = (vec![], vec![]);
    for c in out.chunks_mut(128) {
        v.render(c);
        voicing.push(v.glottis.voicing);
        narrowest.push(v.airway.frame.as_ref().map_or(f64::MAX, |f| f.grid.iter().filter(|g| g.region >= 2).map(|g| g.area).fold(f64::MAX, f64::min)));
    }
    Rendered { plan, fs, out, voicing, narrowest }
}

/// Each sound said between vowels: a fricative or glide held for 200 ms, an affricate at its
/// natural length.
fn say(c: &str, body: HumanBody, name: &str, f0: f64) -> Rendered {
    render(&vcv(c, (!c.starts_with(['t', 'd'])).then_some(0.2)), body, name, f0, FS)
}

impl Rendered {
    fn block(&self, t: f64) -> usize {
        ((t * self.fs / 128.0) as usize).min(self.voicing.len() - 1)
    }
    /// mean voicing over [a, b]
    fn voicing_between(&self, a: f64, b: f64) -> f64 {
        let v = &self.voicing[self.block(a)..=self.block(b)];
        v.iter().sum::<f64>() / v.len() as f64
    }
    /// the narrowest oral area (cm²) over [a, b]: 0 where the tract is sealed
    fn narrowest_between(&self, a: f64, b: f64) -> f64 {
        self.narrowest[self.block(a)..=self.block(b)].iter().fold(f64::MAX, |m, x| m.min(*x))
    }
    /// power spectrum (dB) of the 1024 samples centred on t (21 ms at 48 kHz)
    fn spectrum(&self, t: f64) -> Vec<f64> {
        const N: usize = 1024;
        let mid = (t * self.fs) as usize;
        let frame: Vec<f64> = self.out[mid - N / 2..mid + N / 2].iter().map(|x| *x as f64).collect();
        power_spectrum_db(&frame, &Fft::new(N), &hann(N))
    }
    /// power-weighted mean frequency (Hz) between lo and hi of the window at t
    fn centroid(&self, t: f64, lo: f64, hi: f64) -> f64 {
        let db = self.spectrum(t);
        let df = self.fs / 1024.0;
        let (mut num, mut den) = (0.0, 0.0);
        for k in (lo / df) as usize..(hi / df) as usize {
            let p = 10f64.powf(db[k] / 10.0);
            num += p * k as f64 * df;
            den += p;
        }
        num / den
    }
    /// level (dB) of the power between lo and hi in the window at t
    fn level(&self, t: f64, lo: f64, hi: f64) -> f64 {
        let db = self.spectrum(t);
        let df = self.fs / 1024.0;
        10.0 * db[(lo / df) as usize..(hi / df) as usize].iter().map(|d| 10f64.powf(d / 10.0)).sum::<f64>().log10()
    }
    /// formant (Hz, 1-based) of the 40 ms centred on t: low-passed and decimated to a quarter of the
    /// rate first, so that an LPC of order 14 spends its poles on the first four formants
    fn formant(&self, t: f64, n: usize) -> f64 {
        let (mid, w) = ((t * self.fs) as usize, (0.04 * self.fs) as usize);
        let frame: Vec<f64> = self.out[mid - w / 2..mid + w / 2].iter().map(|x| *x as f64).collect();
        let low: Vec<f64> = frame.windows(8).step_by(4).map(|k| k.iter().sum::<f64>() / 8.0).collect();
        estimate_formants(&low, self.fs / 4.0, FormantOpts { order: Some(14), max_bw: Some(400.0), ..Default::default() }).get(n - 1).map_or(f64::NAN, |f| f.freq)
    }
    /// the consonant's middle 60 % (voicing is read there)
    fn middle(&self) -> (f64, f64) {
        let s = &self.plan.segments[1];
        (s.t0 + 0.2 * (s.t1 - s.t0), s.t1 - 0.2 * (s.t1 - s.t0))
    }
}

/// mean over five windows across the middle of the consonant
fn mean5(r: &Rendered, f: impl Fn(f64) -> f64) -> f64 {
    let s = &r.plan.segments[1];
    (0..5).map(|k| f(s.t0 + (0.3 + 0.1 * k as f64) * (s.t1 - s.t0))).sum::<f64>() / 5.0
}

#[test]
fn the_voiceless_alveolo_palatal_fricative_is_noise_and_sits_near_esh_and_well_below_s() {
    // The spectral centroid (Hz, 2-12 kHz, mean of five 21 ms windows across the middle of a
    // 200 ms consonant), the 3-12 kHz level against the 100-1000 Hz level (dB), the voicing:
    //   male   ʃ 4100   ɕ 3975   s 4987;  ɕ +41 dB over the low band, voicing 0.00
    //   female ʃ 3868   ɕ 4324   s 5827;  ɕ +40 dB over the low band, voicing 0.00
    // The order ʃ < ɕ < s holds for the female body (ɕ 456 Hz above ʃ) but not the male: there ɕ
    // sits 125 Hz (3 %) below ʃ. Reported, not tuned (#32 step 3). Asserted: ɕ is well below s
    // (by 1000 and 1500 Hz) and near ʃ, above it for the female body.
    for (body, name, f0) in VOICES {
        let centroid = |c: &str| {
            let r = say(c, body, name, f0);
            mean5(&r, |t| r.centroid(t, 2000.0, 12000.0))
        };
        let (sh, c, s) = (centroid("ʃ"), centroid("ɕ"), centroid("s"));
        println!("{name}: centroid ʃ {sh:.0}, ɕ {c:.0}, s {s:.0} Hz");
        assert!(c < s - 600.0, "{name}: ɕ {c:.0} Hz against s {s:.0} Hz");
        assert!(c > sh * 0.9 && c < sh * 1.2, "{name}: ɕ {c:.0} Hz against ʃ {sh:.0} Hz");
        if name == "female" {
            assert!(c > sh + 200.0, "{name}: ɕ {c:.0} Hz against ʃ {sh:.0} Hz");
        }
        let r = say("ɕ", body, name, f0);
        let (a, b) = r.middle();
        let noise = mean5(&r, |t| r.level(t, 3000.0, 12000.0) - r.level(t, 100.0, 1000.0));
        println!("{name}: ɕ frication {noise:.1} dB over the low band, voicing {:.2}", r.voicing_between(a, b));
        assert!(noise > 25.0, "{name}: ɕ has {noise:.1} dB of high band over low band");
        assert!(r.voicing_between(a, b) < 0.1, "{name}: ɕ is voiced");
    }
}

#[test]
fn the_voiced_alveolo_palatal_fricative_is_voiced_with_frication_on_top() {
    // voicing 1.00 in the middle in both bodies; the 3-12 kHz level (dB) against that of the glide
    // /j/, which has no frication: male ʑ 13.4, j 6.5; female ʑ 5.0, j 3.9
    for (body, name, f0) in VOICES {
        let (z, j) = (say("ʑ", body, name, f0), say("j", body, name, f0));
        let (a, b) = z.middle();
        let v = z.voicing_between(a, b);
        let (hz, hj) = (mean5(&z, |t| z.level(t, 3000.0, 12000.0)), mean5(&j, |t| j.level(t, 3000.0, 12000.0)));
        let low = mean5(&z, |t| z.level(t, 100.0, 1000.0));
        println!("{name}: ʑ voicing {v:.2}, low band {low:.1} dB, high band {hz:.1} dB, /j/ {hj:.1} dB");
        assert!(v > 0.8, "{name}: ʑ voicing {v:.2}");
        assert!(hz > hj + 0.5, "{name}: ʑ {hz:.1} dB against /j/ {hj:.1} dB in the high band");
        // the voice is there: the low band is loud, unlike ɕ's
        assert!(low > 15.0, "{name}: ʑ low band {low:.1} dB");
    }
}

#[test]
fn an_affricate_closes_and_then_frictions() {
    // The oral tract is sealed in the first third and open (0.05-0.13 cm²) in the last; the 3-12 kHz
    // level (dB) is quiet at 25 % of the sound and loud at 95 %, as for the engine's own tʃ:
    //   male   tɕ -42 -> 26, tʂ -9 -> 10, tʃ -5 -> 17;  female tɕ -61 -> 17, tʂ -11 -> 2, tʃ -14 -> 10
    // (tʂ's closure window picks up the vowel's tail, so its rise is the smallest.)
    for (body, name, f0) in VOICES {
        for (c, rise) in [("tɕ", 30.0), ("tʂ", 8.0), ("tʃ", 8.0)] {
            let r = say(c, body, name, f0);
            let s = &r.plan.segments[1];
            let d = s.t1 - s.t0;
            let (sealed, open) = (r.narrowest_between(s.t0, s.t0 + d / 3.0), r.narrowest_between(s.t0 + 0.66 * d, s.t1));
            let (quiet, loud) = (r.level(s.t0 + 0.25 * d, 3000.0, 12000.0), r.level(s.t0 + 0.95 * d, 3000.0, 12000.0));
            println!("{name} {c}: sealed {sealed:.4} cm², open {open:.3} cm², high band {quiet:.1} -> {loud:.1} dB");
            assert!(sealed < 1e-6, "{name} {c}: leaks ({sealed} cm²) in the closure");
            assert!(open > 0.03, "{name} {c}: the release stays shut ({open} cm²)");
            assert!(loud > quiet + rise, "{name} {c}: {quiet:.1} dB in the closure, {loud:.1} dB after the release");
        }
        // the voiceless one is voiceless throughout
        let r = say("tɕ", body, name, f0);
        let (a, b) = r.middle();
        let v = r.voicing_between(a, b);
        println!("{name}: tɕ voicing {v:.2}");
        assert!(v < 0.1, "{name}: tɕ is voiced ({v:.2})");
        // and the voiced ones have the voice on through the closure (0.58 to 0.76 in the middle 60 %, against 0.00-0.01 for tɕ)
        for c in ["dʑ", "dʐ"] {
            let r = say(c, body, name, f0);
            let (a, b) = r.middle();
            let v = r.voicing_between(a, b);
            println!("{name}: {c} voicing {v:.2}");
            assert!(v > 0.4, "{name} {c}: voicing {v:.2}");
        }
    }
}

#[test]
fn the_labial_palatal_glide_is_j_with_the_lips_rounded() {
    // Formants (Hz, mean of five 40 ms windows across a 200 ms glide, decimated LPC) of /j/, ɥ and /w/:
    //   male   F2 2051 / 2078 / 944;   F3 3967 / 2311 / 2215
    //   female F2 2618 / 2394 / 1041;  F3 3725 / 2638 / 2681
    // Rounding lowers F3 by 1.1-1.7 kHz in both bodies; F2 falls by 224 Hz in the female body and
    // not at all in the male (+27 Hz).
    for (body, name, f0) in VOICES {
        let (j, u) = (say("j", body, name, f0), say("ɥ", body, name, f0));
        let formant = |r: &Rendered, n| mean5(r, |t| r.formant(t, n));
        let (j2, u2, j3, u3) = (formant(&j, 2), formant(&u, 2), formant(&j, 3), formant(&u, 3));
        println!("{name}: j F2 {j2:.0} F3 {j3:.0}, ɥ F2 {u2:.0} F3 {u3:.0}");
        assert!(u3 < j3 - 600.0, "{name}: ɥ F3 {u3:.0} Hz, /j/ {j3:.0} Hz");
        assert!(u2 < j2 + 100.0, "{name}: ɥ F2 {u2:.0} Hz, /j/ {j2:.0} Hz");
        assert!(u2 + u3 < j2 + j3 - 600.0, "{name}: F2+F3 {:.0} against {:.0}", u2 + u3, j2 + j3);
        // still a front glide, not /w/
        assert!(u2 > 1500.0, "{name}: ɥ F2 {u2:.0} Hz");
    }
}

#[test]
fn the_new_sounds_hold_at_low_sample_rates() {
    // The rates of issue #33: nothing goes NaN or silent, and each sounds about as loud as at
    // 48 kHz (RMS of the whole utterance, within a factor of two).
    let rms = |r: &Rendered| (r.out.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / r.out.len() as f64).sqrt();
    for (body, name, f0) in VOICES {
        for c in ["ɕ", "ʑ", "tɕ", "dʑ", "tʂ", "dʐ", "ɥ"] {
            let reference = rms(&render(&vcv(c, None), body, name, f0, FS));
            for fs in [8000.0, 16000.0, 22050.0, 24000.0, 32000.0] {
                let r = render(&vcv(c, None), body, name, f0, fs);
                assert!(r.out.iter().all(|x| x.is_finite()), "{name} {c} at {fs} Hz: NaN");
                let s = &r.plan.segments[1];
                let peak = r.out[(s.t0 * fs) as usize..(s.t1 * fs) as usize].iter().fold(0.0f32, |m, x| m.max(x.abs()));
                println!("{name} {c} at {fs} Hz: RMS {:.4} (48 kHz {reference:.4}), peak in the consonant {peak:.3}", rms(&r));
                assert!(rms(&r) > 0.5 * reference && rms(&r) < 2.0 * reference, "{name} {c} at {fs} Hz: RMS {:.4}, 48 kHz {reference:.4}", rms(&r));
                assert!(peak > 1e-3, "{name} {c} at {fs} Hz: silent consonant ({peak})");
            }
        }
    }
}
