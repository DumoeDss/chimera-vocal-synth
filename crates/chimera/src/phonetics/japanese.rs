//! Kana lyrics → sung syllables. One mora is one note, the convention of Japanese singing editors:
//! each kana (or a kana with a small ゃゅょ or small vowel) is one syllable whose text is the kana
//! as typed. A vowel is one of あ a, い i, う ɯ, え e, お o.
//!
//! The script this front end claims is hiragana, katakana (with ー) and half-width katakana.
//! **Han characters stay with the Mandarin front end** (docs/language-front-ends.md: the open
//! question of a second Han front end), so Japanese lyrics are typed in kana, as in most singing
//! editors. There is no kanji dictionary.
//!
//! What the front end decides:
//! - Consonants: し ɕ, ち tɕ, つ ts, ふ ɸ, ひ ç, じ/ぢ dʑ, ず/づ z, に ɲ, ら行 ɾ, わ w, や行 j. Yōon
//!   (きゃ = k j a; しゃ ɕa, ちゃ tɕa, じゃ dʑa, にゃ ɲa, ひゃ ça, which already carry their palatal)
//!   and the small-kana loanword sounds (ふぁ ɸa, てぃ ti, うぃ wi, ちぇ tɕe, ヴ v…).
//! - Aspiration: Japanese voiceless stops are weakly aspirated and do not contrast with an
//!   unaspirated kind, so every one says the same thing: aspirated, with a short release (30 to
//!   40 ms, between English's 50 and Mandarin's unaspirated 12). Said explicitly, because English's
//!   positional rule only aspirates a stop right before a vowel, which would make the k of きゃ
//!   (k j a) differ from the k of か for no reason.
//! - ん is the previous note's coda, assimilated to what follows: m before p b m, n before t d n
//!   ts z tɕ dʑ ɾ, ŋ before k g, ɴ at the end of a word and before a vowel, semivowel or
//!   fricative. A ん with no note to join (it starts a word, or follows another ん) is sung as its
//!   own note: the nasal and ɯ (the engine has no syllabic nasal).
//! - っ lengthens the next consonant, through the per-consonant sung `dur`: the closure of a stop,
//!   the frication of a fricative or affricate. It is not a second released stop. Before a vowel, a
//!   semivowel, ɾ, or at the end of a word it is a glottal stop coda. Its kana joins the previous
//!   note's text. At the start of a word there is no note to join and it is dropped.
//! - Long vowels sing as melisma notes that continue the previous vowel, still one note per kana:
//!   ー; a vowel kana that repeats the vowel of the mora before it in the word (おお, ああ, きい,
//!   ゆう); and the spellings おう → oo and えい → ee. The simple rule is wrong when the う or い is
//!   a real syllable of its own (the う of 思う, written おもう, sings as a continued o). A lyric
//!   that needs the other reading can be frozen to syllables and edited.
//! - は and へ are read as written (ha, he); type わ and え for the particle readings. を is o.
//!
//! No pitch accent and no vowel devoicing: a sung note carries its own pitch, and a devoiced
//! vowel would be a silent note. No pitch bends.

use super::inventory::phoneme;
use super::sing::SUNG_CONSONANTS;
use super::syllables::FrontEnd;
use super::{Segment, Syllable};
use crate::anatomy::human::BodyMix;

const VOWELS: [&str; 5] = ["a", "i", "ɯ", "e", "o"];

/// Kana this front end reads: hiragana, katakana and half-width katakana letters.
pub fn is_kana(c: char) -> bool {
    matches!(c, '\u{3041}'..='\u{3096}' | '\u{30A1}'..='\u{30FA}' | '\u{FF66}'..='\u{FF9D}')
}

/// Marks that only continue a word: ー, the voicing marks, and the iteration marks ゝ ゞ ヽ ヾ.
fn is_mark(c: char) -> bool {
    matches!(c, '\u{30FC}' | '\u{FF70}' | '\u{3099}'..='\u{309C}' | '\u{309D}' | '\u{309E}' | '\u{30FD}' | '\u{30FE}' | '\u{FF9E}' | '\u{FF9F}')
}

/// Half-width katakana U+FF66.. as full-width.
const HALFWIDTH: &str = "ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン";

