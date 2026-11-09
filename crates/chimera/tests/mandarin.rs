//! Mandarin: the alveolo-palatal and retroflex affricates are single tokens and real phonemes, and
//! the front end reads Han characters: longest phrase match, tone sandhi, pinyin → IPA with
//! aspiration said, tones → pitch bends.
use chimera::anatomy::human::BodyMix;
use chimera::phonetics::english::EnglishG2P;
use chimera::phonetics::inventory::{Manner, phoneme};
use chimera::phonetics::ipa::tokenize_ipa;
use chimera::phonetics::korean::Korean;
use chimera::phonetics::mandarin::{Mandarin, MandarinLexicon, tone_bends};
use chimera::phonetics::syllables::{English, read_lyrics};
use chimera::phonetics::{Edge, Syllable};

#[test]
fn affricates_tokenize_as_one_segment() {
    let ipa: Vec<String> = tokenize_ipa("tɕi tɕʰi dʑa tʂa dʐu ɕy ʑə ɥɛ").into_iter().map(|s| s.ipa).collect();
    // ʰ is not modelled and drops, as before
    assert_eq!(ipa, ["tɕ", "i", "tɕ", "i", "dʑ", "a", "tʂ", "a", "dʐ", "u", "ɕ", "y", "ʑ", "ə", "ɥ", "ɛ"]);
}

#[test]
fn mandarin_consonants_are_in_the_inventory() {
    for (ipa, manner, release) in [("ɕ", Manner::Fricative, None), ("ʑ", Manner::Fricative, None), ("tɕ", Manner::Affricate, Some("ɕ")), ("dʑ", Manner::Affricate, Some("ʑ")), ("tʂ", Manner::Affricate, Some("ʂ")), ("dʐ", Manner::Affricate, Some("ʐ")), ("ɥ", Manner::Approximant, None)] {
        let d = phoneme(ipa, BodyMix::MALE).unwrap_or_else(|| panic!("{ipa} missing"));
        assert_eq!(d.manner, manner, "{ipa}");
        assert_eq!(d.release, release, "{ipa}");
    }
}

const LEX: &str = "你\tni3\n好\thao3\n我\two3\n不\tbu4\n会\thui4\n银\tyin2\n行\txing2\n银行\tyin2 hang2\n走\tzou3\n的\tde5\n跑\tpao3\n一\tyi1\n个\tge4\n绿\tlv4\n学\txue2\n吃\tchi1\n爱\tai4\n去\tqu4\n";

fn lex() -> MandarinLexicon {
    let mut l = MandarinLexicon::new();
    l.load(LEX);
    l
}

fn syls(text: &str) -> Vec<Syllable> {
    let l = lex();
    read_lyrics(text, &[&Mandarin(&l)])
}

/// onset+nucleus+coda, with ʰ/˭ where the front end says a stop's aspiration
fn ipa(s: &Syllable) -> String {
    let m = |segs: &[chimera::phonetics::Segment]| segs.iter().map(|x| format!("{}{}", x.ipa, match x.aspirated { Some(true) => "ʰ", Some(false) => "˭", None => "" })).collect::<String>();
    format!("{}{}{}", m(&s.onset), s.nucleus.ipa, m(&s.coda))
}

#[test]
fn readings_by_longest_phrase_then_tone_sandhi() {
    let l = lex();
    assert_eq!(l.read("银行"), ["yin2", "hang2"]); // the phrase decides 行
    assert_eq!(l.read("行走"), ["xing2", "zou3"]); // no phrase: the character's own reading
    assert_eq!(l.read("你好"), ["ni2", "hao3"]); // 3-3 → 2-3
    assert_eq!(l.read("我你好"), ["wo2", "ni2", "hao3"]); // a run of third tones
    assert_eq!(l.read("不会"), ["bu2", "hui4"]); // 不 before a fourth tone
    assert_eq!(l.read("一个"), ["yi2", "ge4"]); // 一 before a fourth tone
    assert_eq!(l.read("不去"), ["bu2", "qu4"]);
}

#[test]
fn pinyin_becomes_ipa_with_aspiration_said() {
    let all: Vec<String> = syls("跑不吃学绿的爱").iter().map(ipa).collect();
    // p aspirated, b unaspirated; ch aspirated before an apical vowel; xue = ɕ ɥ ɛ; lü; neutral de; ai
    assert_eq!(all, ["pʰaʊ", "p˭u", "tʂʰɨ", "ɕɥɛ", "ly", "t˭ə", "aɪ"]);
}

#[test]
fn tones_become_bends_and_text_keeps_the_character() {
    let s = syls("你好");
    assert_eq!((s[0].text.as_str(), s[1].text.as_str()), ("你", "好"));
    assert_eq!(s[0].pitch, tone_bends(2)); // sandhi: sung as a rising tone
    assert_eq!(s[1].pitch, tone_bends(3));
    assert!(s[1].word_end && !s[0].word_end);
    assert!(tone_bends(1).is_empty() && tone_bends(5).is_empty());
    assert_eq!(tone_bends(4).iter().map(|b| b.at).collect::<Vec<_>>(), [Edge::Start, Edge::End]);
}

#[test]
fn a_mixed_line_routes_each_word_to_its_front_end() {
    let l = lex();
    let mut g = EnglishG2P::new();
    g.load_dict("you\tY UW1\n");
    let out = read_lyrics("我爱 you, 你。", &[&English(&g), &Korean, &Mandarin(&l)]);
    let shown: Vec<(&str, bool, bool)> = out.iter().map(|s| (s.text.as_str(), s.word_end, s.phrase_end)).collect();
    assert_eq!(shown, [("我", false, false), ("爱", true, false), ("you", true, true), ("你", true, true)]);
    // no lexicon loaded: Han reads as nothing, the rest is unaffected
    let empty = MandarinLexicon::new();
    assert_eq!(read_lyrics("我 you", &[&English(&g), &Mandarin(&empty)]).len(), 1);
}
