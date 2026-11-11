//! IPA lyrics: words between slashes. How they split into syllables with no onset table, the marks
//! they honour, the report of what the engine can't sing, and a phrase sung through the voice.
use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE, HumanBody};
use chimera::phonetics::english::arpabet_to_segments;
use chimera::phonetics::ipa::{ignored_ipa_marks, tokenize_ipa, unknown_ipa};
use chimera::phonetics::ipa_lyrics::{ipa_lyric_report, sung_symbols};
use chimera::phonetics::sing::{SongOptions, SungNote, plan_song};
use chimera::phonetics::syllables::lyrics_to_syllables;
use chimera::phonetics::{Plan, Segment, Syllable};
use chimera::voice::human_voice::{HumanVoice, SourcePatch};

fn word(w: &str, _reduce: bool) -> Vec<Segment> {
    match w {
        "star" => arpabet_to_segments("S T AA1 R"),
        "hello" => arpabet_to_segments("HH AH0 L OW1"),
        _ => Vec::new(),
    }
}

fn syls(text: &str) -> Vec<Syllable> {
    lyrics_to_syllables(text, &word)
}

/// Each syllable as onset.nucleus.coda, aspiration marked ʰ (the unmarked default says nothing) and
/// ˭, joined by dots between syllables and spaces between words
fn sung(text: &str) -> String {
    let marked = |segs: &[Segment]| {
        segs.iter()
            .map(|s| format!("{}{}", s.ipa, match s.aspirated { Some(true) => "ʰ", Some(false) => "˭", None => "" }))
            .collect::<Vec<_>>()
            .join("")
    };
    let mut out = String::new();
    for s in syls(text) {
        out += &format!("{}[{}]{}", marked(&s.onset), s.nucleus.ipa, marked(&s.coda));
        out += if s.word_end { " " } else { "." };
    }
    out.trim_end().to_string()
}

/// The syllables' sounds without aspiration (what the splitting decided)
fn split(text: &str) -> String {
    let flat = |segs: &[Segment]| segs.iter().map(|s| s.ipa.as_str()).collect::<String>();
    let mut out = String::new();
    for s in syls(text) {
        out += &format!("{}{}{}", flat(&s.onset), s.nucleus.ipa, flat(&s.coda));
        out += if s.word_end { " " } else { "." };
    }
    out.trim_end().to_string()
}

fn shown(text: &str) -> Vec<String> {
    syls(text).into_iter().map(|s| s.text).collect()
}

#[test]
fn syllables_split_by_sonority_in_several_languages() {
    // German (split() leaves out length): the cluster before the first vowel is its onset, a lone consonant starts the next
    assert_eq!(split("/ˈʃtʁaːsə/"), "ʃtʁa.sə");
    // Spanish: ɡɾ rises, so it goes right; s j after the vowel rises too
    assert_eq!(split("/ˈɡɾasjas/"), "gɾa.sjas");
    // Italian: two vowels are two syllables
    assert_eq!(split("/ˈtʃaːo/"), "tʃa.o");
    // clusters English's table forbids, at the start of a word
    assert_eq!(split("/ˈpsixa/"), "psi.xa");
    assert_eq!(split("/ˈmnɛ/"), "mnɛ");
    // between vowels: a rising pair goes right, a falling pair splits, a plateau splits
    assert_eq!(split("/ˈapra/"), "a.pra");
    assert_eq!(split("/ˈasta/"), "as.ta");
    assert_eq!(split("/ˈamna/"), "am.na");
    // a final cluster stays in the last coda
    assert_eq!(split("/ˈkunst/"), "kunst");
    assert_eq!(split("/ˈtʃiao/"), "tʃi.a.o");
}

#[test]
fn french_nasals_lose_their_tilde_and_say_so() {
    // the tokenizer skips the combining tilde: ɔ̃ is sung as ɔ (no nasal vowel in the inventory)
    assert_eq!(split("/bɔ̃ʒuʁ/"), "bɔ.ʒuʁ");
    assert_eq!(ipa_lyric_report("/bɔ̃ʒuʁ/").ignored_marks, ["ɔ̃"]);
    // a precomposed ã is a symbol the engine doesn't know: dropped, and reported
    assert_eq!(ipa_lyric_report("/lã/").unknown, ["ã"]);
}

#[test]
fn a_dot_forces_a_break_where_the_rule_would_not() {
    // p r rises, so the rule says a.pra; the dot puts the p in the first syllable
    assert_eq!(split("/ˈapra/"), "a.pra");
    assert_eq!(split("/ˈap.ra/"), "ap.ra");
    // s p falls, so the rule says as.pa; the dot puts the s in the second
    assert_eq!(split("/ˈaspa/"), "as.pa");
    assert_eq!(split("/ˈa.spa/"), "a.spa");
    // a dot between vowels
    assert_eq!(split("/ni.xaʊ/"), "ni.xaʊ");
    // a dot with no vowel after it joins the syllable before
    assert_eq!(split("/ˈaʊs.t/"), "aʊst");
}

