//! IPA lyrics: a word written in IPA between slashes (`/ˈʃtʁaːsə/`) sings as IPA, so any language
//! the phoneme inventory covers can be sung without a front end of its own. Text-to-IPA stays
//! outside the engine (`tools/text-to-ipa.ts` runs the user's eSpeak NG); this is the other half,
//! reading what a person or a converter wrote.
//!
//! **Words.** Everything up to the closing slash on the same line is one word. Spaces inside it
//! separate words (`/ˈhɛ loʊ/`). An unclosed slash is dropped like any stray mark.
//!
//! **Syllables** are found from the sounds alone, with no onset table of any language:
//! - Every vowel and diphthong the planner knows is a nucleus.
//! - Consonants before the first nucleus are its onset, after the last its coda (`/ˈpsixa/`, `/ˈmnɛ/`
//!   keep their clusters).
//! - Between two nuclei, the longest run of consonants at the end whose sonority rises goes to the
//!   next syllable; the rest closes the previous one. Sonority comes from the inventory's manners:
//!   stops and affricates, then fricatives, nasals, liquids (laterals, taps, trills), then
//!   approximants. A single consonant always starts the next syllable (`ɡɾa.sjas`), a falling pair
//!   splits (`mas.ta`); write a `.` to say otherwise.
//! - A `.` breaks a syllable there (`,` and `|` too). Stress marks do not: eSpeak NG writes them
//!   right before the vowel (`həlˈəʊ`), the IPA convention puts them before the syllable
//!   (`həˈləʊ`), and sonority gives `hə.ləʊ` for both. Two vowels side by side are two syllables.
//!
//! **Marks** read as the tokenizer reads them: stress, length `ː`, tie bars, `ʰ` and `˭`, and the
//! marks that change how a sound is made: a combining tilde on a vowel (`/bɔ̃ʒuʁ/`: nasalized), `ʲ`
//! on a consonant (palatalized), `ʱ` on a voiced stop or affricate and a diaeresis below on a vowel
//! (breathy); see `docs/architecture.md`. Written as narrow transcription, an unmarked voiceless
//! stop or affricate is unaspirated and `ʰ` makes it aspirated, so `/pa/` and `/pʰa/` contrast (the
//! English positional rule would decide otherwise). A note's text is the syllable's IPA as the
//! engine read it: stress, length and all these marks kept, tie bars dropped, `ɡ` shown as `g`.
//!
//! **Nothing vanishes silently.** The tokenizer drops symbols it does not know and combining marks
//! it does not model (a devoicing ring, a tilde on a consonant), and a mark it reads but cannot
//! apply (`ʲ` after a vowel); a symbol it knows but the planner has no sound for (`ʏ`) is dropped
//! here. A word with no vowel is not sung. [`ipa_lyric_report`] lists all of these for a lyric.

use super::inventory::{Glottal, Manner, PhonemeDef, Voicing, phoneme};
use super::ipa::{SOUND_MARKS, ignored_ipa_marks, known_ipa_symbols, mark_applies, tokenize_ipa, unknown_ipa};
use super::planner::is_vowel;
use super::syllables::{FrontEnd, Token, lyric_tokens};
use super::{Segment, Syllable};
use crate::anatomy::human::BodyMix;

/// The IPA front end: words between slashes.
pub struct IpaLyrics;

impl FrontEnd for IpaLyrics {
    fn starts(&self, c: char) -> bool {
        c == '/'
    }
    /// From an opening slash to the next one on the same line. With none, just the slash, which
    /// `claims` refuses.
    fn scan(&self, s: &[char], i: usize) -> usize {
        match s[i + 1..].iter().position(|&c| c == '/' || c == '\n') {
            Some(k) if s[i + 1 + k] == '/' => i + k + 2,
            _ => i + 1,
        }
    }
    fn claims(&self, word: &str) -> bool {
        word.chars().count() >= 2 && word.ends_with('/')
    }
    fn word(&self, word: &str) -> Vec<Syllable> {
        let mut out = Vec::new();
        for w in inner(word).split_whitespace() {
            let mut syls = word_syllables(w);
            if let Some(last) = syls.last_mut() {
                last.word_end = true;
            }
            out.append(&mut syls);
        }
        out
    }
}

