/**
 * IPA lyrics through the wasm facade: words between slashes typed in Sing mode reach the engine's
 * IPA front end, in one line with other scripts (the splitting rule itself is tested in
 * crates/chimera/tests/ipa_lyrics.rs).
 */
import { expect, test } from "bun:test";
import { initEngine } from "./harness.ts";

await initEngine();
const { lyricsToSyllables, ipaSymbols } = await import("../src/wasm/engine.ts");

test("an IPA word is split into syllables without the English onset table, beside other scripts", () => {
  const s = lyricsToSyllables("さくら /ˈʃtʁaːsə/ hello 빵야");
  expect(s.map((x) => x.text)).toEqual(["さ", "く", "ら", "ˈʃtʁaː", "sə", "hel", "lo", "빵", "야"]);
  const strasse = s.slice(3, 5);
  expect(strasse[0]!.onset.map((x) => x.ipa)).toEqual(["ʃ", "t", "ʁ"]);
  expect(strasse[1]!.onset.map((x) => x.ipa)).toEqual(["s"]);
  expect(strasse.map((x) => x.wordEnd)).toEqual([false, true]);
});

test("aspiration is written: unmarked voiceless stops are unaspirated, ʰ aspirates", () => {
  const [pa, pha] = lyricsToSyllables("/pa/ /pʰa/");
  expect(pa!.onset[0]!.aspirated).toBe(false);
  expect(pha!.onset[0]!.aspirated).toBe(true);
});

test("a break mark and phrase punctuation work around IPA words", () => {
  const s = lyricsToSyllables("/ni.xaʊ/ - /ma/,");
  expect(s.map((x) => x.text)).toEqual(["ni", "xaʊ", "~", "ma"]);
  expect(s[2]!.melisma).toBe(true);
  expect(s[3]!.phraseEnd).toBe(true);
});

test("the symbols the engine can sing are listed: ʃ ø ɔɪ yes, ʏ no", () => {
  const sym = new Set(ipaSymbols());
  for (const s of ["ʃ", "ø", "ɔɪ", "ç", "y"]) expect(sym.has(s)).toBe(true);
  expect(sym.has("ʏ")).toBe(false);
});
