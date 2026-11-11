//! Japanese: kana, one mora per note. The sounds each kana gets, ん and っ joining the note before,
//! long vowels as melisma, and a kana phrase sung through the planner and the voice.
use chimera::anatomy::human::{BODY_FEMALE, BODY_MALE, HumanBody};
use chimera::phonetics::english::arpabet_to_segments;
use chimera::phonetics::japanese::Japanese;
use chimera::phonetics::sing::{SongOptions, SungNote, plan_song};
use chimera::phonetics::syllables::{lyrics_to_syllables, read_lyrics};
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

/// Each syllable as its sounds, aspiration marked ʰ, a melisma note as ~ and its vowel, joined by dots
fn sung(text: &str) -> String {
    let marked = |segs: &[Segment]| segs.iter().map(|s| format!("{}{}", s.ipa, if s.aspirated == Some(true) { "ʰ" } else { "" })).collect::<String>();
    let notes: Vec<String> = syls(text)
        .iter()
        .map(|s| if s.melisma == Some(true) { format!("~{}", s.nucleus.ipa) } else { format!("{}{}{}", marked(&s.onset), s.nucleus.ipa, marked(&s.coda)) })
        .collect();
    notes.join(".")
}

/// The text shown on each note
fn shown(text: &str) -> Vec<String> {
    syls(text).into_iter().map(|s| s.text).collect()
}

#[test]
fn the_basic_table_and_the_irregular_kana() {
    assert_eq!(sung("さくら"), "sa.kʰɯ.ɾa");
    assert_eq!(sung("かえる"), "kʰa.e.ɾɯ");
    assert_eq!(sung("あいうえお"), "a.i.ɯ.e.o");
    assert_eq!(sung("なにぬねの"), "na.ɲi.nɯ.ne.no");
    assert_eq!(sung("まみむめも らりるれろ"), "ma.mi.mɯ.me.mo.ɾa.ɾi.ɾɯ.ɾe.ɾo");
    assert_eq!(sung("やゆよ わを"), "ja.jɯ.jo.wa.o");
    // し ち つ ふ ひ and the rows around them
    assert_eq!(sung("さしすせそ"), "sa.ɕi.sɯ.se.so");
    assert_eq!(sung("たちつてと"), "tʰa.tɕʰi.tsʰɯ.tʰe.tʰo");
    assert_eq!(sung("はひふへほ"), "ha.çi.ɸɯ.he.ho");
    // voiced, and the じ/ぢ and ず/づ pairs
    assert_eq!(sung("ざじずぜぞ"), "za.dʑi.zɯ.ze.zo");
    assert_eq!(sung("だぢづでど"), "da.dʑi.zɯ.de.do");
    assert_eq!(sung("がぎぐげご ばびぶべぼ"), "ga.gi.gɯ.ge.go.ba.bi.bɯ.be.bo");
    // the p row
    assert_eq!(sung("ぱぴぷぺぽ"), "pʰa.pʰi.pʰɯ.pʰe.pʰo");
    // the text is the kana as typed
    assert_eq!(shown("さくら"), ["さ", "く", "ら"]);
}

#[test]
fn voiceless_stops_say_one_weak_aspiration_everywhere() {
    let s = syls("かきゃたちつぱ");
    // each is aspirated, with a short release (shorter than English's 50 ms), even before a glide
    let vot: Vec<Option<f64>> = s.iter().map(|x| x.onset[0].vot).collect();
    assert_eq!(vot, [Some(0.04), Some(0.04), Some(0.03), Some(0.03), Some(0.03), Some(0.03)]);
    assert!(s.iter().all(|x| x.onset[0].aspirated == Some(true)));
    // voiced and fricative consonants say nothing
    assert_eq!(syls("が")[0].onset[0].aspirated, None);
    assert_eq!(syls("さ")[0].onset[0].aspirated, None);
}

#[test]
fn yoon_and_the_loanword_kana() {
    assert_eq!(sung("きょう"), "kʰjo.~o");
    assert_eq!(shown("きょう"), ["きょ", "う"]);
    assert_eq!(sung("しゃしん"), "ɕa.ɕiɴ");
    assert_eq!(sung("りょこう"), "ɾjo.kʰo.~o");
    assert_eq!(sung("ちゃ じゃ にゃ ひゃ みゃ ぎゅ びょ ぴゅ"), "tɕʰa.dʑa.ɲa.ça.mja.gjɯ.bjo.pʰjɯ");
    // small kana for loanwords
    assert_eq!(sung("ふぁ ふぃ ふぇ ふぉ"), "ɸa.ɸi.ɸe.ɸo");
    assert_eq!(sung("てぃ でぃ とぅ どぅ"), "tʰi.di.tʰɯ.dɯ");
    assert_eq!(sung("うぃ うぇ うぉ"), "wi.we.wo");
    assert_eq!(sung("ちぇ しぇ じぇ"), "tɕʰe.ɕe.dʑe");
    assert_eq!(sung("ゔぁ ゔぃ ゔ ゔぇ ゔぉ"), "va.vi.vɯ.ve.vo");
    assert_eq!(sung("つぁ くぁ いぇ でゅ ふゅ"), "tsʰa.kʰwa.je.djɯ.ɸjɯ");
    // one note for the kana and its small kana
    assert_eq!(shown("ファイト"), ["ファ", "イ", "ト"]);
    assert_eq!(sung("ファイト"), "ɸa.i.tʰo");
}