#[test]
fn a_stress_mark_does_not_move_a_syllable_break() {
    // eSpeak NG writes the mark before the vowel (həlˈoʊ), the IPA convention before the syllable
    // (həˈloʊ): both split by sonority, and the mark lands on the stressed syllable's text
    assert_eq!(split("/həlˈoʊ/"), "hə.loʊ");
    assert_eq!(split("/həˈloʊ/"), "hə.loʊ");
    assert_eq!(shown("/həlˈoʊ/"), ["hə", "ˈloʊ"]);
    assert_eq!(split("/kaˈsta/"), "kas.ta");
}

#[test]
fn texts_keep_the_marks_the_engine_read() {
    assert_eq!(shown("/ˈʃtʁaːsə/"), ["ˈʃtʁaː", "sə"]);
    // tie bars go, the stress mark and length stay on the syllable, ɡ is g
    assert_eq!(shown("/ˈt͡ʃaːo/"), ["ˈtʃaː", "o"]);
    assert_eq!(shown("/ɡa/"), ["ga"]);
    // an explicit secondary stress
    assert_eq!(shown("/ˌaˈka/"), ["ˌa", "ˈka"]);
}

#[test]
fn aspiration_marks_say_it_and_unmarked_stops_are_unaspirated() {
    assert_eq!(sung("/pa/"), "p˭[a]");
    assert_eq!(sung("/pʰa/"), "pʰ[a]");
    assert_eq!(sung("/tʃa/"), "tʃ˭[a]");
    assert_eq!(sung("/ta˭/"), "t˭[a]");
    // voiced stops, fricatives and the glottal stop say nothing
    assert_eq!(sung("/baʔ/"), "b[a]ʔ");
    assert_eq!(sung("/sa/"), "s[a]");
    // and the mark shows in the note's text
    assert_eq!(shown("/pʰa pa/"), ["pʰa", "pa"]);
}

#[test]
fn stress_marks_and_length_reach_the_segments() {
    let s = syls("/ˈʃtʁaːsə/");
    assert_eq!(s[0].nucleus.stress, Some(1));
    assert_eq!(s[1].nucleus.stress, Some(0));
    assert!(s[0].nucleus.dur.is_some(), "length ː is kept");
    // a diphthong is one nucleus
    assert_eq!(split("/ˈlaɪt/"), "laɪt");
}

#[test]
fn a_word_with_no_vowel_is_dropped_and_reported() {
    assert!(syls("/ʃt/").is_empty());
    let r = ipa_lyric_report("/ʃt/ /ma/ //");
    assert_eq!(r.no_vowel, ["/ʃt/", "//"]);
    assert_eq!(split("/ʃt/ /ma/"), "ma");
}

#[test]
fn symbols_the_engine_does_not_know_are_reported_not_dropped_quietly() {
    assert_eq!(unknown_ipa("ˈʃøːn"), Vec::<String>::new());
    assert_eq!(unknown_ipa("Ŋa ã ǀa Ŋ"), ["Ŋ", "ã", "ǀ"]);
    assert_eq!(ignored_ipa_marks("n̥ ɔ̃ n̥"), ["n̥", "ɔ̃"]);
    // tokenize_ipa itself is unchanged: the unknown ones are still dropped
    assert_eq!(tokenize_ipa("Ŋa").len(), 1);
    // ʏ is a letter the tokenizer reads, but the planner has no vowel for it: reported, not sung
    let r = ipa_lyric_report("/ˈfʏʁ/ /ʃøːn/ /ˈŊa/");
    assert_eq!((r.no_sound, r.unknown), (vec!["ʏ".to_string()], vec!["Ŋ".to_string()]));
    let r = ipa_lyric_report("/ˈfʏʁ/ /ˈŊa/");
    assert!(!r.is_empty());
    let w = r.warnings();
    assert!(w.iter().any(|l| l.contains("no sound") && l.contains('ʏ')), "{w:?}");
    // a clean lyric reports nothing, and lyrics with no IPA words have nothing to check
    assert!(ipa_lyric_report("/ˈʃtʁaːsə/ hello さくら").is_empty());
    assert!(ipa_lyric_report("hello").is_empty());
    // the symbols a tool may write are the ones sung
    let sung = sung_symbols();
    assert!(sung.contains(&"ʃ") && sung.contains(&"ø") && sung.contains(&"ɔɪ"));
    assert!(!sung.contains(&"ʏ"));
}

