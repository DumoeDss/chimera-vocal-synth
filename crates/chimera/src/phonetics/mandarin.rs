//! The Mandarin front end: Han characters → sung syllables, one per character, so a character maps
//! to one note and keeps its spelling as the note's text.
//!
//! - Readings come from a lexicon (`public/data/mandarin.txt.gz`, built by tools/build-mandarin.ts
//!   from MIT pinyin data): each character's most common reading, and the phrases whose reading
//!   differs from their characters' own. A run of characters is read by longest phrase match,
//!   which decides most polyphones (银行 yin2 hang2, 行走 xing2 zou3).
//! - Tone sandhi inside the run: 不 before a fourth tone is second (不会 bu2 hui4); 一 before a
//!   fourth tone is second and before the others fourth; a run of third tones keeps the last and
//!   turns the others second (你好 ni2 hao3).
//! - Pinyin → IPA on the inventory. Stops and affricates say their aspiration, which is what
//!   Mandarin contrasts: b d g z zh j are unaspirated (p˭ t˭ k˭ ts˭ tʂ˭ tɕ˭), p t k c ch q aspirated.
//! - Tones → pitch bends ([`tone_bends`]): the melody stays as written, the tone colours the
//!   note's start or end.

use super::syllables::FrontEnd;
use super::{Bend, Edge, Segment, Syllable};
use std::collections::HashMap;

pub fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}')
}

/// Readings: each character's most common, and the phrases that read otherwise.
#[derive(Default)]
pub struct MandarinLexicon {
    chars: HashMap<char, String>,
    phrases: HashMap<String, Vec<String>>,
    longest: usize,
}

impl MandarinLexicon {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load "字<TAB>zhong1" and "银行<TAB>yin2 hang2" lines (later lines win).
    pub fn load(&mut self, text: &str) {
        for line in text.lines() {
            let Some((w, py)) = line.split_once('\t') else { continue };
            let syls: Vec<String> = py.split(' ').filter(|s| !s.is_empty()).map(str::to_string).collect();
            let chars: Vec<char> = w.chars().collect();
            if chars.len() == 1 && syls.len() == 1 {
                self.chars.insert(chars[0], syls[0].clone());
            } else if chars.len() == syls.len() && chars.len() > 1 {
                self.longest = self.longest.max(chars.len());
                self.phrases.insert(w.to_string(), syls);
            }
        }
    }

    pub fn is_loaded(&self) -> bool {
        !self.chars.is_empty()
    }

    /// A run of Han characters → one numbered pinyin syllable per character ("" where unknown),
    /// by longest phrase match, then tone sandhi.
    pub fn read(&self, run: &str) -> Vec<String> {
        let cs: Vec<char> = run.chars().collect();
        let mut out: Vec<String> = Vec::with_capacity(cs.len());
        let mut i = 0;
        while i < cs.len() {
            let mut took = false;
            for n in (2..=self.longest.min(cs.len() - i)).rev() {
                let w: String = cs[i..i + n].iter().collect();
                if let Some(syls) = self.phrases.get(&w) {
                    out.extend(syls.iter().cloned());
                    i += n;
                    took = true;
                    break;
                }
            }
            if !took {
                out.push(self.chars.get(&cs[i]).cloned().unwrap_or_default());
                i += 1;
            }
        }
        sandhi(&cs, &mut out);
        out
    }
}

fn tone(py: &str) -> u8 {
    py.chars().last().and_then(|c| c.to_digit(10)).map_or(5, |d| d as u8)
}
fn set_tone(py: &mut String, t: u8) {
    if py.ends_with(|c: char| c.is_ascii_digit()) {
        py.pop();
        py.push(char::from(b'0' + t));
    }
}

fn sandhi(cs: &[char], py: &mut [String]) {
    let n = py.len();
    for i in 0..n {
        let next = if i + 1 < n { Some(tone(&py[i + 1])) } else { None };
        match (cs[i], next) {
            ('不', Some(4)) => set_tone(&mut py[i], 2),
            ('一', Some(4)) => set_tone(&mut py[i], 2),
            ('一', Some(1..=3)) => set_tone(&mut py[i], 4),
            _ => {}
        }
    }
    // a run of third tones: all but the last become second
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && tone(&py[j]) == 3 && !py[j].is_empty() {
            j += 1;
        }
        for k in i..j.saturating_sub(1) {
            set_tone(&mut py[k], 2);
        }
        i = j.max(i + 1);
    }
}

/// How a Mandarin tone bends its sung note (semitones from the note; see [`Bend`]): 2 scoops up
/// from below, 3 dips, 4 starts a little high and falls off the end, 1 and 5 stay on the note.
pub fn tone_bends(tone: u8) -> Vec<Bend> {
    let b = |at, len, max, st| Bend { at, len, max, st };
    match tone {
        2 => vec![b(Edge::Start, 0.35, 0.15, -2.0)],
        3 => vec![b(Edge::Start, 0.49, 0.21, -3.0)],
        4 => vec![b(Edge::Start, 0.21, 0.09, 1.0), b(Edge::End, 0.35, 0.15, -3.0)],
        _ => vec![],
    }
}

/// An initial → its sound and, for stops and affricates, its aspiration.
fn initial(i: &str) -> Option<(&'static str, Option<bool>)> {
    Some(match i {
        "b" => ("p", Some(false)),
        "p" => ("p", Some(true)),
        "m" => ("m", None),
        "f" => ("f", None),
        "d" => ("t", Some(false)),
        "t" => ("t", Some(true)),
        "n" => ("n", None),
        "l" => ("l", None),
        "g" => ("k", Some(false)),
        "k" => ("k", Some(true)),
        "h" => ("x", None),
        "j" => ("tɕ", Some(false)),
        "q" => ("tɕ", Some(true)),
        "x" => ("ɕ", None),
        "zh" => ("tʂ", Some(false)),
        "ch" => ("tʂ", Some(true)),
        "sh" => ("ʂ", None),
        "r" => ("ɻ", None),
        "z" => ("ts", Some(false)),
        "c" => ("ts", Some(true)),
        "s" => ("s", None),
        _ => return None,
    })
}