#[test]
fn n_takes_the_place_of_what_follows() {
    assert_eq!(sung("しんぶん"), "ɕim.bɯɴ"); // before b, and at the end of a word
    assert_eq!(sung("さんぽ"), "sam.pʰo");
    assert_eq!(sung("もんもん"), "mom.moɴ");
    assert_eq!(sung("ぎんこう"), "giŋ.kʰo.~o");
    assert_eq!(sung("まんが"), "maŋ.ga");
    assert_eq!(sung("ほん"), "hoɴ");
    assert_eq!(sung("こんや"), "kʰoɴ.ja"); // before a semivowel
    assert_eq!(sung("れんあい"), "ɾeɴ.a.i"); // before a vowel
    assert_eq!(sung("せんせい"), "seɴ.se.~e"); // before a fricative
    assert_eq!(sung("こんにちは"), "kʰon.ɲi.tɕʰi.ha"); // before ɲ
    assert_eq!(sung("かんたん"), "kʰan.tʰaɴ");
    assert_eq!(sung("あんざい"), "an.za.i");
    assert_eq!(sung("ごりらんど"), "go.ɾi.ɾan.do");
    assert_eq!(sung("さんま"), "sam.ma");
    // the nasal is the note's coda, held a mora long; the text shows both kana
    let s = syls("かん");
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].text, "かん");
    assert_eq!(s[0].coda[0].dur, Some(0.12));
}

#[test]
fn a_ん_with_no_note_to_join_is_its_own_note() {
    // at the start of a word: the nasal and ɯ, in the place of what follows
    assert_eq!(sung("んば"), "mɯ.ba");
    assert_eq!(shown("んば"), ["ん", "ば"]);
    // after another ん
    assert_eq!(sung("あんん"), "aɴ.ɴɯ");
    // ん then a long mark holds that vowel
    assert_eq!(sung("んー"), "ɴɯ.~ɯ");
}

#[test]
fn small_tsu_lengthens_the_next_consonant() {
    // がっこう: the k is held about twice as long as a single one, and there is no extra note or stop
    assert_eq!(shown("がっこう"), ["がっ", "こ", "う"]);
    assert_eq!(sung("がっこう"), "ga.kʰo.~o");
    let long = syls("がっこう")[1].onset[0].dur.unwrap();
    assert!(syls("がこう")[1].onset[0].dur.is_none());
    let single = 0.08 * 1.25; // /k/'s spoken length times the sung factor
    assert!((long / single - 2.0).abs() < 1e-9, "geminate {long} vs single {single}");
    // きって, a fricative and an affricate, and a sound that cannot be held
    assert_eq!(shown("きって"), ["きっ", "て"]);
    assert_eq!(sung("きって"), "kʰi.tʰe");
    assert!(syls("きって")[1].onset[0].dur.is_some());
    assert!(syls("ざっし")[1].onset[0].dur.is_some());
    assert!(syls("まっちゃ")[1].onset[0].dur.is_some());
    // at the end of a word: a glottal stop coda, text joined
    assert_eq!(sung("あっ"), "aʔ");
    assert_eq!(shown("あっ"), ["あっ"]);
    // before a vowel or a glide: the same
    assert_eq!(sung("あっあ"), "aʔ.a");
    assert_eq!(sung("あっや"), "aʔ.ja");
    // at the start of a word there is nothing to join
    assert_eq!(sung("っか"), "kʰa");
}

#[test]
fn long_vowels_continue_the_vowel_as_melisma() {
    assert_eq!(sung("コーヒー"), "kʰo.~o.çi.~i");
    assert_eq!(shown("コーヒー"), ["コ", "ー", "ヒ", "ー"]);
    assert_eq!(sung("おかあさん"), "o.kʰa.~a.saɴ");
    assert_eq!(shown("おかあさん"), ["お", "か", "あ", "さん"]);
    assert_eq!(sung("とうきょう"), "tʰo.~o.kʰjo.~o"); // おう is oo
    assert_eq!(sung("せんせい"), "seɴ.se.~e"); // えい is ee
    assert_eq!(sung("おおきい"), "o.~o.kʰi.~i");
    assert_eq!(sung("ゆうき"), "jɯ.~ɯ.kʰi");
    // a different vowel is a note of its own
    assert_eq!(sung("かい"), "kʰa.i");
    assert_eq!(sung("とおい"), "tʰo.~o.i");
    // a ん after the long mark sings after the chain: the coda is on the chain's first note (where the
    // planner reads it), the kana is shown on the last
    let s = syls("ほーん");
    assert_eq!((s.len(), s[0].coda.len(), s[1].coda.len(), s[1].text.as_str()), (2, 1, 0, "ーん"));
    // every melisma note is a continuation of the vowel before it
    for s in syls("コーヒー おかあさん").iter().filter(|s| s.melisma == Some(true)) {
        assert!(s.onset.is_empty() && s.coda.is_empty());
    }
}