/// The IPA between a scanned word's slashes.
fn inner(word: &str) -> &str {
    word.strip_prefix('/').and_then(|w| w.strip_suffix('/')).unwrap_or("")
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Vowel,
    /// with its sonority
    Consonant(u8),
    /// read by the tokenizer, but the planner has no sound for it
    Silent,
}

/// How sonorous a consonant is, from its manner (higher is nearer a vowel).
fn sonority(d: &PhonemeDef) -> u8 {
    match d.manner {
        Manner::Stop | Manner::Affricate => 1,
        Manner::Glottal if d.glottal == Some(Glottal::Stop) => 1,
        Manner::Fricative | Manner::Glottal => 2,
        Manner::Nasal => 3,
        Manner::Lateral | Manner::Tap | Manner::Trill => 4,
        Manner::Approximant => 5,
        Manner::Vowel => 6,
    }
}

fn kind(ipa: &str) -> Kind {
    if is_vowel(ipa, BodyMix::MALE) {
        return Kind::Vowel;
    }
    phoneme(ipa, BodyMix::MALE).map_or(Kind::Silent, |d| Kind::Consonant(sonority(&d)))
}

/// A segment as the tokenizer read it, with the IPA that shows it and what kind of sound it is.
struct Seg {
    seg: Segment,
    typed: String,
    kind: Kind,
}

/// Length and the sound marks, as the tokenizer took them: a tilde or a diaeresis below right after
/// the vowel it marks, `ʰ ˭ ʲ ʱ` after the consonant.
fn typed(s: &Segment) -> String {
    let aspirated = match s.aspirated {
        Some(true) => "ʰ",
        Some(false) => "˭",
        None => "",
    };
    let vowel = is_vowel(&s.ipa, BodyMix::MALE);
    let (tilde, below) = (if s.nasalized == Some(true) { "\u{303}" } else { "" }, if s.breathy == Some(true) && vowel { "\u{324}" } else { "" });
    let (soft, breathy) = (if s.palatalized == Some(true) { "ʲ" } else { "" }, if s.breathy == Some(true) && !vowel { "ʱ" } else { "" });
    format!("{}{tilde}{below}{}{aspirated}{soft}{breathy}", s.ipa, if s.dur.is_some() { "ː" } else { "" })
}

/// The pieces a word is read in: it breaks at `.` `,` `|` (dropped).
fn pieces(word: &str) -> Vec<&str> {
    word.split(['.', ',', '|']).filter(|p| !p.is_empty()).collect()
}

/// One space-free IPA word → its syllables.
fn word_syllables(word: &str) -> Vec<Syllable> {
    let mut segs: Vec<Seg> = Vec::new();
    let mut breaks: Vec<usize> = Vec::new();
    for piece in pieces(word) {
        let before = segs.len();
        for s in tokenize_ipa(piece) {
            if s.ipa == "_" {
                continue;
            }
            let k = kind(&s.ipa);
            if k != Kind::Silent {
                segs.push(Seg { typed: typed(&s), seg: s, kind: k });
            }
        }
        if before > 0 && segs.len() > before {
            breaks.push(before);
        }
    }
    let nuclei: Vec<usize> = (0..segs.len()).filter(|&i| segs[i].kind == Kind::Vowel).collect();
    if nuclei.is_empty() {
        return Vec::new();
    }
    let son = |i: usize| if let Kind::Consonant(n) = segs[i].kind { n } else { 0 };
    // where each syllable after the first begins
    let mut starts = vec![0];
    for w in nuclei.windows(2) {
        let (v, next) = (w[0], w[1]);
        let cut = if let Some(&b) = breaks.iter().find(|&&b| b > v && b <= next) {
            b
        } else if next == v + 1 {
            next
        } else {
            // the last consonant starts the next syllable, and so does each one before it that is less sonorous than the one after
            let mut cut = next - 1;
            while cut > v + 1 && son(cut - 1) < son(cut) {
                cut -= 1;
            }
            cut
        };
        starts.push(cut);
    }
    starts.push(segs.len());
    let aspiration = |s: &Segment| {
        let mut s = s.clone();
        let unaspirated = phoneme(&s.ipa, BodyMix::MALE).is_some_and(|d| d.voicing == Voicing::Voiceless && matches!(d.manner, Manner::Stop | Manner::Affricate));
        if unaspirated && s.aspirated.is_none() {
            s.aspirated = Some(false);
        }
        s
    };
    nuclei
        .iter()
        .enumerate()
        .map(|(n, &v)| {
            let (from, to) = (starts[n], starts[n + 1]);
            let stress = match segs[v].seg.stress {
                Some(1) => "ˈ",
                Some(2) => "ˌ",
                _ => "",
            };
            let text: String = segs[from..to].iter().map(|s| s.typed.as_str()).collect();
            Syllable {
                text: format!("{stress}{text}"),
                onset: segs[from..v].iter().map(|s| aspiration(&s.seg)).collect(),
                nucleus: segs[v].seg.clone(),
                coda: segs[v + 1..to].iter().map(|s| aspiration(&s.seg)).collect(),
                ..Default::default()
            }
        })
        .collect()
}

