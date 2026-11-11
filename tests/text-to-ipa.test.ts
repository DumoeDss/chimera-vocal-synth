/**
 * tools/text-to-ipa.ts: the cleaner (tools/lib/text-to-ipa.ts) on hand-written examples of eSpeak
 * NG's documented `--ipa` output format, and the tool itself run as a user runs it, without eSpeak
 * (`--ipa` input, and the missing-eSpeak message). The examples are NOT captured from a real
 * eSpeak NG: nothing here shows what a real one prints for these words.
 */
import { expect, test } from "bun:test";
import { fileURLToPath } from "node:url";
import { initEngine } from "./harness.ts";
import { MAPPINGS, MODIFIERS, bareWord, cleanWord, cleanWords, engineOf, splitEspeak, toLyric } from "../tools/lib/text-to-ipa.ts";

await initEngine();
const { ipaSymbols, ipaMarks, lyricsToSyllables } = await import("../src/wasm/engine.ts");
const engine = engineOf(ipaSymbols(), ipaMarks());
const clean = (w: string) => cleanWord(w, engine);

const TOOL = fileURLToPath(new URL("../tools/text-to-ipa.ts", import.meta.url));
function run(args: string[], stdin = "") {
  const p = Bun.spawnSync(["bun", TOOL, ...args], { stdin: new TextEncoder().encode(stdin) });
  return { code: p.exitCode, out: p.stdout.toString(), err: p.stderr.toString() };
}

// ------------------------------------------------------------------ splitting eSpeak's output
test("eSpeak's output splits into words: clauses on separate lines, joiners and tie bars removed", () => {
  // hand-written example of the format: two clauses, a ZWJ after a word, a tie bar in an affricate
  const raw = "ʃtʁˈaːsə dˈeːɐ\nmˈɔʁɡənʃtˌɛɐn‍\nt͡ʃˈaʊ\n";
  expect(splitEspeak(raw).words).toEqual(["ʃtʁˈaːsə", "dˈeːɐ", "mˈɔʁɡənʃtˌɛɐn", "tʃˈaʊ"]);
});

test("language-switch annotations are removed, and reported once each", () => {
  const r = splitEspeak("(en)θˈæŋks(de) fˈiːl");
  expect(r.words).toEqual(["θˈæŋks", "fˈiːl"]);
  expect(r.switches).toEqual(["(en)", "(de)"]);
  expect(splitEspeak("(fr-fr)bɔ̃ʒˈuʁ(de)").switches).toEqual(["(fr-fr)", "(de)"]);
});

test("the _ separator of --ipa=1 is removed", () => {
  expect(splitEspeak("h_ə_l_ˈoʊ").words).toEqual(["həlˈoʊ"]);
});

test("a word from --ipa input may carry its slashes or brackets", () => {
  expect(bareWord("/ˈʃøːn/")).toBe("ˈʃøːn");
  expect(bareWord("[ˈʃøːn]")).toBe("ˈʃøːn");
});

// ------------------------------------------------------------------ cleaning
test("symbols the engine sings pass through untouched, stress and length included", () => {
  for (const w of ["ʃtʁˈaːsə", "mˈɔʁɡənʃtˌɛɐn".replace("ɡ", "g"), "tʃˈaːo", "pʰˈa"]) {
    const r = clean(w);
    expect(r.ipa).toBe(w);
    expect(r.changes).toEqual([]);
  }
});

test("ç stays ç: German ich keeps its fricative, and no sung symbol is ever stripped to its base", () => {
  // the trap in the closed eSpeak PR: accent-stripping turned ç into c, a stop
  const r = clean("ˈɪç");
  expect(r.ipa).toBe("ˈɪç");
  expect(r.changes).toEqual([]);
  expect(clean("ˈiːç").ipa).toBe("ˈiːç");
  // every sung symbol that decomposes (ç) survives alone
  for (const s of engine.symbols) if (s.normalize("NFD") !== s) expect(clean(s).ipa).toBe(s);
});

