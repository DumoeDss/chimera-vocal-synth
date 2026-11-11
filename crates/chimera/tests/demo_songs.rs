//! The demo songs of Sing mode: each one's lyrics, read by the front ends the app uses, give one
//! sung syllable per note (a melisma mark is a note), and each sings through the planner and the
//! voice on the male and female bodies with every note sounding.
use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE, HumanBody};
use chimera::phonetics::english::EnglishG2P;
use chimera::phonetics::ipa_lyrics::IpaLyrics;
use chimera::phonetics::japanese::Japanese;
use chimera::phonetics::korean::Korean;
use chimera::phonetics::mandarin::{Mandarin, MandarinLexicon, Pinyin};
use chimera::phonetics::sing::{SongOptions, SungNote, plan_song};
use chimera::phonetics::songs::{DEMO_SONGS, DemoSong};
use chimera::phonetics::syllables::{English, read_lyrics};
use chimera::phonetics::Syllable;
use chimera::voice::human_voice::{HumanVoice, SourcePatch};
use std::io::Read;

fn data(name: &str) -> String {
    let path = format!("{}/../../public/data/{name}.txt.gz", env!("CARGO_MANIFEST_DIR"));
    let gz = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut text = String::new();
    flate2::read::GzDecoder::new(&gz[..]).read_to_string(&mut text).unwrap();
    text
}

/// The app's reading of a song's lyrics: every front end, CMUdict and the Mandarin lexicon loaded
fn read(text: &str) -> Vec<Syllable> {
    let mut g = EnglishG2P::new();
    g.load_dict(&data("cmudict"));
    let mut lex = MandarinLexicon::new();
    lex.load(&data("mandarin"));
    read_lyrics(text, &[&Pinyin, &English(&g), &Korean, &Japanese, &IpaLyrics, &Mandarin(&lex)])
}

fn rms(x: &[f32], fs: f64, a: f64, b: f64) -> f64 {
    let (i, j) = ((a * fs) as usize, ((b * fs) as usize).min(x.len()));
    (x[i..j].iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / (j - i).max(1) as f64).sqrt()
}

#[test]
fn every_demo_song_has_one_sung_syllable_per_note() {
    assert!(DEMO_SONGS.len() >= 5, "the catalogue lost a song");
    for s in DEMO_SONGS {
        let n = read(s.lyrics).len();
        assert_eq!(n, s.notes.len(), "{}: {n} syllables for {} notes", s.name, s.notes.len());
    }
}

/// One song through the planner and the voice on one body
fn sing(s: &DemoSong, name: &str, body: HumanBody, shift: f64, f0: f64) {
    const FS: f64 = 48000.0;
    let syl = read(s.lyrics);
    let notes: Vec<SungNote> = s.notes.iter().zip(&syl).map(|(n, syl)| SungNote { midi: n[0] + shift * 12.0, beats: n[1], syl: syl.clone() }).collect();
    let o = SongOptions { body: name.into(), bpm: s.bpm, t0: 0.35, vibrato: 0.0, ..Default::default() };
    let plan = plan_song(&notes, &o);
    assert!(plan.gestures.iter().all(|g| g.t0.is_finite() && g.t1.is_finite() && g.t0 <= g.t1 + 1e-9), "{} ({name}): a gesture is not a span", s.name);
    let mut voice = HumanVoice::new(FS, body, 2);
    voice.set_source(&SourcePatch::f0(f0));
    voice.schedule(&plan.gestures);
    let mut out = vec![0f32; (FS * (plan.end + 0.3)) as usize];
    for block in out.chunks_mut(128) {
        voice.render(block);
    }
    assert!(out.iter().all(|x| x.is_finite()), "{} ({name}): the voice went NaN", s.name);
    let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs())) as f64;
    assert!(peak > 0.05, "{} ({name}): silence (peak {peak})", s.name);
    // every note sounds: the stretch a tenth of a second after its beat is audible
    let (mut t, beat) = (0.35, 60.0 / s.bpm);
    for (k, n) in s.notes.iter().enumerate() {
        let r = rms(&out, FS, t + 0.1, t + 0.25);
        assert!(r > 0.01 * peak, "{} ({name}): note {k} is silent (RMS {r:.5}, peak {peak:.3})", s.name);
        t += n[1] * beat;
    }
}

#[test]
fn every_demo_song_sings_on_the_male_and_female_bodies() {
    // one thread per song and body: a song is half a minute of audio
    std::thread::scope(|scope| {
        let jobs: Vec<_> = DEMO_SONGS
            .iter()
            .flat_map(|s| [("male", BODY_MALE, s.octave.male, 110.0), ("female", BODY_FEMALE, s.octave.female, 220.0)].map(|(name, body, shift, f0)| scope.spawn(move || sing(s, name, body, shift, f0))))
            .collect();
        for j in jobs {
            j.join().unwrap();
        }
    });
}