#[test]
fn ipa_words_mix_with_the_other_front_ends_keeping_word_and_phrase_ends() {
    let s = syls("さくら /ʃøːn/, hello /ˈʃtʁaːsə/ 빵야.");
    let got: Vec<(&str, bool, bool)> = s.iter().map(|s| (s.text.as_str(), s.word_end, s.phrase_end)).collect();
    assert_eq!(
        got,
        [
            ("さ", false, false),
            ("く", false, false),
            ("ら", true, false),
            ("ʃøːn", true, true),
            ("hel", false, false),
            ("lo", true, false),
            ("ˈʃtʁaː", false, false),
            ("sə", true, false),
            ("빵", false, false),
            ("야", true, true),
        ]
    );
    // no space needed between an IPA word and the next script
    assert_eq!(shown("star/ma/さ"), ["star", "ma", "さ"]);
}

#[test]
fn spaces_inside_slashes_separate_words_and_an_unclosed_slash_is_dropped() {
    let s = syls("/ˈhɛ loʊ/");
    assert_eq!(s.iter().map(|s| s.word_end).collect::<Vec<_>>(), [true, true]);
    // an unclosed slash is skipped and the rest reads as ordinary lyrics
    assert_eq!(shown("hello /"), ["hel", "lo"]);
    assert_eq!(shown("/ˈka\nma/"), Vec::<String>::new().into_iter().chain(shown("ka\nma")).collect::<Vec<_>>());
    // a slash that would close on the next line doesn't: the line break is a phrase end
    assert_eq!(shown("/ˈka\n/ma"), shown("ka\nma"));
    // melisma marks and punctuation work around IPA words
    let m = syls("/ma/ - /na/,");
    assert_eq!(m[1].melisma, Some(true));
    assert!(m[2].phrase_end);
}

#[test]
fn ipa_syllables_round_trip_through_json() {
    let s = syls("/ˈʃtʁaːsə/ /pʰa/");
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(s, serde_json::from_str::<Vec<Syllable>>(&json).unwrap());
}

/// RMS of `x` between two times (s)
fn rms(x: &[f32], fs: f64, a: f64, b: f64) -> f64 {
    let (i, j) = ((a * fs) as usize, ((b * fs) as usize).min(x.len()));
    (x[i..j].iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / (j - i).max(1) as f64).sqrt()
}

fn sing_it(text: &str, name: &str, body: HumanBody, f0: f64) -> (Vec<f32>, Plan, Vec<f64>) {
    const FS: f64 = 48000.0;
    let notes: Vec<SungNote> = syls(text).into_iter().enumerate().map(|(i, syl)| SungNote { midi: [60.0, 62.0, 64.0, 62.0][i % 4], beats: 1.0, syl }).collect();
    let o = SongOptions { body: name.into(), bpm: 100.0, t0: 0.4, vibrato: 0.0, ..Default::default() };
    let plan = plan_song(&notes, &o);
    let mut voice = HumanVoice::new(FS, body, 2);
    voice.set_source(&SourcePatch::f0(f0));
    voice.schedule(&plan.gestures);
    let mut out = vec![0f32; (FS * (plan.end + 0.3)) as usize];
    for block in out.chunks_mut(128) {
        voice.render(block);
    }
    let beats: Vec<f64> = (0..notes.len()).map(|i| 0.4 + i as f64 * 0.6).collect();
    (out, plan, beats)
}

#[test]
fn an_ipa_phrase_sings_through_the_planner_and_the_voice() {
    // German, French, Spanish, Italian, a Greek cluster, an aspirated stop and a long vowel
    let text = "/ˈʃtʁaːsə/ /ˈbɔ̃ʒuʁ/ /ˈɡɾasjas/ /ˈtʃaːo/ /ˈpsixa/ /ˈpʰaːta/ /ʃøːn/";
    for (name, body, f0) in [("male", BODY_MALE, 110.0), ("female", BODY_FEMALE, 200.0)] {
        let (out, plan, beats) = sing_it(text, name, body, f0);
        assert!(out.iter().all(|x| x.is_finite()), "{name}: the voice went NaN");
        assert!(plan.gestures.iter().all(|g| g.t0.is_finite() && g.t1.is_finite() && g.t0 <= g.t1 + 1e-9), "{name}: a gesture is not a span");
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs())) as f64;
        assert!(peak > 0.05, "{name}: silence (peak {peak})");
        let mut quietest = f64::MAX;
        for (k, &t) in beats.iter().enumerate() {
            let r = rms(&out, 48000.0, t + 0.1, t + 0.3);
            quietest = quietest.min(r / peak);
            assert!(r > 0.01 * peak, "{name}: note {k} is silent (RMS {r:.5}, peak {peak:.3})");
        }
        println!("{name}: {} notes, peak {peak:.3}, quietest note RMS {:.3} of peak", beats.len(), quietest);
    }
}