/// A kana's voiced form (か → が, う → ゔ), or None.
fn voiced(c: char) -> Option<char> {
    let u = c as u32;
    let v = match u {
        0x304B..=0x3062 if (u - 0x304B) % 2 == 0 => u + 1,
        0x3064..=0x3069 if (u - 0x3064) % 2 == 0 => u + 1,
        0x306F..=0x307D if (u - 0x306F) % 3 == 0 => u + 1,
        0x3046 => 0x3094,
        _ => return None,
    };
    char::from_u32(v)
}

/// A kana's half-voiced form (は → ぱ), or None.
fn semivoiced(c: char) -> Option<char> {
    let u = c as u32;
    if (0x306F..=0x307D).contains(&u) && (u - 0x306F) % 3 == 0 { char::from_u32(u + 2) } else { None }
}

/// One kana as hiragana, with the characters it was typed as.
type Kana = (char, String);

/// The word as hiragana: katakana and half-width folded, voicing marks and iteration marks applied.
fn normalise(word: &str) -> Vec<Kana> {
    let mut out: Vec<Kana> = Vec::new();
    for ch in word.chars() {
        let full = match ch {
            '\u{FF66}'..='\u{FF9D}' => HALFWIDTH.chars().nth(ch as usize - 0xFF66).unwrap_or(ch),
            other => other,
        };
        let u = full as u32;
        match ch {
            '\u{3099}' | '\u{309B}' | '\u{FF9E}' | '\u{309A}' | '\u{309C}' | '\u{FF9F}' => {
                if let Some(last) = out.last_mut() {
                    let semi = matches!(ch, '\u{309A}' | '\u{309C}' | '\u{FF9F}');
                    if let Some(v) = if semi { semivoiced(last.0) } else { voiced(last.0) } {
                        last.0 = v;
                    }
                    last.1.push(ch);
                }
            }
            '\u{309D}' | '\u{30FD}' | '\u{309E}' | '\u{30FE}' => {
                if let Some(prev) = out.last().map(|k| k.0).filter(|&c| base(c).is_some()) {
                    let c = if matches!(ch, '\u{309E}' | '\u{30FE}') { voiced(prev).unwrap_or(prev) } else { prev };
                    out.push((c, ch.to_string()));
                }
            }
            // ヷ ヸ ヹ ヺ: v + a small vowel
            '\u{30F7}'..='\u{30FA}' => {
                out.push(('ゔ', ch.to_string()));
                out.push((['ぁ', 'ぃ', 'ぇ', 'ぉ'][(u - 0x30F7) as usize], String::new()));
            }
            _ => {
                let hira = if (0x30A1..=0x30F6).contains(&u) { char::from_u32(u - 0x60).unwrap_or(full) } else { full };
                out.push((hira, ch.to_string()));
            }
        }
    }
    out
}

/// Consonant rows: the five kana of a row (a i u e o) and each one's consonant.
const ROWS: [(&str, [&str; 5]); 12] = [
    ("かきくけこ", ["k"; 5]),
    ("がぎぐげご", ["g"; 5]),
    ("さしすせそ", ["s", "ɕ", "s", "s", "s"]),
    ("ざじずぜぞ", ["z", "dʑ", "z", "z", "z"]),
    ("たちつてと", ["t", "tɕ", "ts", "t", "t"]),
    ("だぢづでど", ["d", "dʑ", "z", "d", "d"]),
    ("なにぬねの", ["n", "ɲ", "n", "n", "n"]),
    ("はひふへほ", ["h", "ç", "ɸ", "h", "h"]),
    ("ばびぶべぼ", ["b"; 5]),
    ("ぱぴぷぺぽ", ["p"; 5]),
    ("まみむめも", ["m"; 5]),
    ("らりるれろ", ["ɾ"; 5]),
];

/// A base kana's consonant ("" for none) and vowel.
fn base(c: char) -> Option<(&'static str, &'static str)> {
    for (kana, cons) in &ROWS {
        if let Some(i) = kana.chars().position(|k| k == c) {
            return Some((cons[i], VOWELS[i]));
        }
    }
    Some(match c {
        'あ' => ("", "a"),
        'い' | 'ゐ' => ("", "i"),
        'う' => ("", "ɯ"),
        'え' | 'ゑ' => ("", "e"),
        'お' | 'を' => ("", "o"),
        'や' => ("j", "a"),
        'ゆ' => ("j", "ɯ"),
        'よ' => ("j", "o"),
        'わ' | 'ゎ' => ("w", "a"),
        'ゔ' => ("v", "ɯ"),
        'ゕ' => ("k", "a"),
        'ゖ' => ("k", "e"),
        _ => return None,
    })
}