/// A final (after its initial, with y/w spellings undone) → glide, nucleus, coda.
fn final_parts(f: &str) -> Option<(&'static [&'static str], &'static str, &'static [&'static str])> {
    const N: &[&str] = &[];
    Some(match f {
        "a" => (N, "a", N),
        "o" => (&["w"], "o", N),
        "e" => (N, "ɤ", N),
        "i" => (N, "i", N),
        "u" => (N, "u", N),
        "v" => (N, "y", N),
        "er" => (N, "ɝ", N),
        "ai" => (N, "aɪ", N),
        "ei" => (N, "eɪ", N),
        "ao" => (N, "aʊ", N),
        "ou" => (N, "oʊ", N),
        "an" => (N, "a", &["n"]),
        "en" => (N, "ə", &["n"]),
        "ang" => (N, "ɑ", &["ŋ"]),
        "eng" => (N, "ə", &["ŋ"]),
        "ong" => (N, "ʊ", &["ŋ"]),
        "ia" => (&["j"], "a", N),
        "ie" => (&["j"], "ɛ", N),
        "iao" => (&["j"], "aʊ", N),
        "iu" | "iou" => (&["j"], "oʊ", N),
        "ian" => (&["j"], "ɛ", &["n"]),
        "in" => (N, "i", &["n"]),
        "iang" => (&["j"], "ɑ", &["ŋ"]),
        "ing" => (N, "i", &["ŋ"]),
        "iong" => (&["j"], "ʊ", &["ŋ"]),
        "ua" => (&["w"], "a", N),
        "uo" => (&["w"], "o", N),
        "uai" => (&["w"], "aɪ", N),
        "ui" | "uei" => (&["w"], "eɪ", N),
        "uan" => (&["w"], "a", &["n"]),
        "un" | "uen" => (&["w"], "ə", &["n"]),
        "uang" => (&["w"], "ɑ", &["ŋ"]),
        "ueng" => (&["w"], "ə", &["ŋ"]),
        "ve" | "ue" => (&["ɥ"], "ɛ", N),
        "van" => (&["ɥ"], "ɛ", &["n"]),
        "vn" => (N, "y", &["n"]),
        // syllabic nasals (嗯 n, ng, m)
        "n" | "ng" | "m" | "hm" | "hng" => (N, "ə", &["n"]),
        _ => return None,
    })
}

/// One numbered pinyin syllable ("zhong1") → a sung syllable with its spelling `text`; None when
/// it isn't pinyin.
pub fn pinyin_syllable(py: &str, text: &str) -> Option<Syllable> {
    let t = tone(py);
    let body = py.trim_end_matches(|c: char| c.is_ascii_digit());
    let ini = ["zh", "ch", "sh"].into_iter().find(|i| body.starts_with(i)).or_else(|| {
        let c = &body[..body.chars().next()?.len_utf8()];
        (initial(c).is_some() || c == "y" || c == "w").then_some(c)
    });
    let (ini, mut fin) = match ini {
        Some(i) if body.len() > i.len() => (i, body[i.len()..].to_string()),
        _ => ("", body.to_string()),
    };
    // spellings: y/w stand for glides; u after j q x (and y) is ü
    match ini {
        "y" => {
            fin = match fin.as_str() {
                "i" | "in" | "ing" => fin,
                f if f.starts_with('u') => format!("v{}", &f[1..]),
                f if f.starts_with('i') => f.to_string(),
                f => format!("i{f}"),
            }
        }
        "w" => {
            fin = match fin.as_str() {
                "u" => fin,
                "o" => "uo".into(),
                f => format!("u{f}"),
            }
        }
        "j" | "q" | "x" if fin.starts_with('u') => fin = format!("v{}", &fin[1..]),
        _ => {}
    }
    let apical = matches!(ini, "z" | "c" | "s" | "zh" | "ch" | "sh" | "r") && fin == "i";
    let (glide, mut nucleus, coda) = if apical { (&[][..], "ɨ", &[][..]) } else { final_parts(&fin)? };
    // a neutral-tone syllable's e reduces (的 de, 了 le)
    if t == 5 && nucleus == "ɤ" {
        nucleus = "ə";
    }
    let mut onset: Vec<Segment> = Vec::new();
    if let Some((ipa, aspirated)) = initial(ini) {
        onset.push(Segment { aspirated, ..Segment::new(ipa) });
    }
    onset.extend(glide.iter().map(|g| Segment::new(g)));
    Some(Syllable {
        text: text.to_string(),
        onset,
        nucleus: Segment::new(nucleus),
        coda: coda.iter().map(|c| Segment::new(c)).collect(),
        pitch: tone_bends(t),
        ..Default::default()
    })
}

/// The Mandarin front end, over a loaded lexicon (without one it reads nothing).
pub struct Mandarin<'a>(pub &'a MandarinLexicon);

impl FrontEnd for Mandarin<'_> {
    fn starts(&self, c: char) -> bool {
        is_han(c)
    }
    fn word(&self, run: &str) -> Vec<Syllable> {
        let cs: Vec<char> = run.chars().collect();
        self.0.read(run).iter().zip(&cs).filter_map(|(py, c)| pinyin_syllable(py, &c.to_string())).collect()
    }
}
