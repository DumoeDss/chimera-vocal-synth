/**
 * The cleaning half of tools/text-to-ipa.ts, as pure functions: eSpeak NG's `--ipa` output (or any
 * other IPA) → the IPA words the engine sings (`/ˈʃøːn/`, see docs/language-front-ends.md).
 *
 * eSpeak NG's output format, as its documentation describes it (not verified against a real
 * eSpeak NG on this project's machines): one line per clause, words separated by spaces, stress
 * marks ˈ ˌ before the stressed vowel, length ː, and, with `--ipa` (no argument) possibly a
 * zero-width joiner or a tie bar between the parts of a phoneme. When a word is spoken by another
 * voice, the output is annotated with the language, `(en)thanks(de)`.
 *
 * What the engine can sing comes from the engine (`ipaSymbols()`, the symbols the planner has a
 * sound for), passed in as `engine`: nothing here lists the engine's sounds, so a sound the
 * engine gains is kept as it is and the table below can't rot (tests/text-to-ipa.test.ts fails
 * when an entry's source became a sung symbol or its target is not one).
 */

/** Combining marks (the tilde of ɔ̃, the ring of n̥, the non-syllabic arch of ɪ̯). */
const COMBINING = /[̀-ͯ]/;

/** Marks the engine reads that are not sounds: stress, length, aspiration. */
export const MODIFIERS: ReadonlySet<string> = new Set(["ˈ", "ˌ", "ː", "ʰ", "˭"]);

export interface Mapping {
  /** what eSpeak (or another G2P) writes */
  from: string;
  /** the sound the engine has that is nearest, written in symbols it sings */
  to: string;
  why: string;
  /** the same sound written another way: not reported */
  exact?: boolean;
}

/**
 * THE table: symbols the engine does not sing, and the nearest it does. Keys are matched longest
 * first. A combining mark that is not here is dropped from its letter and reported (nasal vowels
 * ɔ̃ are sung as ɔ), so a nasal needs no entry. A symbol that is neither here nor sung is dropped
 * and reported. Choose by ear and say why: these are nearest-sound guesses, not phonology.
 */
export const MAPPINGS: readonly Mapping[] = [
  // German (eSpeak de)
  { from: "ʏ", to: "y", why: "no ʏ vowel; near-close ʏ sung as close y" },
  { from: "ɔʏ", to: "ɔɪ", why: "German eu: the engine's diphthong is ɔɪ" },
  { from: "ɐ̯", to: "ɐ", why: "the engine takes ɐ, with no non-syllabic mark" },
  // diphthong glides written non-syllabic: the engine's diphthongs are whole nuclei
  { from: "aɪ̯", to: "aɪ", why: "non-syllabic mark: the diphthong is one nucleus" },
  { from: "aʊ̯", to: "aʊ", why: "non-syllabic mark: the diphthong is one nucleus" },
  { from: "ɔɪ̯", to: "ɔɪ", why: "non-syllabic mark: the diphthong is one nucleus" },
  { from: "ɔʏ̯", to: "ɔɪ", why: "non-syllabic mark, and no ʏ: ɔɪ" },
  { from: "eɪ̯", to: "eɪ", why: "non-syllabic mark: the diphthong is one nucleus" },
  { from: "oʊ̯", to: "oʊ", why: "non-syllabic mark: the diphthong is one nucleus" },
  // a lone non-syllabic vowel is a glide: it would otherwise be a nucleus of its own
  { from: "ɪ̯", to: "j", why: "non-syllabic vowel sung as the glide" },
  { from: "i̯", to: "j", why: "non-syllabic vowel sung as the glide" },
  { from: "ʊ̯", to: "w", why: "non-syllabic vowel sung as the glide" },
  { from: "u̯", to: "w", why: "non-syllabic vowel sung as the glide" },
  { from: "y̯", to: "ɥ", why: "non-syllabic vowel sung as the glide" },
  { from: "ʏ̯", to: "ɥ", why: "non-syllabic vowel sung as the glide" },
  // vowels with no calibrated sound (the engine's list is ipaSymbols())
  { from: "ɜ", to: "ə", why: "no ɜ vowel; the nearest central vowel is ə" },
  { from: "ɘ", to: "ə", why: "no ɘ vowel; the nearest central vowel is ə" },
  { from: "ɵ", to: "ø", why: "no ɵ vowel; the nearest rounded mid vowel is ø" },
  { from: "ɶ", to: "œ", why: "no ɶ vowel; the nearest open front rounded vowel is œ" },
  // eSpeak's English: the centralised near-close vowels of unstressed syllables
  { from: "ᵻ", to: "ɪ", why: "eSpeak's centralised ɪ" },
  { from: "ᵿ", to: "ʊ", why: "eSpeak's centralised ʊ" },
  // the same sounds under other symbols: the tokenizer reads them too, so they are not reported
  { from: "ɡ", to: "g", why: "script g", exact: true },
  { from: "ʧ", to: "tʃ", why: "ligature", exact: true },
  { from: "ʤ", to: "dʒ", why: "ligature", exact: true },
];