fn small_vowel(c: char) -> Option<usize> {
    "ぁぃぅぇぉ".chars().position(|k| k == c)
}

fn small_y(c: char) -> Option<&'static str> {
    Some(match c {
        'ゃ' => "a",
        'ゅ' => "ɯ",
        'ょ' => "o",
        _ => return None,
    })
}

/// Consonants that already carry their palatalisation: a yōon after them has no j of its own.
fn is_palatal(c: &str) -> bool {
    matches!(c, "ɕ" | "tɕ" | "dʑ" | "ɲ" | "ç")
}

/// A mora, before notes are made of them.
enum Mora {
    /// onset consonants (IPA), the vowel, the kana typed, and which of あいうえお it is (a vowel kana
    /// may continue the vowel before it)
    Syl { onset: Vec<&'static str>, vowel: &'static str, src: String, plain: Option<usize> },
    /// ん
    Nasal(String),
    /// っ
    Sokuon(String),
    /// ー
    Long(String),
}

fn moras(kana: &[Kana]) -> Vec<Mora> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < kana.len() {
        let (c, src) = &kana[i];
        let mut src = src.clone();
        i += 1;
        match c {
            'ん' => out.push(Mora::Nasal(src)),
            'っ' => out.push(Mora::Sokuon(src)),
            'ー' => out.push(Mora::Long(src)),
            // small kana that follow no kana of their own are read like the full-size ones
            'ゃ' | 'ゅ' | 'ょ' => out.push(Mora::Syl { onset: vec!["j"], vowel: small_y(*c).unwrap(), src, plain: None }),
            'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' => {
                let k = small_vowel(*c).unwrap();
                out.push(Mora::Syl { onset: vec![], vowel: VOWELS[k], src, plain: Some(k) });
            }
            _ => {
                let Some((cons, vowel)) = base(*c) else { continue };
                let mut onset: Vec<&'static str> = if cons.is_empty() { vec![] } else { vec![cons] };
                let mut vowel = vowel;
                let plain = (cons.is_empty() && *c != 'を').then(|| VOWELS.iter().position(|v| *v == vowel).unwrap());
                if let Some((next, next_src)) = kana.get(i) {
                    if let Some(k) = small_vowel(*next) {
                        // ふぁ ɸa, てぃ ti, ちぇ tɕe, うぃ wi, くぁ kwa, いぇ je; あぁ is a lengthened あ
                        let joined = match c {
                            'う' if k != 2 => Some(vec!["w"]),
                            'い' if k == 3 => Some(vec!["j"]),
                            'く' | 'ぐ' => Some(vec![cons, "w"]),
                            'あ' | 'い' | 'う' | 'え' | 'お' | 'を' | 'ゐ' | 'ゑ' => None,
                            _ if cons.is_empty() => None,
                            _ => Some(onset.clone()),
                        };
                        if let Some(o) = joined {
                            onset = o;
                            vowel = VOWELS[k];
                            src.push_str(next_src);
                            i += 1;
                        }
                    } else if let Some(v) = small_y(*next) {
                        // きゃ k j a; しゃ ɕa carries its own palatal
                        let joined = match cons {
                            "" if *c == 'い' => Some(vec!["j"]),
                            "" | "j" => None,
                            c if is_palatal(c) => Some(vec![c]),
                            c => Some(vec![c, "j"]),
                        };
                        if let Some(o) = joined {
                            onset = o;
                            vowel = v;
                            src.push_str(next_src);
                            i += 1;
                        }
                    }
                }
                let plain = plain.filter(|_| onset.is_empty());
                out.push(Mora::Syl { onset, vowel, src, plain });
            }
        }
    }
    out
}

/// How long a geminate consonant is sung, relative to a single one sung by the planner
const GEMINATE: f64 = 2.0;
/// A moraic nasal is a whole mora: sung about as long as Mandarin's held -ng (s)
const NASAL_CODA: f64 = 0.12;

/// A consonant as the front end says it: voiceless stops and affricates weakly aspirated.
fn consonant(ipa: &str) -> Segment {
    let vot = match ipa {
        "k" => Some(0.04),
        "p" | "t" | "ts" | "tɕ" => Some(0.03),
        _ => None,
    };
    Segment { aspirated: vot.map(|_| true), vot, ..Segment::new(ipa) }
}

