/**
 * tools/text-to-ipa.ts: the cleaner (tools/lib/text-to-ipa.ts) on hand-written examples of eSpeak
 * NG's documented `--ipa` output format, and the tool itself run as a user runs it, without eSpeak
 * (`--ipa` input, and the missing-eSpeak message). The examples are NOT captured from a real
 * eSpeak NG: nothing here shows what a real one prints for these words.
 */
import { expect, test } from "bun:test";
import { fileURLToPath } from "node:url";
import { initEngine } from "./harness.ts";
import { MAPPINGS, MODIFIERS, bareWord, cleanWord, cleanWords, splitEspeak, toLyric } from "../tools/lib/text-to-ipa.ts";

await initEngine();
const { ipaSymbols, lyricsToSyllables } = await import("../src/wasm/engine.ts");
const engine = new Set(ipaSymbols());
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
  for (const s of engine) if (s.normalize("NFD") !== s) expect(clean(s).ipa).toBe(s);
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

test("a mark the engine does not model is dropped from its letter and reported; the letter stays", () => {
  const r = clean("bɔ̃ʒˈuʁ");
  expect(r.ipa).toBe("bɔʒˈuʁ");
  expect(r.changes.map((c) => [c.from, c.to])).toEqual([["ɔ̃", "ɔ"]]);
  // a precomposed nasal vowel is read as its letter and the mark
  expect(clean("lã").ipa).toBe("la");
  expect(clean("lã").changes.map((c) => [c.from, c.to])).toEqual([["ã", "a"]]);
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
      const n = [2, 1].find((k) => engine.has(s.slice(i, i + k)) || (k === 1 && MODIFIERS.has(s[i])));
      if (!n) return false;
      i += n;
    }
    return true;
  };
  for (const m of MAPPINGS) {
    expect(sung(m.to), `${m.from} → ${m.to}: the target is not sung`).toBe(true);
    if (!m.exact) expect(engine.has(m.from), `${m.from} is sung now: delete its entry`).toBe(false);
  }
});

// ------------------------------------------------------------------ the tool
test("--ipa cleans IPA from an argument, prints the lyric line, and reports what it changed on stderr", () => {
  const r = run(["--ipa", "ˈʃøːn", "fˈʏʁ", "bɔ̃ʒˈuʁ"]);
  expect(r.code).toBe(0);
  expect(r.out.trim()).toBe("/ˈʃøːn/ /fˈyʁ/ /bɔʒˈuʁ/");
  expect(r.err).toContain("ʏ → y");
  expect(r.err).toContain("ɔ̃ → ɔ");
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