/// Every symbol the engine can sing in IPA text: the tokenizer's symbols that the planner has a
/// sound for (`ʏ`, a vowel letter without a calibrated vowel, is not one), sorted. Tools that
/// convert other IPA to what the engine takes (`tools/text-to-ipa.ts`) read this.
pub fn sung_symbols() -> Vec<&'static str> {
    known_ipa_symbols().into_iter().filter(|s| kind(s) != Kind::Silent).collect()
}

/// A mark the engine sings, and the sung symbols it is realised on.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SungMark {
    pub mark: String,
    pub on: Vec<&'static str>,
}

/// Every mark the tokenizer reads that changes how a sound is made (`ʰ ˭ ʲ ʱ`, the combining tilde
/// and diaeresis below), each with the sung symbols it applies to. Tools that convert other IPA
/// (`tools/text-to-ipa.ts`) read this, so a mark the engine gains is kept, not stripped.
pub fn sung_marks() -> Vec<SungMark> {
    let sung = sung_symbols();
    SOUND_MARKS.iter().map(|&m| SungMark { mark: m.to_string(), on: sung.iter().copied().filter(|s| mark_applies(m, s)).collect() }).collect()
}

/// What an IPA lyric asks for that the engine does not sing as written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IpaReport {
    /// symbols the tokenizer does not know (dropped)
    pub unknown: Vec<String>,
    /// symbols the tokenizer knows but the planner has no sound for (dropped)
    pub no_sound: Vec<String>,
    /// letters with a mark the engine does not model or cannot apply there (the letter is sung, the
    /// mark is not)
    pub ignored_marks: Vec<String>,
    /// words with no vowel (not sung), as typed between their slashes
    pub no_vowel: Vec<String>,
}

impl IpaReport {
    pub fn is_empty(&self) -> bool {
        self.unknown.is_empty() && self.no_sound.is_empty() && self.ignored_marks.is_empty() && self.no_vowel.is_empty()
    }
    /// One line per kind of gap, ready to print.
    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut line = |what: &str, items: &[String]| {
            if !items.is_empty() {
                out.push(format!("IPA lyrics: {what}: {}", items.join(" ")));
            }
        };
        line("symbols the engine does not know (not sung)", &self.unknown);
        line("symbols the engine has no sound for (not sung)", &self.no_sound);
        line("marks the engine does not model, or cannot apply there (the letter is sung without them)", &self.ignored_marks);
        line("words with no vowel (not sung)", &self.no_vowel);
        out
    }
}

fn add(list: &mut Vec<String>, items: impl IntoIterator<Item = String>) {
    for i in items {
        if !list.contains(&i) {
            list.push(i);
        }
    }
}

/// What the IPA words of `text` (as the router reads them) lose when sung. Each item appears once.
pub fn ipa_lyric_report(text: &str) -> IpaReport {
    let mut r = IpaReport::default();
    for tok in lyric_tokens(&text.replace('’', "'"), &[&IpaLyrics]) {
        let Token::Word(_, w) = tok else { continue };
        let ipa = inner(&w);
        add(&mut r.unknown, unknown_ipa(ipa));
        add(&mut r.ignored_marks, ignored_ipa_marks(ipa));
        add(&mut r.no_sound, tokenize_ipa(ipa).into_iter().filter(|s| kind(&s.ipa) == Kind::Silent).map(|s| s.ipa));
        for sub in ipa.split_whitespace().chain(ipa.trim().is_empty().then_some("")) {
            if word_syllables(sub).is_empty() {
                add(&mut r.no_vowel, [format!("/{sub}/")]);
            }
        }
    }
    r
}