/// The nasal a ん sings before this mora.
fn nasal_before(next: Option<&Mora>) -> &'static str {
    let Some(Mora::Syl { onset, .. }) = next else { return "ɴ" };
    match onset.first().copied().unwrap_or("") {
        "p" | "b" | "m" => "m",
        // ざ行 after ん is [dz]
        "t" | "d" | "n" | "ts" | "z" | "tɕ" | "dʑ" | "ɾ" | "ɲ" => "n",
        "k" | "g" => "ŋ",
        _ => "ɴ",
    }
}

/// Whether the consonant after a っ can be held (a stop, fricative or affricate; not a glide or tap).
fn can_geminate(next: Option<&Mora>) -> bool {
    matches!(next, Some(Mora::Syl { onset, .. }) if onset.first().is_some_and(|c| !matches!(*c, "j" | "w" | "ɾ")))
}

/// One word of kana → its sung syllables, the last marked as the word's end.
pub fn word_syllables(word: &str) -> Vec<Syllable> {
    let ms = moras(&normalise(word));
    let mut out: Vec<Syllable> = Vec::with_capacity(ms.len());
    // the chain's first note (a coda is sung after the chain, and the planner reads it from here),
    // and the vowel the last mora sang (what a vowel kana may continue)
    let mut head: Option<usize> = None;
    let mut last_vowel: Option<&str> = None;
    let mut geminate = false;
    let male = BodyMix::MALE;
    for (k, m) in ms.iter().enumerate() {
        let next = ms.get(k + 1);
        match m {
            Mora::Syl { onset, vowel, src, plain } => {
                let continues = plain.is_some_and(|p| {
                    last_vowel.is_some_and(|l| l == *vowel || (p == 2 && l == "o") || (p == 1 && l == "e"))
                });
                if continues {
                    let nucleus = out.last().map(|s| s.nucleus.clone()).unwrap_or_default();
                    out.push(Syllable { text: src.clone(), nucleus, melisma: Some(true), ..Default::default() });
                } else {
                    let mut segs: Vec<Segment> = onset.iter().map(|c| consonant(c)).collect();
                    if let (true, Some(first)) = (geminate, segs.first_mut()) {
                        let spoken = phoneme(&first.ipa, male).map_or(0.07, |d| d.dur);
                        first.dur = Some(spoken * SUNG_CONSONANTS * GEMINATE);
                    }
                    out.push(Syllable { text: src.clone(), onset: segs, nucleus: Segment::new(vowel), ..Default::default() });
                    head = Some(out.len() - 1);
                    last_vowel = Some(vowel);
                }
                geminate = false;
            }
            Mora::Long(src) => {
                if let Some(prev) = out.last() {
                    let nucleus = prev.nucleus.clone();
                    out.push(Syllable { text: src.clone(), nucleus, melisma: Some(true), ..Default::default() });
                }
                geminate = false;
            }
            Mora::Nasal(src) => {
                let place = nasal_before(next);
                let nasal = Segment { dur: Some(NASAL_CODA), ..Segment::new(place) };
                match head.filter(|&h| out[h].coda.is_empty()) {
                    Some(h) => {
                        out[h].coda.push(nasal);
                        out.last_mut().unwrap().text.push_str(src);
                    }
                    None => {
                        out.push(Syllable { text: src.clone(), onset: vec![Segment::new(place)], nucleus: Segment::new("ɯ"), ..Default::default() });
                        head = Some(out.len() - 1);
                    }
                }
                last_vowel = None;
                geminate = false;
            }
            Mora::Sokuon(src) => {
                if let Some(last) = out.last_mut() {
                    last.text.push_str(src);
                    if can_geminate(next) {
                        geminate = true;
                    } else if let Some(h) = head.filter(|&h| out[h].coda.is_empty()) {
                        out[h].coda.push(Segment::new("ʔ"));
                    }
                }
                last_vowel = None;
            }
        }
    }
    out
}

/// The Japanese front end: kana, by the rules above (no data needed).
pub struct Japanese;

impl FrontEnd for Japanese {
    fn starts(&self, c: char) -> bool {
        is_kana(c)
    }
    fn continues(&self, c: char) -> bool {
        is_kana(c) || is_mark(c)
    }
    fn word(&self, word: &str) -> Vec<Syllable> {
        word_syllables(word)
    }
}