test("symbols the engine lacks become the nearest it has, and are reported", () => {
  const r = clean("fˈʏʁ");
  expect(r.ipa).toBe("fˈyʁ");
  expect(r.changes).toEqual([{ from: "ʏ", to: "y", why: expect.any(String) }]);
  expect(clean("lˈɔʏçtət").ipa).toBe("lˈɔɪçtət"); // ɔʏ before ɔ + ʏ
  expect(clean("dˈeːɐ̯").ipa).toBe("dˈeːɐ");
  expect(clean("ˈaɪ̯n").ipa).toBe("ˈaɪn");
  expect(clean("bˈɪ̯a").ipa).toBe("bˈja");
});

test("a nasal vowel keeps its tilde: the engine sings it, so nothing is reported", () => {
  const r = clean("bɔ̃ʒˈuʁ");
  expect(r.ipa).toBe("bɔ̃ʒˈuʁ");
  expect(r.changes).toEqual([]);
  // a precomposed nasal vowel is read as its letter and the mark, which stays
  expect(clean("lã").ipa).toBe("la\u0303");
  expect(clean("lã").changes).toEqual([]);
});

test("a mark the engine does not read is dropped from its letter and reported; the letter stays", () => {
  // a devoicing ring, and a tilde on a consonant (the engine reads the tilde on vowels only)
  const r = clean("ˈn̥a");
  expect(r.ipa).toBe("ˈna");
  expect(r.changes.map((c) => [c.from, c.to])).toEqual([["n̥", "n"]]);
  expect(clean("m\u0303a").changes.map((c) => [c.from, c.to])).toEqual([["m\u0303", "m"]]);
  expect(clean("m\u0303a").ipa).toBe("ma");
});

test("the marks the engine reads stay on the sounds it reads them on: ʲ ʱ ʰ, the tilde, the diaeresis below", () => {
  for (const w of ["tʲat", "bʱa", "pʰa", "ɛ\u0324", "ã","aː\u0303"]) {
    expect(clean(w).ipa, w).toBe(w);
    expect(clean(w).changes, w).toEqual([]);
  }
  // on a sound the engine does not read it on, the mark is dropped and said
  const r = clean("aʲ");
  expect(r.ipa).toBe("a");
  expect(r.changes.map((c) => [c.from, c.to])).toEqual([["ʲ", ""]]);
  expect(clean("pʱa").ipa).toBe("pa");
  // a mark on nothing
  expect(clean("ʲa").ipa).toBe("a");
});

test("every mark the engine lists is kept on each symbol it applies to and dropped from every other", () => {
  expect(engine.marks.size).toBeGreaterThanOrEqual(6);
  for (const [mark, on] of engine.marks) {
    expect(on.size, `${mark} applies to nothing`).toBeGreaterThan(0);
    for (const sym of engine.symbols) {
      const r = clean(sym + mark);
      if (on.has(sym)) {
        expect(r, `${sym}${mark}`).toEqual({ ipa: sym + mark, changes: [] });
      } else {
        expect(r.ipa, `${sym}${mark}`).toBe(sym);
        expect(r.changes.length, `${sym}${mark}`).toBe(1);
      }
    }
  }
});

test("a symbol with no sound and no entry is dropped, and said", () => {
  const r = clean("ˈŊa");
  expect(r.ipa).toBe("ˈa");
  expect(r.changes.map((c) => [c.from, c.to])).toEqual([["Ŋ", ""]]);
});

test("a dot the writer chose stays, and the same sound under another symbol is not reported", () => {
  const r = clean("ɡa.ʧa");
  expect(r.ipa).toBe("ga.tʃa");
  expect(r.changes).toEqual([]);
});

test("changes merge across words, with the words they happened in", () => {
  const { words, report } = cleanWords(["fˈʏʁ", "mˈʏdə", "ˈɪç"], engine);
  expect(words).toEqual(["fˈyʁ", "mˈydə", "ˈɪç"]);
  expect(report).toEqual([{ from: "ʏ", to: "y", why: expect.any(String), words: ["fˈʏʁ", "mˈʏdə"] }]);
  expect(toLyric(words)).toBe("/fˈyʁ/ /mˈydə/ /ˈɪç/");
});