/** One thing the cleaner changed, for the user to read. */
export interface Change {
  /** what was in the input (a symbol, or a letter with its mark) */
  from: string;
  /** what it became; "" when it was dropped */
  to: string;
  why: string;
}

/** What eSpeak printed → its IPA words, and the language switches it annotated. */
export function splitEspeak(raw: string): { words: string[]; switches: string[] } {
  const switches: string[] = [];
  // (en) (de) (fr-fr): the voice eSpeak switched to for a word of another language
  const text = raw.replace(/\([a-z]{2,3}(?:-[a-z0-9]+)*\)/gi, (m) => {
    if (!switches.includes(m)) switches.push(m);
    return " ";
  });
  return { words: formatless(text).split(/\s+/).filter(Boolean), switches };
}

/** Joiners, tie bars and eSpeak's `_` phoneme separator (--ipa=1) leave no trace in the sound. */
function formatless(s: string): string {
  return s.replace(/[​-‍⁠﻿͜͡_]/g, "");
}

/** A word from `--ipa` input: its slashes or brackets, if it came with them, are not the IPA. */
export function bareWord(w: string): string {
  return formatless(w).replace(/^[/[]+|[/\]]+$/g, "");
}

const MAP_ORDER = [...MAPPINGS].sort((a, b) => b.from.length - a.from.length);

/** The longest `s.slice(i, i + n)`, n from `max` down to 1, that `has` accepts, or "". */
function longest(s: string, i: number, max: number, has: (x: string) => boolean): string {
  for (let n = Math.min(max, s.length - i); n >= 1; n--) if (has(s.slice(i, i + n))) return s.slice(i, i + n);
  return "";
}

/**
 * One IPA word → the IPA the engine sings, with what changed. A symbol the engine sings stays as
 * it is, so is a stress, length or aspiration mark. A symbol in MAPPINGS becomes its entry. A
 * letter with a combining mark the engine does not model loses the mark (a nasal vowel is sung
 * oral), a precomposed letter (ã) is read as its base and the mark, and anything else is dropped.
 * Diacritics are never stripped from a symbol the engine sings: ç stays ç.
 */
export function cleanWord(word: string, engine: ReadonlySet<string>): { ipa: string; changes: Change[] } {
  const w = bareWord(word);
  const changes: Change[] = [];
  let out = "";
  let i = 0;
  const note = (from: string, to: string, why: string) => changes.push({ from, to, why });
  // a letter's combining marks, which the engine does not model
  const marks = (j: number): string => {
    let m = "";
    while (j + m.length < w.length && COMBINING.test(w[j + m.length])) m += w[j + m.length];
    return m;
  };
  while (i < w.length) {
    const key = longest(w, i, 3, (x) => MAP_ORDER.some((m) => m.from === x));
    if (key) {
      const m = MAP_ORDER.find((e) => e.from === key)!;
      if (!m.exact) note(key, m.to, m.why);
      out += m.to;
      i += key.length;
      continue;
    }
    const sym = longest(w, i, 2, (x) => engine.has(x));
    if (sym) {
      const m = marks(i + sym.length);
      if (m) note(sym + m, sym, "mark not modelled by the engine; the letter is sung without it");
      out += sym;
      i += sym.length + m.length;
      continue;
    }
    const c = w[i];
    if (MODIFIERS.has(c)) {
      out += c;
      i++;
      continue;
    }
    if (c === ".") {
      out += c; // a syllable break the writer chose
      i++;
      continue;
    }
    // a precomposed letter (ã): its base is the sound, the mark is not modelled
    const d = c.normalize("NFD");
    if (d.length > 1 && engine.has(d[0]) && COMBINING.test(d[1])) {
      const m = marks(i + 1);
      note(c + m, d[0], "mark not modelled by the engine; the letter is sung without it");
      out += d[0];
      i += 1 + m.length;
      continue;
    }
    const m = COMBINING.test(c) ? "" : marks(i + 1);
    note(c + m, "", COMBINING.test(c) ? "a combining mark on nothing the engine sings" : "not a sound the engine has, and no nearest one in the table");
    i += 1 + m.length;
  }
  return { ipa: out, changes };
}

/** Every change of every word, merged by what changed, with the words it happened in. */
export interface Report {
  from: string;
  to: string;
  why: string;
  words: string[];
}

export function cleanWords(words: readonly string[], engine: ReadonlySet<string>): { words: string[]; report: Report[] } {
  const report: Report[] = [];
  const out: string[] = [];
  for (const w of words) {
    const { ipa, changes } = cleanWord(w, engine);
    if (ipa) out.push(ipa);
    for (const c of changes) {
      let r = report.find((x) => x.from === c.from && x.to === c.to);
      if (!r) report.push((r = { ...c, words: [] }));
      if (!r.words.includes(bareWord(w))) r.words.push(bareWord(w));
    }
  }
  return { words: out, report };
}

/** The lyric line Sing and a melody file's `"lyrics"` take. */
export function toLyric(words: readonly string[]): string {
  return words.map((w) => `/${w}/`).join(" ");
}

/** One stderr line per change. */
export function formatReport(report: readonly Report[]): string[] {
  return report.map((r) => `${r.from} → ${r.to || "(dropped)"}  ${r.why}  [${r.words.join(" ")}]`);
}
