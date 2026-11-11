//! The sound marks, heard: each is rendered through the real voice (male and female bodies) and
//! measured against the same sound without the mark.
use chimera::analysis::fft::{Fft, hann, power_spectrum_db};
use chimera::analysis::lpc::{FormantOpts, estimate_formants};
use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE, HumanBody};
use chimera::phonetics::planner::{PlanOptions, plan_utterance};
use chimera::phonetics::{Plan, Segment};
use chimera::voice::human_voice::{HumanVoice, SourcePatch};

const FS: f64 = 48000.0;
const VOICES: [(HumanBody, &str, f64); 2] = [(BODY_MALE, "male", 112.0), (BODY_FEMALE, "female", 205.0)];
/// analysis window (samples): 85 ms, about ten periods of the male voice
const N: usize = 4096;

struct Rendered {
    plan: Plan,
    out: Vec<f32>,
    /// velum port area (cm²) at the end of each 128-sample block
    port: Vec<f64>,
    /// glottal voicing (0..1) at the end of each 128-sample block
    voicing: Vec<f64>,
    /// the narrowest area (cm²) of the oral tract and lips at the end of each 128-sample block
    narrowest: Vec<f64>,
}

fn render(segs: &[Segment], body: HumanBody, name: &str, f0: f64) -> Rendered {
    let mut v = HumanVoice::new(FS, body, 2);
    v.set_source(&SourcePatch::f0(f0));
    let plan = plan_utterance(segs, &PlanOptions { body: name.into(), t0: 0.05, f0: Some(f0), ..Default::default() });
    v.schedule(&plan.gestures);
    let mut out = vec![0.0f32; (FS * (plan.end + 0.3)).round() as usize];
    let (mut port, mut voicing, mut narrowest) = (vec![], vec![], vec![]);
    for c in out.chunks_mut(128) {
        v.render(c);
        port.push(v.airway.frame.as_ref().map_or(0.0, |f| f.velum_port));
        voicing.push(v.glottis.voicing);
        narrowest.push(v.airway.frame.as_ref().map_or(f64::MAX, |f| f.grid.iter().filter(|g| g.region >= 2).map(|g| g.area).fold(f64::MAX, f64::min)));
    }
    Rendered { plan, out, port, voicing, narrowest }
}

impl Rendered {
    fn port_at(&self, t: f64) -> f64 {
        self.port[((t * FS / 128.0) as usize).min(self.port.len() - 1)]
    }
    /// power spectrum (dB) of the N samples centred on t
    fn spectrum(&self, t: f64) -> Vec<f64> {
        let mid = (t * FS) as usize;
        let frame: Vec<f64> = self.out[mid - N / 2..mid + N / 2].iter().map(|x| *x as f64).collect();
        power_spectrum_db(&frame, &Fft::new(N), &hann(N))
    }
}

impl Rendered {
    /// when the voice sets in after `t` (the first block with voicing above one half)
    fn voicing_onset(&self, t: f64) -> f64 {
        let first = (t * FS / 128.0) as usize;
        (first..self.voicing.len()).find(|&i| self.voicing[i] > 0.5).expect("the voice sets in") as f64 * 128.0 / FS
    }
    /// the widest "narrowest oral area" (cm²) over [a, b]: 0 while the tract stays sealed
    fn min_area_between(&self, a: f64, b: f64) -> f64 {
        let blocks = (a * FS / 128.0) as usize..(b * FS / 128.0) as usize;
        self.narrowest[blocks].iter().fold(0.0, |m, x| m.max(*x))
    }
    /// second formant (Hz) of the 30 ms centred on t
    fn f2(&self, t: f64) -> f64 {
        let (mid, w) = ((t * FS) as usize, (0.03 * FS) as usize);
        let frame: Vec<f64> = self.out[mid - w / 2..mid + w / 2].iter().map(|x| *x as f64).collect();
        estimate_formants(&frame, FS, FormantOpts::default())[1].freq
    }
}

/// Level (dB) of the power in [lo, hi) Hz of a spectrum from [`Rendered::spectrum`].
fn band(db: &[f64], lo: f64, hi: f64) -> f64 {
    let df = FS / N as f64;
    10.0 * db[(lo / df) as usize..(hi / df) as usize].iter().map(|d| 10f64.powf(d / 10.0)).sum::<f64>().log10()
}

/// A held vowel, steady enough to analyse.
fn held(ipa: &str, nasalized: bool) -> Vec<Segment> {
    vec![Segment { dur: Some(0.5), nasalized: nasalized.then_some(true), ..Segment::new(ipa) }]
}

#[test]
fn a_nasalized_vowel_is_nasal_and_an_oral_one_is_not() {
    // Nasality is audible from a velum port of about 0.01 cm² (Bunton & Story, as in
    // speech_quality.rs). The first formant's region (400-900 Hz) also loses level against the
    // whole voice when the nasal tract is coupled in (the nasal vowel's A1 drop). Measured on a
    // held vowel, the F1-band share of the total (dB), oral → nasalized:
    //   male /ɛ/ -4.2 → -7.8, /a/ -4.5 → -5.8;  female /ɛ/ -1.0 → -2.4, /a/ -0.9 → -3.0;
    // the port 0 → 0.30 (male), 0.24 (female; the voice scales the port with the body).
    for (body, name, f0) in VOICES {
        for vowel in ["ɛ", "a"] {
            let oral = render(&held(vowel, false), body, name, f0);
            let nasal = render(&held(vowel, true), body, name, f0);
            let seg = &oral.plan.segments[0];
            let mid = (seg.t0 + seg.t1) / 2.0;
            assert!(oral.port_at(mid) < 0.01, "{vowel} ({name}): oral vowel, port {}", oral.port_at(mid));
            assert!(nasal.port_at(mid) > 0.1, "{vowel} ({name}): nasalized vowel, port {}", nasal.port_at(mid));
            let share = |r: &Rendered| {
                let db = r.spectrum(mid);
                band(&db, 400.0, 900.0) - band(&db, 100.0, 4000.0)
            };
            let drop = share(&oral) - share(&nasal);
            println!("{vowel} ({name}): F1-band share {:.1} dB oral, {:.1} dB nasalized", share(&oral), share(&nasal));
            assert!(drop > 1.0, "{vowel} ({name}): the F1 band falls only {drop:.1} dB when nasalized");
        }
    }
}