test("the cleaned words are what the engine sings: every one has a syllable", () => {
  const { words } = cleanWords(splitEspeak("ʃtʁˈaːsə fˈʏʁ lˈɔʏçtət bɔ̃ʒˈuʁ ˈɪç").words, engine);
  const s = lyricsToSyllables(toLyric(words));
  expect(s.filter((x) => x.wordEnd).length).toBe(5);
});

// ------------------------------------------------------------------ the table can't rot
test("every table entry still names a symbol the engine lacks and a replacement it sings", () => {
  // a replacement is symbols the engine sings (or marks), longest match first
  const sung = (s: string) => {
    let i = 0;
    while (i < s.length) {
      const n = [2, 1].find((k) => engine.symbols.has(s.slice(i, i + k)) || (k === 1 && (MODIFIERS.has(s[i]) || engine.marks.has(s[i]))));
      if (!n) return false;
      i += n;
    }
    return true;
  };
  for (const m of MAPPINGS) {
    expect(sung(m.to), `${m.from} → ${m.to}: the target is not sung`).toBe(true);
    if (!m.exact) expect(engine.symbols.has(m.from), `${m.from} is sung now: delete its entry`).toBe(false);
  }
});

test("no table entry maps away a mark the engine reads, and no sound mark is left to MODIFIERS", () => {
  for (const [mark, on] of engine.marks) {
    expect(MODIFIERS.has(mark), `${mark} is the engine's now: take it out of MODIFIERS`).toBe(false);
    for (const m of MAPPINGS) {
      if (m.exact || !m.from.includes(mark)) continue;
      const base = [...on].find((sym) => m.from.includes(sym + mark));
      expect(base, `${m.from}: the engine reads ${mark} on ${base} now: delete the entry`).toBeUndefined();
    }
  }
});

// ------------------------------------------------------------------ the tool
test("--ipa cleans IPA from an argument, prints the lyric line, and reports what it changed on stderr", () => {
  const r = run(["--ipa", "ˈʃøːn", "fˈʏʁ", "bɔ̃ʒˈuʁ", "n̥a"]);
  expect(r.code).toBe(0);
  expect(r.out.trim()).toBe("/ˈʃøːn/ /fˈyʁ/ /bɔ̃ʒˈuʁ/ /na/");
  expect(r.err).toContain("ʏ → y");
  expect(r.err).toContain("n̥ → n");
  // the nasal vowel is sung, so it is not reported
  expect(r.err).not.toContain("ɔ̃");
});

test("--ipa reads stdin and --json prints the syllables array a melody file takes", () => {
  const r = run(["--ipa", "--json"], "ˈʃtʁaːsə\n");
  expect(r.code).toBe(0);
  const s = JSON.parse(r.out);
  expect(s.map((x: { text: string }) => x.text)).toEqual(["ˈʃtʁaː", "sə"]);
  expect(s[1].wordEnd).toBe(true);
});

test("a word with no vowel is said to be unsung", () => {
  const r = run(["--ipa", "ʃt ma"]);
  expect(r.out.trim()).toBe("/ʃt/ /ma/");
  expect(r.err).toContain("no vowel");
  expect(r.err).toContain("ʃt");
});

test("without eSpeak NG the tool exits non-zero with a one-line install hint that names --ipa", () => {
  const r = run(["--espeak", "no-such-espeak-binary", "--voice", "de", "Guten Morgen"]);
  expect(r.code).not.toBe(0);
  const lines = r.err.trim().split("\n");
  expect(lines.length).toBe(1);
  expect(lines[0]).toContain("Install eSpeak NG");
  expect(lines[0]).toContain("--ipa");
  expect(r.out).toBe("");
});

test("no input is a usage error", () => {
  expect(run([]).code).toBe(2);
  expect(run(["--bogus"]).code).toBe(2);
});