#[test]
fn katakana_and_half_width_katakana_read_like_hiragana() {
    assert_eq!(sung("コーヒー"), sung("こーひー"));
    assert_eq!(sung("ヴァイオリン"), "va.i.o.ɾiɴ");
    assert_eq!(sung("ガッコウ"), sung("がっこう"));
    // half-width: voicing marks join the kana before them and stay in its text
    assert_eq!(sung("ｶﾞｯｺｳ"), "ga.kʰo.~o");
    assert_eq!(shown("ｶﾞｯｺｳ"), ["ｶﾞｯ", "ｺ", "ｳ"]);
    assert_eq!(sung("ﾊﾟﾝ ｺｰﾋｰ"), "pʰaɴ.kʰo.~o.çi.~i");
    // a combining mark does too (text typed as か + U+3099)
    assert_eq!(sung("か\u{3099}"), "ga");
    // iteration marks repeat the kana before them
    assert_eq!(sung("ここゝ"), "kʰo.kʰo.kʰo");
    assert_eq!(sung("はゞ"), "ha.ba");
}

#[test]
fn particles_read_as_written() {
    assert_eq!(sung("わたしは"), "wa.tʰa.ɕi.ha");
    assert_eq!(sung("へや"), "he.ja");
    assert_eq!(sung("わたしわ"), "wa.tʰa.ɕi.wa");
}

#[test]
fn kana_mix_with_english_and_hangul_keeping_word_and_phrase_ends() {
    let s = syls("star さくら、빵야 hello");
    let got: Vec<(&str, bool, bool)> = s.iter().map(|s| (s.text.as_str(), s.word_end, s.phrase_end)).collect();
    assert_eq!(
        got,
        [("star", true, false), ("さ", false, false), ("く", false, false), ("ら", true, true), ("빵", false, false), ("야", true, false), ("hel", false, false), ("lo", true, false)]
    );
    // kana and English next to each other, with no space, are two words
    let t: Vec<(String, bool)> = syls("starさく").into_iter().map(|s| (s.text, s.word_end)).collect();
    assert_eq!(t, [("star".to_string(), true), ("さ".to_string(), false), ("く".to_string(), true)]);
    // Han stays with Mandarin: with no Mandarin front end loaded it reads as nothing
    assert_eq!(sung("さ私く"), "sa.kʰɯ");
    assert_eq!(read_lyrics("日本", &[&Japanese]).len(), 0);
    // ・ separates words
    let n: Vec<bool> = syls("コーヒー・ブレイク").iter().map(|s| s.word_end).collect();
    assert_eq!(n.iter().filter(|w| **w).count(), 2);
}

#[test]
fn full_width_punctuation_ends_phrases() {
    let phrases = |t: &str| syls(t).iter().filter(|s| s.phrase_end).count();
    assert_eq!(phrases("さくら、さくら。"), 2);
    assert_eq!(phrases("やよいの！そらは？ みわたす；"), 3);
    // the break lands on the last note of the word before it
    let s = syls("かん、あっ。");
    assert!(s[0].phrase_end && s[1].phrase_end && s[1].coda[0].ipa == "ʔ");
    assert_eq!(s[1].text, "あっ");
}

#[test]
fn syllables_round_trip_through_json_with_the_format_the_back_end_reads() {
    let s = syls("きょう がっ");
    let json = serde_json::to_string(&s).unwrap();
    let back: Vec<Syllable> = serde_json::from_str(&json).unwrap();
    assert_eq!(s, back);
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
fn a_kana_phrase_sings_through_the_planner_and_the_voice() {
    // every kind of mora: plain, yōon, ん coda, っ before a stop, glottal stop, long vowel, loanword
    let text = "さくら さくら やよいの そらは んー がっこう きょう しゃしん ふぁいと こんにちは あっ コーヒー";
    for (name, body, f0) in [("male", BODY_MALE, 110.0), ("female", BODY_FEMALE, 200.0)] {
        let (out, plan, beats) = sing_it(text, name, body, f0);
        assert!(out.iter().all(|x| x.is_finite()), "{name}: the voice went NaN");
        assert!(plan.gestures.iter().all(|g| g.t0.is_finite() && g.t1.is_finite() && g.t0 <= g.t1 + 1e-9), "{name}: a gesture is not a span");
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs())) as f64;
        assert!(peak > 0.05, "{name}: silence (peak {peak})");
        // every note sounds: its vowel, a tenth of a second after the beat, is audible
        let mut quietest = f64::MAX;
        for (k, &t) in beats.iter().enumerate() {
            let r = rms(&out, 48000.0, t + 0.1, t + 0.3);
            quietest = quietest.min(r / peak);
            assert!(r > 0.01 * peak, "{name}: note {k} is silent (RMS {r:.5}, peak {peak:.3})");
        }
        println!("{name}: peak {peak:.3}, quietest note RMS {:.3} of peak", quietest);
    }
}