#[test]
fn a_nasalized_vowel_leaves_the_stop_before_it_closed() {
    // the velum opens as the vowel starts, after the /p/ has been sealed: the port is shut
    // through the closure and the burst is not lost
    for (body, name, f0) in VOICES {
        let segs = |n: bool| vec![Segment::new("p"), Segment { stress: Some(1), nasalized: n.then_some(true), ..Segment::new("ɛ") }];
        let oral = render(&segs(false), body, name, f0);
        let nasal = render(&segs(true), body, name, f0);
        let p = &nasal.plan.segments[0];
        assert!(nasal.port_at(p.t1 - 0.02) < 0.01, "{name}: port {} 20 ms before the release", nasal.port_at(p.t1 - 0.02));
        let burst = |r: &Rendered| {
            let a = ((p.t1 - 0.01) * FS) as usize;
            r.out[a..a + (0.04 * FS) as usize].iter().fold(0.0f64, |m, x| m.max(x.abs() as f64))
        };
        assert!(burst(&nasal) > 0.5 * burst(&oral), "{name}: burst {} nasalized, {} oral", burst(&nasal), burst(&oral));
        assert!(nasal.out.iter().all(|x| x.is_finite()));
    }
}

/// A consonant and a vowel, the vowel held and stressed.
fn cv(consonant: Segment, vowel: &str) -> Vec<Segment> {
    vec![consonant, Segment { dur: Some(0.35), stress: Some(1), ..Segment::new(vowel) }]
}

fn palatalized(ipa: &str) -> Segment {
    Segment { palatalized: Some(true), ..Segment::new(ipa) }
}

#[test]
fn a_palatalized_consonant_starts_its_vowel_with_a_high_f2() {
    // The vowel's F2 (LPC, 30 ms window) as the voice sets in after the release, and once the
    // vowel has settled, /a/ after a plain and a palatalized consonant (Hz):
    //   male   /ta/ 1485 → /tʲa/ 2092 (+607);  /ka/ 1464 → /kʲa/ 1724 (+260)
    //   female /ta/ 1790 → /tʲa/ 2269 (+479);  /ka/ 1779 → /kʲa/ 2117 (+338)
    // and 200 ms later both are back at the vowel's own F2, within 27 Hz (the largest gap, /kʲa/ male).
    for (body, name, f0) in VOICES {
        for (c, rise) in [("t", 350.0), ("p", 350.0), ("k", 150.0)] {
            let plain = render(&cv(Segment::new(c), "a"), body, name, f0);
            let pal = render(&cv(palatalized(c), "a"), body, name, f0);
            let release = plain.plan.segments[0].t1;
            let (on_plain, on_pal) = (plain.voicing_onset(release), pal.voicing_onset(release));
            let (a, b) = (plain.f2(on_plain + 0.015), pal.f2(on_pal + 0.015));
            println!("{name} /{c}a/ F2 {a:.0} Hz, /{c}ʲa/ F2 {b:.0} Hz at the voice onset");
            assert!(b > a + rise, "{name} /{c}ʲa/: F2 {b:.0} Hz against {a:.0} Hz plain");
            let (a, b) = (plain.f2(on_plain + 0.2), pal.f2(on_pal + 0.2));
            assert!((a - b).abs() < 50.0, "{name} /{c}ʲa/: F2 {b:.0} Hz against {a:.0} Hz plain, 200 ms after the voice sets in");
        }
    }
}

#[test]
fn a_palatalized_stop_still_closes() {
    // The tongue body's posture is a secondary gesture: the primary articulator still seals. The
    // oral tract's narrowest area is 0 through the closure's middle, and a voiceless closure is as
    // silent as the plain one (-47 to -72 dB re the vowel in every case measured; a voiced
    // closure's voice bar varies from -19 to -62 dB with or without the mark, so it is not compared).
    for (body, name, f0) in VOICES {
        for c in ["p", "t", "k", "b", "d", "g"] {
            let r = render(&cv(palatalized(c), "a"), body, name, f0);
            let (s, vowel) = (&r.plan.segments[0], &r.plan.segments[1]);
            let (a, b) = (s.t0 + 0.2 * (s.t1 - s.t0), s.t1 - 0.03);
            let widest = r.min_area_between(a, b);
            assert!(widest < 1e-6, "/{c}ʲ/ ({name}) leaks: the narrowest oral area is {widest} cm² in the closure");
            if "ptk".contains(c) {
                let peak = |a: f64, b: f64| r.out[(a * FS) as usize..(b * FS) as usize].iter().fold(0.0f64, |m, x| m.max(x.abs() as f64));
                let db = 20.0 * (peak(a, b) / peak(vowel.t0 + 0.1, vowel.t0 + 0.3)).log10();
                println!("{name} /{c}ʲ/: closure {db:.1} dB re the vowel");
                assert!(db < -40.0, "/{c}ʲ/ ({name}): closure at {db:.1} dB re the vowel");
            }
        }
    }
}
