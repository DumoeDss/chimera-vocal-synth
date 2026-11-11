//! The sound marks, heard: each is rendered through the real voice (male and female bodies) and
//! measured against the same sound without the mark.
use chimera::analysis::fft::{Fft, hann, power_spectrum_db};
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
}

fn render(segs: &[Segment], body: HumanBody, name: &str, f0: f64) -> Rendered {
    let mut v = HumanVoice::new(FS, body, 2);
    v.set_source(&SourcePatch::f0(f0));
    let plan = plan_utterance(segs, &PlanOptions { body: name.into(), t0: 0.05, f0: Some(f0), ..Default::default() });
    v.schedule(&plan.gestures);
    let mut out = vec![0.0f32; (FS * (plan.end + 0.3)).round() as usize];
    let mut port = vec![];
    for c in out.chunks_mut(128) {
        v.render(c);
        port.push(v.airway.frame.as_ref().map_or(0.0, |f| f.velum_port));
    }
    Rendered { plan, out, port }
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
